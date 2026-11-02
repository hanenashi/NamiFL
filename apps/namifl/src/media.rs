//! One decoder worker, bounded request/result queues, and a bounded UI texture cache.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, mpsc};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::ImageDecoder;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Kind {
    Thumbnail,
    Preview,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub generation: u64,
    pub path: PathBuf,
    pub kind: Kind,
}

pub struct Decoded {
    pub key: Key,
    pub image: Result<(ColorImage, [u32; 2]), String>,
}

#[derive(Default)]
struct Queue {
    requests: VecDeque<Key>,
    stopped: bool,
}

pub struct MediaWorker {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    pub rx: mpsc::Receiver<Decoded>,
}

impl MediaWorker {
    pub fn start(ctx: egui::Context) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let worker_queue = queue.clone();
        let (results, rx) = mpsc::sync_channel(2);
        std::thread::spawn(move || {
            loop {
                let key = {
                    let (lock, ready) = &*worker_queue;
                    let mut queue = lock.lock().expect("media queue poisoned");
                    while queue.requests.is_empty() && !queue.stopped {
                        queue = ready.wait(queue).expect("media queue poisoned");
                    }
                    if queue.stopped {
                        break;
                    }
                    queue.requests.pop_front().expect("nonempty media queue")
                };
                let image = decode(&key).map_err(|e| e.to_string());
                if results.send(Decoded { key, image }).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { queue, rx }
    }

    pub fn request(&self, key: Key) -> bool {
        let (lock, ready) = &*self.queue;
        let mut queue = lock.lock().expect("media queue poisoned");
        if queue.requests.len() >= 24 {
            return false;
        }
        if key.kind == Kind::Preview {
            queue.requests.push_front(key);
        } else {
            queue.requests.push_back(key);
        }
        ready.notify_one();
        true
    }

    pub fn clear(&self) {
        self.queue
            .0
            .lock()
            .expect("media queue poisoned")
            .requests
            .clear();
    }
}

impl Drop for MediaWorker {
    fn drop(&mut self) {
        let mut queue = self.queue.0.lock().expect("media queue poisoned");
        queue.stopped = true;
        self.queue.1.notify_one();
    }
}

fn decode(key: &Key) -> image::ImageResult<(ColorImage, [u32; 2])> {
    let mut reader = image::ImageReader::open(&key.path)?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let dimensions = [image.width(), image.height()];
    let image = match key.kind {
        Kind::Thumbnail => image.thumbnail(240, 180),
        Kind::Preview if image.width().max(image.height()) > 8192 => image.thumbnail(8192, 8192),
        Kind::Preview => image,
    }
    .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Ok((
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        dimensions,
    ))
}

pub struct CachedImage {
    pub texture: TextureHandle,
    pub original_size: [u32; 2],
    bytes: usize,
    used: u64,
}

#[derive(Default)]
pub struct ImageCache {
    images: HashMap<Key, CachedImage>,
    pub errors: HashMap<Key, String>,
    pending: HashSet<Key>,
    declined_prefetch: HashSet<Key>,
    protected: Option<Key>,
    clock: u64,
}

impl ImageCache {
    pub fn protect(&mut self, key: Option<Key>) {
        if let Some(key) = &key {
            self.declined_prefetch.remove(key);
        }
        self.protected = key;
    }

    pub fn request(&mut self, worker: &MediaWorker, key: Key) {
        if !self.images.contains_key(&key)
            && !self.errors.contains_key(&key)
            && !self.pending.contains(&key)
            && !self.declined_prefetch.contains(&key)
            && worker.request(key.clone())
        {
            self.pending.insert(key);
        }
    }

    pub fn get(&mut self, key: &Key) -> Option<&CachedImage> {
        self.clock += 1;
        let image = self.images.get_mut(key)?;
        image.used = self.clock;
        Some(image)
    }

    pub fn accept(&mut self, ctx: &egui::Context, decoded: Decoded) {
        self.pending.remove(&decoded.key);
        match decoded.image {
            Ok((image, original_size)) => {
                let bytes = image.pixels.len() * 4;
                // A single active image may exceed the budget, but no other full
                // preview is retained alongside it. Tiny thumbnails remain cheap.
                let budget = 160 * 1024 * 1024;
                let pinned_bytes = self
                    .protected
                    .as_ref()
                    .and_then(|key| self.images.get(key))
                    .map_or(0, |image| image.bytes);
                if decoded.key.kind == Kind::Preview
                    && Some(&decoded.key) != self.protected.as_ref()
                    && pinned_bytes + bytes > budget
                {
                    self.declined_prefetch.insert(decoded.key);
                    return;
                }
                while self.images.len() >= 64
                    || self.images.values().map(|v| v.bytes).sum::<usize>() + bytes > budget
                {
                    let Some(oldest) = self
                        .images
                        .iter()
                        .filter(|(key, _)| Some(*key) != self.protected.as_ref())
                        .min_by_key(|(_, value)| value.used)
                        .map(|(key, _)| key.clone())
                    else {
                        break;
                    };
                    self.images.remove(&oldest);
                }
                self.clock += 1;
                let texture =
                    ctx.load_texture(format!("{:?}", decoded.key), image, TextureOptions::LINEAR);
                self.images.insert(
                    decoded.key,
                    CachedImage {
                        texture,
                        original_size,
                        bytes,
                        used: self.clock,
                    },
                );
            }
            Err(error) => {
                self.errors.insert(decoded.key, error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_exif_orientation_and_bounds_thumbnail() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rotated.jpg");
        let pixels = image::RgbImage::from_pixel(800, 400, image::Rgb([200, 80, 40]));
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut encoded)
            .encode_image(&pixels)
            .unwrap();
        // EXIF orientation 6 (90 degrees clockwise), little-endian TIFF.
        let exif: &[u8] =
            b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut rotated = encoded[..2].to_vec();
        rotated.extend_from_slice(&[0xff, 0xe1]);
        rotated.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        rotated.extend_from_slice(exif);
        rotated.extend_from_slice(&encoded[2..]);
        std::fs::write(&path, rotated).unwrap();
        let (image, original) = decode(&Key {
            generation: 1,
            path,
            kind: Kind::Thumbnail,
        })
        .unwrap();
        assert_eq!(original, [400, 800]);
        assert_eq!(image.size, [90, 180]);
    }
}
