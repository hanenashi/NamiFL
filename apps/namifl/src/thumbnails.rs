//! Thumbnail layout and scrolling adapt to the pane without recentering visible cards.
use super::*;

const GAP: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct LayoutSpec {
    grid: bool,
    columns: usize,
    card_width: f32,
    card_height: f32,
}

impl LayoutSpec {
    fn for_pane(size: Vec2, preferred_width: f32) -> Self {
        let width = preferred_width.min((size.x - 14.0).max(60.0));
        let height = width * 0.68 + 30.0;
        let grid = size.y >= height * 2.0 + GAP;
        let columns = if grid {
            ((size.x - 14.0 + GAP) / (width + GAP)).floor().max(1.0) as usize
        } else {
            1
        };
        Self {
            grid,
            columns,
            card_width: width,
            card_height: height,
        }
    }

    fn content_extent(self, count: usize) -> f32 {
        let (cells, length) = if self.grid {
            (count.div_ceil(self.columns), self.card_height)
        } else {
            (count, self.card_width)
        };
        cells as f32 * length + cells.saturating_sub(1) as f32 * GAP
    }

    fn target_start(self, position: usize) -> f32 {
        if self.grid {
            (position / self.columns) as f32 * (self.card_height + GAP)
        } else {
            position as f32 * (self.card_width + GAP)
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ViewportState {
    layout: LayoutSpec,
    offset: f32,
    extent: f32,
}

pub(super) fn navigation_target(visible: &[usize], current: usize, delta: isize) -> Option<usize> {
    let position = visible.iter().position(|&index| index == current)?;
    let next = position
        .saturating_add_signed(delta)
        .min(visible.len().saturating_sub(1));
    (next != position).then(|| visible[next])
}

fn reveal_offset(offset: f32, start: f32, length: f32, viewport: f32, content: f32) -> f32 {
    let offset = if start < offset {
        start
    } else if start + length > offset + viewport {
        start + length - viewport
    } else {
        offset
    };
    offset.clamp(0.0, (content - viewport).max(0.0))
}

impl CullApp {
    pub(super) fn filmstrip(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().always_scroll_the_only_direction = true;
        ui.spacing_mut().item_spacing = Vec2::splat(GAP);
        ui.spacing_mut().slider_width = (ui.available_width() - 45.0).clamp(45.0, 140.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Size").small().color(MUTED));
            ui.add(
                egui::Slider::new(&mut self.settings.thumbnail_width, 90.0..=280.0)
                    .show_value(false),
            )
            .on_hover_text("Thumbnail size. The layout follows the pane shape.");
        });

        let available = ui.available_size();
        let layout = LayoutSpec::for_pane(available, self.settings.thumbnail_width);
        let visible = self.visible();
        let old = self.thumb_viewport.filter(|view| view.layout == layout);
        let reveal = self.scroll_to_current || old.is_none();
        let mut scroll = if layout.grid {
            egui::ScrollArea::vertical()
        } else {
            egui::ScrollArea::horizontal()
        }
        .id_salt(if layout.grid {
            "thumbnail-grid"
        } else {
            "thumbnail-strip"
        })
        .auto_shrink([false, false])
        .animated(false);

        if reveal {
            let viewport = old.map_or(
                if layout.grid {
                    available.y
                } else {
                    available.x
                },
                |view| view.extent,
            );
            let offset = old.map_or(0.0, |view| view.offset);
            let offset = visible
                .iter()
                .position(|&index| index == self.current)
                .map_or(0.0, |position| {
                    reveal_offset(
                        offset,
                        layout.target_start(position),
                        if layout.grid {
                            layout.card_height
                        } else {
                            layout.card_width
                        },
                        viewport,
                        layout.content_extent(visible.len()),
                    )
                });
            scroll = if layout.grid {
                scroll.vertical_scroll_offset(offset)
            } else {
                scroll.horizontal_scroll_offset(offset)
            };
        }
        self.scroll_to_current = false;

        let output = scroll.show(ui, |ui| {
            if layout.grid {
                for row in visible.chunks(layout.columns) {
                    ui.horizontal(|ui| {
                        for &index in row {
                            self.thumbnail_card(ui, index, layout);
                        }
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    for &index in &visible {
                        self.thumbnail_card(ui, index, layout);
                    }
                });
            }
        });
        self.thumb_viewport = Some(ViewportState {
            layout,
            offset: if layout.grid {
                output.state.clamped_offset().y
            } else {
                output.state.clamped_offset().x
            },
            extent: if layout.grid {
                output.inner_rect.height()
            } else {
                output.inner_rect.width()
            },
        });
    }

    fn thumbnail_card(&mut self, ui: &mut egui::Ui, index: usize, layout: LayoutSpec) {
        let card_width = layout.card_width;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(card_width, layout.card_height), Sense::click());
        if !ui.is_rect_visible(rect) {
            return;
        }
        self.paint_thumbnail_contents(ui, index, card_width, rect, response);
    }
}

impl CullApp {
    fn paint_thumbnail_contents(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        card_width: f32,
        rect: Rect,
        response: egui::Response,
    ) {
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
            match group.decision {
                Decision::Keep => Color32::from_rgb(33, 83, 57),
                Decision::Reject => Color32::from_rgb(96, 39, 43),
                Decision::Unreviewed if selected => Color32::from_rgb(36, 50, 59),
                Decision::Unreviewed => BACKGROUND,
            },
        );
        if selected {
            painter.rect_stroke(rect, 7.0, Stroke::new(1.5, ACCENT), StrokeKind::Inside);
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
                let scale = (image_rect.width() / size.x).min(image_rect.height() / size.y);
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
        let name: String = group
            .stem
            .chars()
            .take((card_width / 7.0) as usize)
            .collect();
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
        if group.decision != Decision::Unreviewed {
            painter.text(
                rect.right_top() + Vec2::new(-9.0, 9.0),
                Align2::RIGHT_TOP,
                if group.decision == Decision::Keep {
                    "KEEP"
                } else {
                    "REJECT"
                },
                FontId::monospace(10.0),
                Color32::WHITE,
            );
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_at_either_boundary_do_not_request_a_selection_change() {
        assert_eq!(navigation_target(&[4, 7, 9], 9, 1), None);
        assert_eq!(navigation_target(&[4, 7, 9], 4, -1), None);
        assert_eq!(navigation_target(&[4, 7, 9], 7, 1), Some(9));
        assert_eq!(navigation_target(&[], 0, 1), None);
    }

    #[test]
    fn repeated_last_card_reveals_never_overshoot_or_recenter() {
        let mut offset = 106.0;
        for _ in 0..30 {
            offset = reveal_offset(offset, 462.0, 144.0, 500.0, 606.0);
        }
        assert_eq!(offset, 106.0);
        assert_eq!(reveal_offset(0.0, 462.0, 144.0, 500.0, 606.0), 106.0);
        assert_eq!(reveal_offset(106.0, 0.0, 144.0, 500.0, 606.0), 0.0);
    }

    #[test]
    fn pane_shape_selects_a_strip_or_wrapping_grid() {
        let strip = LayoutSpec::for_pane(Vec2::new(1000.0, 150.0), 144.0);
        assert!(!strip.grid);
        let grid = LayoutSpec::for_pane(Vec2::new(530.0, 600.0), 144.0);
        assert!(grid.grid);
        assert_eq!(grid.columns, 3);
        let narrow = LayoutSpec::for_pane(Vec2::new(180.0, 600.0), 280.0);
        assert!(narrow.grid);
        assert_eq!(narrow.columns, 1);
        assert!(narrow.card_width <= 180.0);
    }
}
