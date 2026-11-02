use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Settings {
    pub thumbnail_width: f32,
    pub loupe_levels: Vec<u32>,
    pub auto_advance: bool,
    pub show_preview: bool,
    pub show_thumbnails: bool,
    pub show_review: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            thumbnail_width: 144.0,
            loupe_levels: vec![100, 200, 500],
            auto_advance: true,
            show_preview: true,
            show_thumbnails: true,
            show_review: true,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        if !self.thumbnail_width.is_finite() {
            self.thumbnail_width = 144.0;
        }
        self.thumbnail_width = self.thumbnail_width.clamp(90.0, 280.0);
        self.loupe_levels
            .retain(|level| (25..=2000).contains(level));
        self.loupe_levels.truncate(12);
        if self.loupe_levels.is_empty() {
            self.loupe_levels = vec![100, 200, 500];
        }
    }
}

pub fn settings_path(session_root: &Path) -> PathBuf {
    session_root.join("preferences.json")
}

pub fn load(root: &Path) -> io::Result<Settings> {
    let path = settings_path(root);
    let mut settings = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
        Err(error) => return Err(error),
    };
    Settings::normalize(&mut settings);
    Ok(settings)
}

pub fn save(root: &Path, settings: &Settings) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let mut temporary = tempfile::NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(&mut temporary, settings).map_err(io::Error::other)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(settings_path(root))
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_and_invalid_values_are_bounded() {
        let root = tempfile::tempdir().unwrap();
        let mut settings = Settings {
            thumbnail_width: 900.0,
            loupe_levels: vec![0, 100, 500, 9999],
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(settings.thumbnail_width, 280.0);
        assert_eq!(settings.loupe_levels, [100, 500]);
        save(root.path(), &settings).unwrap();
        assert_eq!(load(root.path()).unwrap(), settings);
    }
}
