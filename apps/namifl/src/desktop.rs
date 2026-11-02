use std::collections::BTreeSet;
use std::path::PathBuf;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use namifl_core::domain::{Decision, PhotoGroup};
use namifl_core::review::ReviewRecord;

use crate::media::{ImageCache, Key as ImageKey, Kind, MediaWorker};
use crate::session::{IoRequest, IoResult, LoadedFolder, SessionWorker};

const BACKGROUND: Color32 = Color32::from_rgb(17, 20, 25);
const PANEL: Color32 = Color32::from_rgb(25, 29, 35);
const MUTED: Color32 = Color32::from_rgb(147, 158, 174);
const ACCENT: Color32 = Color32::from_rgb(113, 208, 219);
const KEEP: Color32 = Color32::from_rgb(124, 212, 160);
const REJECT: Color32 = Color32::from_rgb(239, 143, 139);

#[derive(Clone, Copy, Default, PartialEq)]
enum Filter {
    #[default]
    All,
    Unreviewed,
    Keep,
    Reject,
}

impl Filter {
    fn accepts(self, decision: Decision) -> bool {
        match self {
            Self::All => true,
            Self::Unreviewed => decision == Decision::Unreviewed,
            Self::Keep => decision == Decision::Keep,
            Self::Reject => decision == Decision::Reject,
        }
    }
}

pub struct CullApp {
    io: SessionWorker,
    media: MediaWorker,
    cache: ImageCache,
    loaded: Option<LoadedFolder>,
    generation: u64,
    revision: u64,
    saved_revision: u64,
    opening: bool,
    save_error: Option<String>,
    message: Option<String>,
    current: usize,
    selected: BTreeSet<usize>,
    anchor: usize,
    jpeg_variant: usize,
    filter: Filter,
    zoom: f32,
    pan: Vec2,
    auto_advance: bool,
    scroll_to_current: bool,
    close_after_save: bool,
    state_root: PathBuf,
}

