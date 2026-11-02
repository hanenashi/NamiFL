//! Review controls wrap to fit a wide/shallow pane or a narrow sidebar.
use super::*;

impl CullApp {
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        ui.spacing_mut().button_padding = Vec2::new(8.0, 5.0);
        let Some(group) = self.current_group().cloned() else {
            ui.label(RichText::new("Open a folder and select a photo to review.").color(MUTED));
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&group.stem).size(17.0).strong())
                .on_hover_text(
                    group
                        .jpegs
                        .iter()
                        .chain(&group.raws)
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            ui.label(
                RichText::new(format!(
                    "{} JPEG / {} RAW",
                    group.jpegs.len(),
                    group.raws.len()
                ))
                .small()
                .color(MUTED),
            );
            ui.colored_label(
                decision_color(group.decision),
                decision_text(group.decision),
            );
            if self.selected.len() != 1 {
                ui.label(format!("{} selected", self.selected.len()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            let enabled = !self.selected.is_empty() && !self.opening;
            for (decision, label) in [
                (Decision::Keep, "Keep  K"),
                (Decision::Reject, "Reject  D"),
                (Decision::Unreviewed, "Unreviewed  U"),
            ] {
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(RichText::new(label).color(decision_color(decision)))
                            .min_size(Vec2::new(64.0, 27.0)),
                    )
                    .clicked()
                {
                    self.mark(decision);
                }
            }
            ui.checkbox(&mut self.settings.auto_advance, "Auto-advance");
            if compact && group.jpegs.len() > 1 {
                ui.menu_button("JPEG variant", |ui| self.review_files(ui, &group));
            }
        });
        if !compact {
            ui.separator();
            egui::CollapsingHeader::new("Source files")
                .default_open(true)
                .show(ui, |ui| self.review_files(ui, &group));
            egui::CollapsingHeader::new("DxO handoff").show(ui, |ui| {
                ui.label(RichText::new("Develop your keepers in DxO and export finished JPEGs to a separate folder. Transport comes next.").color(MUTED));
            });
        }
    }

    fn review_files(&mut self, ui: &mut egui::Ui, group: &PhotoGroup) {
        if group.is_ambiguous() {
            ui.label(RichText::new("Multiple variants — choose a JPEG to preview.").color(ACCENT));
        }
        ui.horizontal_wrapped(|ui| {
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
        });
        if group.jpegs.is_empty() {
            ui.label("RAW-only group. No JPEG preview.");
        }
    }
}
