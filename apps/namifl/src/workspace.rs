//! Movable, resizable workspace panes, preferences and explicit cleanup preview.
use super::*;

impl CullApp {
    pub(super) fn prepare_delete(&mut self) {
        if self.opening || self.revision != self.saved_revision {
            return;
        }
        let Some(loaded) = &self.loaded else {
            return;
        };
        match plan_deletion(&loaded.groups, &loaded.stamps) {
            Ok(plan) if plan.files.is_empty() => {
                self.message = Some(
                    "Nothing to delete. Unreviewed files and keeper JPEGs are retained.".into(),
                )
            }
            Ok(plan) => self.delete_plan = Some(plan),
            Err(error) => self.message = Some(error),
        }
    }

    pub(super) fn workspace(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let area = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(8))
            .show(ui, |ui| ui.available_rect_before_wrap())
            .inner;
        let strip_height = (self.settings.thumbnail_width * 0.68 + 110.0).clamp(180.0, 310.0);
        let preview_rect =
            Rect::from_min_max(area.min, area.max - Vec2::new(290.0, strip_height + 8.0));
        let review_rect = Rect::from_min_max(
            Pos2::new(area.max.x - 280.0, area.min.y),
            Pos2::new(area.max.x, area.max.y - strip_height - 8.0),
        );
        let strip_rect =
            Rect::from_min_max(Pos2::new(area.min.x, area.max.y - strip_height), area.max);
        let reset = self.layout_reset;

        let mut open = self.settings.show_preview;
        if open {
            pane("Preview", preview_rect, area, reset)
                .open(&mut open)
                .show(&ctx, |ui| self.preview(ui));
            self.settings.show_preview = open;
        }
        let mut open = self.settings.show_review;
        if open {
            pane("Review", review_rect, area, reset)
                .open(&mut open)
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.sidebar(ui));
                });
            self.settings.show_review = open;
        }
        let mut open = self.settings.show_thumbnails;
        if open {
            pane("Thumbnails", strip_rect, area, reset)
                .min_height((self.settings.thumbnail_width * 0.68 + 65.0).min(area.height() - 80.0))
                .open(&mut open)
                .show(&ctx, |ui| self.filmstrip(ui));
            self.settings.show_thumbnails = open;
        }
        self.layout_reset = false;
        self.settings_window(&ctx);
        self.delete_window(&ctx);
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = true;
        egui::Window::new("Settings").id(egui::Id::unique("settings"))
            .open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading(format!("NamiFL {}", env!("CARGO_PKG_VERSION")));
                ui.hyperlink_to("NamiFL on GitHub", "https://github.com/hanenashi/NamiFL");
                ui.separator();
                ui.label("Loupe magnification levels (%)");
                ui.text_edit_singleline(&mut self.loupe_edit);
                ui.label(RichText::new("Hold right-click for the loupe. Middle-click cycles levels.").small().color(MUTED));
                if ui.button("Apply loupe levels").clicked() {
                    match parse_loupe_levels(&self.loupe_edit) {
                        Ok(levels) => { self.settings.loupe_levels = levels; self.loupe_index = 0; }
                        Err(error) => { self.message = Some(error); }
                    }
                }
                ui.separator();
                ui.checkbox(&mut self.settings.auto_advance, "Advance after tagging one photo");
                ui.add(egui::Slider::new(&mut self.settings.thumbnail_width, 90.0..=280.0).text("Thumbnail size"));
                ui.label(RichText::new("Drag pane title bars to rearrange. Resize from pane edges. Restore closed panes from View.").color(MUTED));
            });
        self.show_settings = open;
    }

    fn delete_window(&mut self, ctx: &egui::Context) {
        let Some(plan) = self.delete_plan.clone() else {
            return;
        };
        egui::Modal::new(egui::Id::unique("delete-confirmation")).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.heading("Delete reviewed files?");
            ui.label(format!("Delete {} JPEGs and {} RAWs permanently.", plan.jpeg_count, plan.raw_count));
            ui.colored_label(KEEP, format!("Keep {} keeper JPEGs.", plan.keeper_jpegs));
            ui.label(format!("{} unreviewed groups are excluded. This applies to the entire folder, regardless of the current filter or selection.", plan.unreviewed_groups));
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for file in &plan.files { ui.label(file.path.file_name().unwrap_or(file.path.as_os_str()).to_string_lossy()); }
            });
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() { self.delete_plan = None; }
                if ui.button(RichText::new(format!("Delete {} files", plan.files.len())).color(REJECT)).clicked() {
                    let Some(loaded) = &self.loaded else { return; };
                    self.generation += 1;
                    self.media.clear();
                    self.opening = true;
                    let request = IoRequest::Delete { folder: loaded.session.folder.clone(), plan: plan.clone(), session: loaded.session.clone(), generation: self.generation };
                    if self.io.tx.send(request).is_err() { self.opening = false; self.message = Some("The file worker stopped. No cleanup was started.".into()); }
                    self.delete_plan = None;
                }
            });
        });
    }
}

fn pane<'a>(name: &'static str, slot: Rect, area: Rect, reset: bool) -> egui::Window<'a> {
    let body_size = (slot.size() - Vec2::new(24.0, 50.0)).max(Vec2::new(160.0, 90.0));
    let window = egui::Window::new(name)
        .id(egui::Id::unique(("workspace-pane", name)))
        .default_pos(slot.min)
        .default_size(body_size)
        .min_size([160.0, 90.0])
        .constrain_to(area)
        .resizable(true)
        .collapsible(true);
    if reset {
        window.current_pos(slot.min).fixed_size(body_size)
    } else {
        window
    }
}

fn parse_loupe_levels(input: &str) -> Result<Vec<u32>, String> {
    let levels: Result<Vec<u32>, _> = input.split(',').map(|part| part.trim().parse()).collect();
    match levels {
        Ok(levels) if !levels.is_empty() && levels.len() <= 12 && levels.iter().all(|level| (25..=2000).contains(level)) => Ok(levels),
        _ => Err("Enter 1–12 percentages from 25 to 2000, separated by commas (for example: 100, 200, 500).".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_loupe_levels() {
        assert_eq!(
            parse_loupe_levels("100, 200, 500").unwrap(),
            [100, 200, 500]
        );
        assert!(parse_loupe_levels("0, 100").is_err());
        assert!(parse_loupe_levels("100, nope").is_err());
    }
}