impl CullApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        folder: Option<PathBuf>,
        state_root: PathBuf,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.global_style_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10.0, 9.0);
            style.spacing.button_padding = Vec2::new(13.0, 8.0);
            style.visuals.panel_fill = PANEL;
            style.visuals.window_fill = PANEL;
            style.visuals.selection.bg_fill = Color32::from_rgb(38, 75, 85);
            style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        });
        let mut app = Self {
            io: SessionWorker::start(cc.egui_ctx.clone(), state_root.clone()),
            media: MediaWorker::start(cc.egui_ctx.clone()),
            cache: ImageCache::default(),
            loaded: None,
            generation: 0,
            revision: 0,
            saved_revision: 0,
            opening: false,
            save_error: None,
            message: None,
            current: 0,
            selected: BTreeSet::new(),
            anchor: 0,
            jpeg_variant: 0,
            filter: Filter::All,
            zoom: 1.0,
            pan: Vec2::ZERO,
            auto_advance: true,
            scroll_to_current: false,
            close_after_save: false,
            state_root,
        };
        if let Some(folder) = folder {
            app.open(Some(folder));
        }
        app
    }

    fn open(&mut self, folder: Option<PathBuf>) {
        if self.opening || self.revision != self.saved_revision {
            return;
        }
        self.generation += 1;
        self.opening = true;
        self.message = None;
        if self
            .io
            .tx
            .send(IoRequest::Open {
                folder,
                generation: self.generation,
            })
            .is_err()
        {
            self.message = Some("The folder worker stopped. Please restart NamiFL.".into());
            self.opening = false;
        }
    }

    fn collect_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.io.rx.try_recv() {
            match result {
                IoResult::Opened { result, generation } if generation == self.generation => {
                    self.opening = false;
                    match result {
                        Ok(Some(loaded)) => {
                            self.loaded = Some(loaded);
                            self.current = 0;
                            self.selected.clear();
                            if self.loaded.as_ref().is_some_and(|l| !l.groups.is_empty()) {
                                self.selected.insert(0);
                            }
                            self.anchor = 0;
                            self.jpeg_variant = 0;
                            self.filter = Filter::All;
                            self.zoom = 1.0;
                            self.pan = Vec2::ZERO;
                            self.cache = ImageCache::default();
                            self.media.clear();
                            self.revision = 0;
                            self.saved_revision = 0;
                            self.save_error = None;
                            self.scroll_to_current = true;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            self.message = Some(error);
                        }
                    }
                }
                IoResult::Saved {
                    result,
                    revision,
                    generation,
                } if generation == self.generation => match result {
                    Ok(()) => {
                        self.saved_revision = revision;
                        self.save_error = None;
                    }
                    Err(error) => {
                        self.save_error = Some(error);
                        self.close_after_save = false;
                    }
                },
                _ => {}
            }
        }
        // Bound uploads per frame as well as the worker result queue.
        for _ in 0..2 {
            let Ok(decoded) = self.media.rx.try_recv() else {
                break;
            };
            if decoded.key.generation == self.generation {
                self.cache.accept(ctx, decoded);
            }
        }
    }

    fn visible(&self) -> Vec<usize> {
        self.loaded.as_ref().map_or_else(Vec::new, |loaded| {
            loaded
                .groups
                .iter()
                .enumerate()
                .filter(|(_, group)| self.filter.accepts(group.decision))
                .map(|(index, _)| index)
                .collect()
        })
    }

    fn select(&mut self, index: usize, ctrl: bool, shift: bool) {
        if shift {
            let visible = self.visible();
            let start = visible.iter().position(|&i| i == self.anchor);
            let end = visible.iter().position(|&i| i == index);
            if let (Some(start), Some(end)) = (start, end) {
                if !ctrl {
                    self.selected.clear();
                }
                self.selected
                    .extend(visible[start.min(end)..=start.max(end)].iter().copied());
            } else {
                self.selected.insert(index);
            }
        } else if ctrl {
            if !self.selected.remove(&index) {
                self.selected.insert(index);
            }
            self.anchor = index;
        } else {
            self.selected.clear();
            self.selected.insert(index);
            self.anchor = index;
        }
        if self.current != index {
            self.zoom = 1.0;
            self.pan = Vec2::ZERO;
            self.jpeg_variant = 0;
        }
        self.current = index;
        self.scroll_to_current = true;
    }

    fn navigate(&mut self, delta: isize, shift: bool) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let position = visible.iter().position(|&i| i == self.current).unwrap_or(0);
        let next = position.saturating_add_signed(delta).min(visible.len() - 1);
        self.select(visible[next], false, shift);
    }

    fn save(&mut self) {
        if let Some(loaded) = &self.loaded
            && self
                .io
                .tx
                .send(IoRequest::Save {
                    session: loaded.session.clone(),
                    revision: self.revision,
                    generation: self.generation,
                })
                .is_err()
        {
            self.save_error = Some(
                "The session worker stopped. Your latest decisions have not been saved.".into(),
            );
        }
    }

    fn mark(&mut self, decision: Decision) {
        if self.opening || self.selected.is_empty() {
            return;
        }
        let visible = self.visible();
        let next = visible
            .iter()
            .position(|&i| i == self.current)
            .and_then(|position| visible.get(position + 1))
            .copied();
        let Some(loaded) = &mut self.loaded else {
            return;
        };
        let mut changed = false;
        for &index in &self.selected {
            let group = &mut loaded.groups[index];
            if group.decision == decision {
                continue;
            }
            group.decision = decision;
            loaded.session.records.insert(
                group.stem.clone(),
                ReviewRecord {
                    files: loaded.stamps[&group.stem].clone(),
                    decision,
                },
            );
            changed = true;
        }
        if changed {
            self.revision += 1;
            self.save();
        }
        if self.auto_advance
            && self.selected.len() == 1
            && let Some(next) = next
        {
            self.select(next, false, false);
        }
        let visible = self.visible();
        self.selected.retain(|index| visible.contains(index));
        if !visible.contains(&self.current)
            && let Some(&next) = visible
                .iter()
                .find(|&&index| index >= self.current)
                .or_else(|| visible.last())
        {
            self.select(next, false, false);
        }
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused()
            || self.opening
            || self.message.is_some()
            || self.save_error.is_some()
        {
            return;
        }
        ctx.input_mut(|input| {
            // Modifiers belong to each key event: a quick chord can already be
            // released by the time this frame's aggregate modifier state is read.
            for (key, modifiers) in review_key_events(&input.events) {
                let handled = if modifiers.command {
                    match key {
                        Key::O => {
                            self.open(None);
                            true
                        }
                        Key::S => {
                            self.save();
                            true
                        }
                        Key::A => {
                            self.selected = self.visible().into_iter().collect();
                            true
                        }
                        _ => false,
                    }
                } else {
                    match key {
                        Key::ArrowRight => {
                            self.navigate(1, modifiers.shift);
                            true
                        }
                        Key::ArrowLeft => {
                            self.navigate(-1, modifiers.shift);
                            true
                        }
                        Key::K => {
                            self.mark(Decision::Keep);
                            true
                        }
                        Key::D => {
                            self.mark(Decision::Reject);
                            true
                        }
                        Key::U => {
                            self.mark(Decision::Unreviewed);
                            true
                        }
                        Key::Space => {
                            let keep = self
                                .current_group()
                                .is_some_and(|g| g.decision == Decision::Keep);
                            self.mark(if keep {
                                Decision::Unreviewed
                            } else {
                                Decision::Keep
                            });
                            true
                        }
                        Key::F => {
                            self.zoom = 1.0;
                            self.pan = Vec2::ZERO;
                            true
                        }
                        _ => false,
                    }
                };
                if handled {
                    input.consume_key(modifiers, key);
                }
            }
        });
    }

    fn current_group(&self) -> Option<&PhotoGroup> {
        self.loaded
            .as_ref()?
            .groups
            .get(self.current)
            .filter(|group| self.filter.accepts(group.decision))
    }

    fn image_key(&self, kind: Kind) -> Option<ImageKey> {
        let path = self.current_group()?.jpegs.get(self.jpeg_variant)?.clone();
        Some(ImageKey {
            generation: self.generation,
            path,
            kind,
        })
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.label(RichText::new("NamiFL").size(25.0).strong().color(ACCENT));
            ui.add_space(28.0);
            ui.label(RichText::new("CULL").strong());
            ui.label(RichText::new("/  DxO  /  Transport").color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ready = !self.opening && self.revision == self.saved_revision;
                if ui
                    .add_enabled(ready, egui::Button::new("Open folder   Ctrl+O"))
                    .clicked()
                {
                    self.open(None);
                }
            });
        });
    }

    fn folder_bar(&mut self, ui: &mut egui::Ui) {
        let folder = self.loaded.as_ref().map(|l| l.session.folder.clone());
        ui.horizontal(|ui| {
            if let Some(folder) = folder {
                let name = folder
                    .file_name()
                    .unwrap_or(folder.as_os_str())
                    .to_string_lossy();
                ui.label(RichText::new(name).size(17.0).strong())
                    .on_hover_text(folder.display().to_string());
            } else {
                ui.label(RichText::new("Your next keepers").size(17.0).strong());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for (filter, text) in [
                    (Filter::Reject, "Rejected"),
                    (Filter::Keep, "Keepers"),
                    (Filter::Unreviewed, "Unreviewed"),
                    (Filter::All, "All photos"),
                ] {
                    if ui
                        .selectable_value(&mut self.filter, filter, text)
                        .changed()
                    {
                        let visible = self.visible();
                        if let Some(&first) = visible.first() {
                            self.select(first, false, false);
                        } else {
                            self.selected.clear();
                        }
                    }
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(RichText::new("THE REVIEW").color(MUTED).size(11.0));
        if let Some(group) = self.current_group().cloned() {
            ui.add_space(8.0);
            ui.label(RichText::new(&group.stem).size(23.0).strong());
            ui.label(
                RichText::new(format!(
                    "{} JPEG  /  {} RAW",
                    group.jpegs.len(),
                    group.raws.len()
                ))
                .color(MUTED),
            );
            ui.add_space(14.0);
            ui.colored_label(
                decision_color(group.decision),
                decision_text(group.decision),
            );
            ui.label(format!("{} selected", self.selected.len()));
            ui.add_space(12.0);
            let enabled = !self.selected.is_empty() && !self.opening;
            for (decision, text) in [
                (Decision::Keep, "Keep                         K"),
                (Decision::Reject, "Reject                       D"),
                (Decision::Unreviewed, "Unreviewed                 U"),
            ] {
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(RichText::new(text).color(decision_color(decision)))
                            .min_size(Vec2::new(ui.available_width(), 38.0)),
                    )
                    .clicked()
                {
                    self.mark(decision);
                }
            }
            ui.add_space(8.0);
            ui.checkbox(&mut self.auto_advance, "Advance after tagging");
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new("SOURCE FILES").color(MUTED).size(11.0));
            if group.is_ambiguous() {
                ui.colored_label(
                    Color32::from_rgb(236, 194, 123),
                    "Multiple variants — review the files.",
                );
            }
            for (index, path) in group.jpegs.iter().enumerate() {
                let name = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy();
                if ui
                    .selectable_value(&mut self.jpeg_variant, index, name)
                    .changed()
                {
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                }
            }
            for path in &group.raws {
                ui.label(
                    RichText::new(
                        path.file_name()
                            .unwrap_or(path.as_os_str())
                            .to_string_lossy(),
                    )
                    .color(MUTED),
                );
            }
            if group.jpegs.is_empty() {
                ui.label("RAW-only group. No JPEG preview.");
            }
        } else {
            ui.add_space(14.0);
            ui.label("A quieter way to choose.");
            ui.label(
                RichText::new("Open a shoot, review the JPEGs, and keep your decisions here.")
                    .color(MUTED),
            );
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(12.0);
        ui.label(
            RichText::new("NEXT: DEVELOP IN DxO")
                .size(11.0)
                .color(ACCENT),
        );
        ui.label(
            RichText::new(
                "Develop your keepers externally. Export finished JPEGs to a separate folder.",
            )
            .color(MUTED),
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new("Transport is coming next. RAW deletion stays manual.")
                .small()
                .color(MUTED),
        );
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let visible = self.visible();
        if self.loaded.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Find the frames worth keeping.");
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Camera JPEGs for review. RAWs stay with you.").color(MUTED),
                    );
                    ui.add_space(22.0);
                    if ui
                        .add_enabled(
                            !self.opening,
                            egui::Button::new("Open a photo folder")
                                .min_size(Vec2::new(200.0, 44.0)),
                        )
                        .clicked()
                    {
                        self.open(None);
                    }
                    ui.add_space(12.0);
                    ui.label(RichText::new("or drop a folder here").small().color(MUTED));
                });
            });
            return;
        }
        if visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No photos in this view. Choose another filter or folder.");
            });
            return;
        }
        ui.horizontal(|ui| {
            let position = visible.iter().position(|&i| i == self.current).unwrap_or(0) + 1;
            ui.label(
                RichText::new(format!("{position:03} / {:03}", visible.len()))
                    .monospace()
                    .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Fit   F").clicked() {
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                }
                if ui.button("100%").clicked() {
                    self.zoom = -1.0;
                    self.pan = Vec2::ZERO;
                }
                ui.label(
                    RichText::new("Wheel to zoom · drag to pan · hold right-click for loupe")
                        .small()
                        .color(MUTED),
                );
            });
        });
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, BACKGROUND);
        let Some(key) = self.image_key(Kind::Preview) else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "RAW companion only — no JPEG to preview",
                FontId::proportional(17.0),
                MUTED,
            );
            return;
        };
        self.cache.request(&self.media, key.clone());
        if let Some(error) = self.cache.errors.get(&key) {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Cannot preview this JPEG\nThe file may be damaged, unsupported, or too large.",
                FontId::proportional(14.0),
                REJECT,
            );
            response.on_hover_text(error);
            return;
        }
        let Some(image) = self.cache.get(&key) else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Loading preview…",
                FontId::proportional(16.0),
                MUTED,
            );
            return;
        };
        let texture = image.texture.clone();
        let original_size = image.original_size;
        let image_size = texture.size_vec2();
        let available = (rect.size() - Vec2::splat(32.0)).max(Vec2::splat(1.0));
        let fit = (available.x / image_size.x)
            .min(available.y / image_size.y)
            .min(1.0);
        if self.zoom < 0.0 {
            self.zoom = 1.0 / fit;
        }
        if response.double_clicked() || response.clicked_by(egui::PointerButton::Middle) {
            self.zoom = 1.0;
            self.pan = Vec2::ZERO;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.004).exp()).clamp(1.0, 30.0);
                if let Some(pointer) = response.hover_pos() {
                    let relative = pointer - rect.center();
                    self.pan = relative - (relative - self.pan) * (self.zoom / old);
                }
                if self.zoom == 1.0 {
                    self.pan = Vec2::ZERO;
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        let drawn = Rect::from_center_size(rect.center() + self.pan, image_size * fit * self.zoom);
        painter.image(
            texture.id(),
            drawn,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if response.hovered()
            && ui.input(|i| i.pointer.secondary_down())
            && let Some(pointer) = response.hover_pos()
        {
            let uv = (pointer - drawn.min) / drawn.size();
            let half = Vec2::splat(100.0) / image_size;
            let uv_rect = Rect::from_min_max(
                Pos2::new(uv.x - half.x, uv.y - half.y),
                Pos2::new(uv.x + half.x, uv.y + half.y),
            );
            let lens = Rect::from_center_size(pointer, Vec2::splat(200.0))
                .translate(Vec2::new(0.0, -12.0));
            painter.rect_filled(lens.expand(2.0), 2.0, ACCENT);
            painter.image(texture.id(), lens, uv_rect, Color32::WHITE);
        }
        let capped = if original_size[0].max(original_size[1]) > 8192 {
            " · preview capped at 8192 px"
        } else {
            ""
        };
        painter.text(
            rect.left_bottom() + Vec2::new(14.0, -12.0),
            Align2::LEFT_BOTTOM,
            format!(
                "{} × {}  ·  {:.0}%{capped}",
                original_size[0],
                original_size[1],
                fit * self.zoom * 100.0
            ),
            FontId::monospace(11.0),
            MUTED,
        );
        // Prefetch one adjacent full image only when two similarly sized previews
        // fit comfortably in the cache. Thumbnail decoding remains independent.
        if image_size.x * image_size.y * 8.0 < 128.0 * 1024.0 * 1024.0
            && let Some(position) = visible.iter().position(|&i| i == self.current)
            && let Some(&next) = visible.get(position + 1)
            && let Some(path) = self
                .loaded
                .as_ref()
                .and_then(|l| l.groups[next].jpegs.first())
        {
            self.cache.request(
                &self.media,
                ImageKey {
                    generation: self.generation,
                    path: path.clone(),
                    kind: Kind::Preview,
                },
            );
        }
    }

    fn filmstrip(&mut self, ui: &mut egui::Ui) {
        let visible = self.visible();
        let mut scroll = egui::ScrollArea::horizontal().id_salt("filmstrip");
        if self.scroll_to_current {
            if let Some(position) = visible.iter().position(|&i| i == self.current) {
                scroll = scroll.horizontal_scroll_offset(
                    (position as f32 * 154.0 - ui.available_width() / 2.0 + 77.0).max(0.0),
                );
            }
            self.scroll_to_current = false;
        }
        scroll.show(ui, |ui| {
            ui.horizontal(|ui| {
                for index in visible {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(144.0, 126.0), Sense::click());
                    if !ui.is_rect_visible(rect) {
                        continue;
                    }
                    let group = &self
                        .loaded
                        .as_ref()
                        .expect("visible group has folder")
                        .groups[index];
                    let painter = ui.painter();
                    let selected = self.selected.contains(&index);
                    painter.rect_filled(
                        rect,
                        7.0,
                        if selected {
                            Color32::from_rgb(36, 50, 59)
                        } else {
                            BACKGROUND
                        },
                    );
                    if selected {
                        painter.rect_stroke(
                            rect,
                            7.0,
                            Stroke::new(1.5, ACCENT),
                            StrokeKind::Inside,
                        );
                    }
                    let image_rect = Rect::from_min_max(
                        rect.min + Vec2::new(6.0, 6.0),
                        rect.max - Vec2::new(6.0, 33.0),
                    );
                    if let Some(path) = group.jpegs.first() {
                        let key = ImageKey {
                            generation: self.generation,
                            path: path.clone(),
                            kind: Kind::Thumbnail,
                        };
                        self.cache.request(&self.media, key.clone());
                        if let Some(image) = self.cache.get(&key) {
                            let size = image.texture.size_vec2();
                            let scale =
                                (image_rect.width() / size.x).min(image_rect.height() / size.y);
                            painter.image(
                                image.texture.id(),
                                Rect::from_center_size(image_rect.center(), size * scale),
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            let text = if self.cache.errors.contains_key(&key) {
                                "Unreadable JPEG"
                            } else {
                                "…"
                            };
                            painter.text(
                                image_rect.center(),
                                Align2::CENTER_CENTER,
                                text,
                                FontId::proportional(12.0),
                                MUTED,
                            );
                        }
                    } else {
                        painter.text(
                            image_rect.center(),
                            Align2::CENTER_CENTER,
                            "RAW only",
                            FontId::proportional(12.0),
                            MUTED,
                        );
                    }
                    let name: String = group.stem.chars().take(17).collect();
                    painter.text(
                        rect.left_bottom() + Vec2::new(9.0, -17.0),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    painter.circle_filled(
                        rect.right_top() + Vec2::new(-12.0, 12.0),
                        4.0,
                        decision_color(group.decision),
                    );
                    if group.is_ambiguous() {
                        painter.text(
                            rect.left_top() + Vec2::new(9.0, 10.0),
                            Align2::LEFT_TOP,
                            "MULTI",
                            FontId::monospace(9.0),
                            ACCENT,
                        );
                    }
                    response.clone().on_hover_text(format!(
                        "{}\n{}\nCtrl-click to add · Shift-click for range",
                        group.stem,
                        decision_text(group.decision)
                    ));
                    if response.clicked() {
                        let modifiers = ui.input(|i| {
                            i.events
                                .iter()
                                .rev()
                                .find_map(|event| {
                                    if let egui::Event::PointerButton { modifiers, .. } = event {
                                        Some(*modifiers)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(i.modifiers)
                        });
                        self.select(index, modifiers.command, modifiers.shift);
                    }
                }
            });
        });
    }

    fn status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.opening {
                ui.spinner();
                ui.label("Opening folder and restoring review…");
            } else if let Some(loaded) = &self.loaded {
                let keep = loaded
                    .groups
                    .iter()
                    .filter(|g| g.decision == Decision::Keep)
                    .count();
                let reject = loaded
                    .groups
                    .iter()
                    .filter(|g| g.decision == Decision::Reject)
                    .count();
                ui.label(
                    RichText::new(format!(
                        "{keep} keepers  ·  {reject} rejected  ·  {} unreviewed",
                        loaded.groups.len() - keep - reject
                    ))
                    .small()
                    .color(MUTED),
                );
                if loaded.unsupported_names > 0 {
                    ui.label(format!("{} unsupported names", loaded.unsupported_names));
                }
            } else {
                ui.label(RichText::new("Cull → DxO → Transport").small().color(MUTED));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = if self.save_error.is_some() {
                    "Save failed"
                } else if self.revision != self.saved_revision {
                    "Saving review…"
                } else if self.loaded.is_some() {
                    "Review saved locally"
                } else {
                    "Source photos stay untouched"
                };
                ui.label(
                    RichText::new(text)
                        .small()
                        .color(if self.save_error.is_some() {
                            REJECT
                        } else {
                            MUTED
                        }),
                )
                .on_hover_text(self.state_root.display().to_string());
            });
        });
    }
}

