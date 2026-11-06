use egui::{
    epaint::PathShape, Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};

use crate::data::{Page, PIN_MAX};
use crate::theme::*;
use crate::App;

fn pts(centre: Pos2, size: f32, points: &[(f32, f32)]) -> Vec<Pos2> {
    let scale = size / 20.0;
    points
        .iter()
        .map(|(x, y)| {
            Pos2::new(
                centre.x + (x - 10.0) * scale,
                centre.y + (y - 10.0) * scale,
            )
        })
        .collect()
}

fn home_icon(painter: &egui::Painter, centre: Pos2, size: f32, colour: Color32) {
    let stroke = Stroke::new(size / 20.0 * 1.4, colour);
    painter.add(PathShape::closed_line(
        pts(
            centre,
            size,
            &[
                (3.0, 9.2),
                (10.0, 3.0),
                (17.0, 9.2),
                (17.0, 17.0),
                (16.0, 18.0),
                (11.8, 18.0),
                (11.8, 12.8),
                (8.2, 12.8),
                (8.2, 18.0),
                (4.0, 18.0),
                (3.0, 17.0),
            ],
        ),
        stroke,
    ));
}

fn cube_icon(painter: &egui::Painter, centre: Pos2, size: f32, colour: Color32) {
    let stroke = Stroke::new(size / 20.0 * 1.4, colour);
    painter.add(PathShape::closed_line(
        pts(
            centre,
            size,
            &[
                (10.0, 2.5),
                (17.0, 6.2),
                (17.0, 13.8),
                (10.0, 17.5),
                (3.0, 13.8),
                (3.0, 6.2),
            ],
        ),
        stroke,
    ));
    for line in [
        [(10.0, 10.2), (17.0, 6.2)],
        [(10.0, 10.2), (3.0, 6.2)],
        [(10.0, 10.2), (10.0, 17.5)],
    ] {
        painter.add(PathShape::line(pts(centre, size, &line), stroke));
    }
}

fn mod_icon(painter: &egui::Painter, centre: Pos2, size: f32, colour: Color32) {
    let stroke = Stroke::new(size / 20.0 * 1.4, colour);
    painter.add(PathShape::closed_line(
        pts(
            centre,
            size,
            &[
                (8.0, 3.0),
                (12.0, 3.0),
                (12.0, 6.0),
                (15.0, 6.0),
                (15.0, 10.0),
                (12.0, 10.0),
                (12.0, 13.0),
                (8.0, 13.0),
                (8.0, 10.0),
                (5.0, 10.0),
                (5.0, 6.0),
                (8.0, 6.0),
            ],
        ),
        stroke,
    ));
    painter.add(PathShape::closed_line(
        pts(
            centre,
            size,
            &[(12.0, 14.0), (15.0, 14.0), (15.0, 17.0), (12.0, 17.0)],
        ),
        stroke,
    ));
}

fn gear_icon(painter: &egui::Painter, centre: Pos2, size: f32, colour: Color32) {
    let scale = size / 20.0;
    let stroke = Stroke::new(scale * 1.4, colour);
    painter.circle_stroke(centre, 2.4 * scale, stroke);
    for line in [
        [(10.0, 2.8), (10.0, 4.9)],
        [(10.0, 15.1), (10.0, 17.2)],
        [(2.8, 10.0), (4.9, 10.0)],
        [(15.1, 10.0), (17.2, 10.0)],
        [(4.8, 4.8), (6.3, 6.3)],
        [(13.7, 13.7), (15.2, 15.2)],
        [(15.2, 4.8), (13.7, 6.3)],
        [(6.3, 13.7), (4.8, 15.2)],
    ] {
        painter.add(PathShape::line(pts(centre, size, &line), stroke));
    }
}