impl eframe::App for CullApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.cache.protect(self.image_key(Kind::Preview));
        self.collect_results(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && self.revision != self.saved_revision {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_after_save = self.save_error.is_none();
        }
        if self.close_after_save && self.revision == self.saved_revision {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            // Folder validation and canonicalization happen on the I/O worker.
            self.open(Some(path));
        }
        self.keyboard(&ctx);
        egui::Panel::top("header")
            .exact_size(70.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(18))
            .show(ui, |ui| self.header(ui));
        egui::Panel::top("folder")
            .exact_size(58.0)
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(14))
            .show(ui, |ui| self.folder_bar(ui));
        egui::Panel::bottom("status")
            .exact_size(32.0)
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(8))
            .show(ui, |ui| self.status(ui));
        if self.loaded.is_some() {
            egui::Panel::bottom("strip")
                .exact_size(154.0)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(12))
                .show(ui, |ui| self.filmstrip(ui));
        }
        egui::Panel::right("review")
            .exact_size(270.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(20))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.sidebar(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(18))
            .show(ui, |ui| self.preview(ui));
        if let Some(message) = self.message.clone() {
            egui::Window::new("Could not open folder")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.set_max_width(460.0);
                    ui.label(message);
                    if ui.button("Close").clicked() {
                        self.message = None;
                    }
                });
        }
        if let Some(error) = self.save_error.clone() {
            egui::Window::new("Review has not been saved")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.set_max_width(460.0);
                    ui.label(error);
                    ui.label("Keep this window open to retain your latest decisions.");
                    if ui.button("Retry save").clicked() {
                        self.save_error = None;
                        self.save();
                    }
                });
        }
    }
}

fn decision_text(decision: Decision) -> &'static str {
    match decision {
        Decision::Unreviewed => "Unreviewed",
        Decision::Keep => "Keeper",
        Decision::Reject => "Rejected",
    }
}

fn decision_color(decision: Decision) -> Color32 {
    match decision {
        Decision::Unreviewed => MUTED,
        Decision::Keep => KEEP,
        Decision::Reject => REJECT,
    }
}

pub fn run(folder: Option<PathBuf>, state_root: PathBuf) -> Result<(), String> {
    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([900.0, 640.0])
            .with_app_id("NamiFL")
            .with_icon(egui::IconData::default()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "NamiFL — Cull",
        options,
        Box::new(move |cc| Ok(Box::new(CullApp::new(cc, folder, state_root)))),
    )
    .map_err(|error| error.to_string())
}

fn review_key_events(events: &[egui::Event]) -> Vec<(Key, egui::Modifiers)> {
    events
        .iter()
        .filter_map(|event| {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                Some((*key, *modifiers))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_chords_use_keydown_modifiers_not_the_released_state() {
        let down = egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        };
        let up = egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let events = review_key_events(&[down, up]);
        assert_eq!(events.len(), 1);
        assert!(events[0].1.shift);
    }
}