impl App {
    pub fn titlebar(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, TITLEBAR_BG);
        painter.text(
            Pos2::new(rect.min.x + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            "Rus Launcher",
            regular(12.0),
            Color32::from_rgb(0xc8, 0xc9, 0xce),
        );

        let drag = ui.interact(
            Rect::from_min_size(rect.min, Vec2::new(rect.width() - 72.0, rect.height())),
            ui.id().with("drag"),
            Sense::drag(),
        );
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let close = Rect::from_min_size(
            Pos2::new(rect.max.x - 36.0, rect.min.y + 4.0),
            Vec2::new(36.0, 28.0),
        );
        let min = Rect::from_min_size(
            Pos2::new(rect.max.x - 72.0, rect.min.y + 4.0),
            Vec2::new(36.0, 28.0),
        );
        for (rect, id, kind) in [(min, "win-min", 0), (close, "win-close", 1)] {
            let response = ui.interact(rect, ui.id().with(id), Sense::click());
            let t = ui
                .ctx()
                .animate_bool_with_time(ui.id().with(id).with("hover"), response.hovered(), 0.1);
            let bg = if kind == 1 {
                mix(
                    Color32::TRANSPARENT,
                    Color32::from_rgb(0xe2, 0x3b, 0x45),
                    t,
                )
            } else {
                mix(Color32::TRANSPARENT, Color32::from_rgb(0x2a, 0x2a, 0x2e), t)
            };
            ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);
            let colour = if kind == 1 && t > 0.02 {
                mix(
                    Color32::from_rgb(0xd5, 0xd6, 0xdb),
                    Color32::WHITE,
                    t,
                )
            } else {
                Color32::from_rgb(0xd5, 0xd6, 0xdb)
            };
            let stroke = Stroke::new(1.2, colour);
            let centre = rect.center();
            if kind == 0 {
                ui.painter().line_segment(
                    [
                        Pos2::new(centre.x - 5.0, centre.y),
                        Pos2::new(centre.x + 5.0, centre.y),
                    ],
                    stroke,
                );
            } else {
                ui.painter().line_segment(
                    [
                        Pos2::new(centre.x - 4.0, centre.y - 4.0),
                        Pos2::new(centre.x + 4.0, centre.y + 4.0),
                    ],
                    stroke,
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(centre.x + 4.0, centre.y - 4.0),
                        Pos2::new(centre.x - 4.0, centre.y + 4.0),
                    ],
                    stroke,
                );
            }
            if response.clicked() {
                if kind == 0 {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                } else {
                    self.state.save();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    pub fn rail(&mut self, ui: &mut Ui, rect: Rect) {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, RAIL_BG);
        let ctx = ui.ctx().clone();
        let centre_x = rect.center().x;
        let mut y = rect.min.y + 14.0;

        // brand
        if let Some(texture) = self.assets.id(&ctx, "brand/mark.png") {
            let mark = Rect::from_min_size(Pos2::new(centre_x - 16.0, y), Vec2::new(32.0, 32.0));
            ui.painter().add(
                egui::epaint::RectShape::filled(mark, CornerRadius::ZERO, Color32::WHITE)
                    .with_texture(texture, uv()),
            );
        }
        y += 32.0 + 10.0;

        // pins
        let pins = self.state.pins.clone();
        for id in pins.iter() {
            let instance = match self.state.instance(*id) {
                Some(value) => value.clone(),
                None => continue,
            };
            let version = self.state.list[self.state.version(&instance.version)].clone();
            let art = instance.image.clone().unwrap_or_else(|| version.poster());
            let box_rect = Rect::from_min_size(Pos2::new(centre_x - 24.0, y), Vec2::new(48.0, 48.0));
            let response = ui.interact(box_rect, ui.id().with("pin").with(*id), Sense::click());
            let t = ui.ctx().animate_bool_with_time(
                ui.id().with("pin-hover").with(*id),
                response.hovered(),
                0.12,
            );
            let active = self.state.current == *id && self.state.page == Page::Instance;
            let painter = ui.painter();
            painter.rect_filled(box_rect, CornerRadius::same(RADIUS_PIN), CARD);
            let clipped = painter.with_clip_rect(box_rect);
            if let Some(texture) = self.assets.id(&ctx, &art) {
                let aspect = self.assets.aspect(&ctx, &art);
                cover_image(&clipped, box_rect, texture, aspect, RADIUS_PIN);
            } else {
                clipped.text(
                    box_rect.center(),
                    Align2::CENTER_CENTER,
                    instance.name.chars().next().unwrap_or('R').to_uppercase().to_string(),
                    bold(15.0),
                    Color32::from_rgb(0xe8, 0xe9, 0xee),
                );
            }
            let ring = if active {
                RED
            } else {
                mix(
                    Color32::TRANSPARENT,
                    Color32::from_rgb(0x4a, 0x4a, 0x55),
                    t,
                )
            };
            if ring != Color32::TRANSPARENT {
                painter.rect_stroke(
                    box_rect.expand(1.0),
                    CornerRadius::same(RADIUS_PIN),
                    Stroke::new(2.0, ring), egui::StrokeKind::Inside);
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                self.state.current = *id;
                self.set_page(Page::Instance);
            }
            y += 48.0 + 6.0;
        }
        if !self.state.pins.is_empty() {
            ui.painter().text(
                Pos2::new(centre_x, y + 2.0),
                Align2::CENTER_TOP,
                format!("{}/{}", self.state.pins.len(), PIN_MAX),
                medium(9.0),
                FAINT,
            );
            y += 14.0;
        }
        y += 8.0;

        let bottom = rect.max.y - 12.0 - 58.0;
        let tabs = [
            ("Home", Page::Home),
            ("Instance", Page::Instance),
            ("Mods", Page::Mods),
        ];
        for (index, (label, page)) in tabs.iter().enumerate() {
            let box_rect = Rect::from_min_size(
                Pos2::new(centre_x - 32.0, y + index as f32 * 64.0),
                Vec2::new(64.0, 58.0),
            );
            if box_rect.max.y > bottom {
                break;
            }
            self.tab(ui, box_rect, label, page);
        }
        let settings = Rect::from_min_size(
            Pos2::new(centre_x - 32.0, bottom),
            Vec2::new(64.0, 58.0),
        );
        self.tab(ui, settings, "Settings", &Page::Settings);
    }

    fn tab(&mut self, ui: &mut Ui, rect: Rect, label: &str, page: &Page) {
        let active = self.state.page == *page;
        let response = ui.interact(rect, ui.id().with("tab").with(label), Sense::click());
        let t = ui
            .ctx()
            .animate_bool_with_time(ui.id().with("tab-hover").with(label), response.hovered(), 0.12);
        let painter = ui.painter();
        let fill = if active {
            Color32::from_rgb(0x1c, 0x1c, 0x20)
        } else {
            mix(Color32::TRANSPARENT, Color32::from_rgb(0x1a, 0x1a, 0x1e), t)
        };
        painter.rect_filled(rect, CornerRadius::same(RADIUS), fill);
        if active {
            painter.rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                CornerRadius::ZERO,
                RED,
            );
        }
        let colour = if active {
            Color32::WHITE
        } else {
            mix(Color32::from_rgb(0x9b, 0x9c, 0xa3), Color32::WHITE, t)
        };
        let icon_centre = Pos2::new(rect.center().x, rect.min.y + 21.0);
        match label {
            "Home" => home_icon(painter, icon_centre, 20.0, colour),
            "Instance" => cube_icon(painter, icon_centre, 20.0, colour),
            "Mods" => mod_icon(painter, icon_centre, 20.0, colour),
            _ => gear_icon(painter, icon_centre, 20.0, colour),
        }
        tracked(
            painter,
            Pos2::new(rect.center().x, rect.max.y - 12.0),
            &label.to_uppercase(),
            medium(9.0),
            colour,
            Align2::CENTER_CENTER,
            0.36,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            self.set_page(page.clone());
        }
    }
}
