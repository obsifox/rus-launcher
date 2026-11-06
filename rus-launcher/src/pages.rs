use egui::{Align2, Color32, Context, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::assets::Assets;
use crate::data::{Kind,Page, State, TYPES};
use crate::theme::*;
use crate::App;

pub fn text_width(ctx: &Context, label: &str, font: &FontId) -> f32 {
    ctx.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE).size().x)
}

pub fn fit(label: &str, font: &FontId, pad: f32) -> f32 {
    let ctx = egui::Context::default();
    let _ = &ctx;
    // measured at draw time instead; keep a sane estimate here
    label.chars().count() as f32 * font.size * 0.55 + pad * 2.0
}

/// A paragraph in the muted note colour.
pub fn note(ui: &mut Ui, value: &str) {
    let ctx = ui.ctx().clone();
    let width = ui.available_width();
    let galley = ctx.fonts(|f| f.layout(value.to_string(), regular(13.0), MUTED, width));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());
    ui.painter().galley(rect.min, galley, MUTED);
}

pub fn heading(ui: &mut Ui, value: &str, size: f32) {
    let ctx = ui.ctx().clone();
    let galley = ctx.fonts(|f| f.layout(value.to_string(), bold(size), TEXT, ui.available_width()));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter()
        .galley(rect.min, galley, TEXT);
}

pub fn field_label(ui: &mut Ui, value: &str) {
    let ctx = ui.ctx().clone();
    let font = bold(11.0);
    let galley = ctx.fonts(|f| f.layout(value.to_string(), font.clone(), MUTED, 10000.0));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), galley.size().y + 18.0), Sense::hover());
    tracked(
        ui.painter(),
        Pos2::new(rect.min.x, rect.min.y + 9.0),
        value,
        font,
        MUTED,
        Align2::LEFT_TOP,
        1.76,
    );
}

/// The red uppercase line above a heading.
pub fn kicker(painter: &egui::Painter, pos: Pos2, value: &str) {
    tracked(
        painter,
        pos,
        value,
        bold(11.0),
        RED_HI,
        Align2::LEFT_TOP,
        1.76,
    );
}

/// A single line text field with the launcher look.
pub fn input(ui: &mut Ui, value: &mut String, width: f32, hint: &str) -> egui::Response {
    egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            ui.set_min_width(width);
            ui.add_sized(
                Vec2::new(width - 24.0, 36.0),
                egui::TextEdit::singleline(value)
                    .hint_text(egui::RichText::new(hint).color(FAINT).font(regular(13.0)))
                    .font(regular(13.0))
                    .text_color(TEXT)
                    .frame(false)
                    .vertical_align(egui::Align::Center),
            )
        })
        .response
}

fn chips_row(ui: &mut Ui, items: &[(&str, bool)], id: &str) -> Option<String> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for (index, (label, active)) in items.iter().enumerate() {
            let font = medium(13.0);
            let width = text_width(ui.ctx(), label, &font) + 24.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::hover());
            let style = Button::filled(*label).on(*active);
            if button(ui, ui.id().with(id).with(index), rect, &style).clicked() {
                picked = Some(label.to_string());
            }
        }
    });
    picked
}

impl App {
    /* ------------------------------------------------------------------ home */

    pub fn page_home(&mut self, ui: &mut Ui) {
        let id = self.state.current;
        let version = self.state.current_version().clone();
        let name = self.state.current_instance().name.clone();
        let full = ui.max_rect();
        let hero_h = (full.height() * 0.46).max(260.0);

        // hero
        let hero = Rect::from_min_size(full.min, Vec2::new(full.width(), hero_h));
        let ctx = ui.ctx().clone();
        if let Some(texture) = self.assets.id(&ctx, &version.poster()) {
            let aspect = self.assets.aspect(&ctx, &version.poster());
            cover_image(ui.painter(), hero, texture, aspect, 0);
        } else {
            ui.painter().rect_filled(hero, CornerRadius::ZERO, Color32::from_rgb(0x11, 0x11, 0x11));
        }
        let shade = [
            Color32::from_rgba_unmultiplied(8, 8, 10, 224),
            Color32::from_rgba_unmultiplied(8, 8, 10, 128),
            Color32::from_rgba_unmultiplied(8, 8, 10, 13),
        ];
        fill_h(
            ui.painter(),
            Rect::from_min_max(hero.min, Pos2::new(hero.min.x + hero.width() * 0.36, hero.max.y)),
            shade[0],
            shade[1],
        );
        fill_h(
            ui.painter(),
            Rect::from_min_max(
                Pos2::new(hero.min.x + hero.width() * 0.36, hero.min.y),
                Pos2::new(hero.min.x + hero.width() * 0.68, hero.max.y),
            ),
            shade[1],
            shade[2],
        );

        // hero copy
        let left = hero.min.x + 36.0;
        let mut y = hero.max.y - 28.0;
        let play_label = if version.installed { "PLAY" } else { "INSTALL" };
        let play = self.transient_label("play", play_label, "LAUNCHING", 0.8);
        let play_rect = Rect::from_min_size(
            Pos2::new(left, y - 38.0),
            Vec2::new(text_width(ui.ctx(), &play, &bold(13.0)) + 36.0, 38.0),
        );
        y = play_rect.min.y - 18.0;
        let lede = format!(
            "{} is set on {}. {}",
            version.version,
            name,
            if version.installed {
                "Installed and ready to play."
            } else {
                "Not installed yet."
            }
        );
        let lede_galley = ctx.fonts(|f| {
            f.layout(
                lede.clone(),
                regular(14.0),
                DIM,
                (hero.width() * 0.64).min(460.0),
            )
        });
        y -= lede_galley.size().y;
        ui.painter().galley(Pos2::new(left, y), lede_galley, DIM);
        y -= 10.0 + 40.0;
        ui.painter().text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            version.name,
            bold(40.0),
            TEXT,
        );
        y -= 10.0 + 14.0;
        kicker(ui.painter(), Pos2::new(left, y), "LATEST UPDATE");

        if button(
            ui,
            ui.id().with("hero-play"),
            play_rect,
            &Button::primary(play.clone()),
        )
        .clicked()
        {
            if !self.state.list[self.state.version(&version.version)].installed {
                let index = self.state.version(&version.version);
                self.state.list[index].installed = true;
                self.state.save();
            } else {
                self.transient_set("play", 0.8);
            }
        }

        // feed
        let feed = Rect::from_min_max(
            Pos2::new(full.min.x, hero.max.y),
            full.max,
        );
        let pad = 16.0;
        let gap = 14.0;
        let inner = feed.shrink2(Vec2::new(18.0, 0.0));
        let top = feed.min.y + pad;
        let row_h = 292.0f32;
        let total = inner.width() - gap * 3.0;
        let weights = [1.15f32, 1.0, 1.0, 0.9];
        let sum: f32 = weights.iter().sum();
        let others: Vec<usize> = (0..self.state.list.len())
            .filter(|i| *i != self.state.version(&version.version))
            .take(3)
            .collect();

        let mut x = inner.min.x;
        for (slot, weight) in weights.iter().enumerate() {
            let w = total * weight / sum;
            let rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(w, row_h.min(feed.height() - pad - 18.0)));
            x += w + gap;
            if slot < 3 {
                if let Some(index) = others.get(slot) {
                    self.news_card(ui, *index, rect);
                }
            } else {
                self.game_tile(ui, rect, &version.poster(), &name);
            }
        }
        let _ = id;
    }

    fn news_card(&mut self, ui: &mut Ui, index: usize, rect: Rect) {
        let version = self.state.list[index].clone();
        let response = ui.interact(rect, ui.id().with("news").with(index), Sense::click());
        let t = ui.ctx().animate_bool_with_time(
            ui.id().with("news-hover").with(index),
            response.hovered(),
            0.14,
        );
        let lift = 2.0 * t;
        let rect = rect.translate(Vec2::new(0.0, -lift));
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(RADIUS), mix(CARD, CARD_HOVER, t));
        let clipped = painter.with_clip_rect(rect);
        let thumb = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 156.0));
        let ctx = ui.ctx().clone();
        if let Some(texture) = self.assets.id(&ctx, &version.poster()) {
            let aspect = self.assets.aspect(&ctx, &version.poster());
            cover_image(&clipped, thumb, texture, aspect, 0);
        } else {
            clipped.rect_filled(thumb, CornerRadius::ZERO, Color32::from_rgb(0x11, 0x11, 0x11));
        }
        let pad_x = 14.0;
        let mut y = thumb.max.y + 12.0;
        tracked(
            &clipped,
            Pos2::new(rect.min.x + pad_x, y),
            if version.installed { "INSTALLED" } else { "UPDATE" },
            bold(10.0),
            RED_HI,
            Align2::LEFT_TOP,
            1.4,
        );
        y += 8.0 + 15.0;
        clipped.text(
            Pos2::new(rect.min.x + pad_x, y),
            Align2::LEFT_TOP,
            format!("{} {}", version.version, version.name),
            bold(15.0),
            TEXT,
        );
        y += 19.0 + 6.0;
        let body = ctx.fonts(|f| {
            f.layout(
                version.blurb.to_string(),
                regular(12.0),
                MUTED,
                rect.width() - pad_x * 2.0,
            )
        });
        let body_h = body.size().y;
        clipped.galley(Pos2::new(rect.min.x + pad_x, y), body, MUTED);
        y += body_h + 12.0;
        tracked(
            &clipped,
            Pos2::new(rect.min.x + pad_x, y),
            "READ MORE",
            bold(11.0),
            RED_HI,
            Align2::LEFT_TOP,
            1.4,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            let target = self.state.list[index].version.to_string();
            self.state.current_instance_mut().version = target;
            self.state.save();
            self.set_page(Page::Instance);
        }
    }

    fn game_tile(&mut self, ui: &mut Ui, rect: Rect, poster: &str, name: &str) {
        let response = ui.interact(rect, ui.id().with("game-tile"), Sense::click());
        let t = ui
            .ctx()
            .animate_bool_with_time(ui.id().with("game-hover"), response.hovered(), 0.14);
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(RADIUS), mix(CARD, CARD_HOVER, t));
        let clipped = painter.with_clip_rect(rect);
        let ctx = ui.ctx().clone();
        if let Some(texture) = self.assets.id(&ctx, poster) {
            let aspect = self.assets.aspect(&ctx, poster);
            cover_image(&clipped, rect, texture, aspect, RADIUS);
        }
        fill_v(
            &clipped,
            rect,
            Color32::from_rgba_unmultiplied(0, 0, 0, 13),
            Color32::from_rgba_unmultiplied(0, 0, 0, 184),
        );
        let bottom = rect.max.y - 14.0;
        clipped.text(
            Pos2::new(rect.min.x + 14.0, bottom - 17.0),
            Align2::LEFT_TOP,
            name,
            bold(14.0),
            TEXT,
        );
        tracked(
            &clipped,
            Pos2::new(rect.min.x + 14.0, bottom - 17.0 - 12.0 - 10.0),
            "NOW PLAYING",
            bold(10.0),
            Color32::from_rgb(0xff, 0xb4, 0xb8),
            Align2::LEFT_TOP,
            1.2,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            self.set_page(Page::Instance);
        }
    }

    /* -------------------------------------------------------------- instance */

    pub fn page_instance(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let side_w = 340.0;
        let gap = 22.0;
        let form = Rect::from_min_max(
            full.min,
            Pos2::new(full.max.x - side_w, full.max.y),
        );
        let side = Rect::from_min_max(
            Pos2::new(full.max.x - side_w, full.min.y + gap),
            Pos2::new(full.max.x - gap, full.max.y - gap),
        );

        // The page has 28px of padding on each side, like the web build.
        let content = Rect::from_min_max(
            Pos2::new(form.min.x + 28.0, form.min.y),
            Pos2::new(form.max.x - 28.0, form.max.y),
        );
        ui.allocate_ui_at_rect(content, |ui| {
            egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.add_space(22.0);
                    heading(ui, "Instance", 28.0);
                    ui.add_space(8.0);
                    note(
                        ui,
                        "Installed clients on this machine. Everything about adding a version lives in New +.",
                    );
                    ui.add_space(16.0);

                    // top bar
                    let height = 34.0;
                    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
                    let chip_w = text_width(ui.ctx(), "Installed", &medium(13.0)) + 24.0;
                    let chip_rect = Rect::from_min_size(row.min, Vec2::new(chip_w, height));
                    let chip_style = Button::chip("Installed", true);
                    button(ui, ui.id().with("installed-chip"), chip_rect, &chip_style);
                    ui.painter().text(
                        chip_rect.center(),
                        Align2::CENTER_CENTER,
                        "Installed",
                        medium(13.0),
                        TEXT,
                    );
                    let pill_w = text_width(ui.ctx(), "New +", &bold(13.0)) + 32.0;
                    let pill = Rect::from_min_size(
                        Pos2::new(row.max.x - pill_w, row.min.y),
                        Vec2::new(pill_w, height),
                    );
                    if button(
                        ui,
                        ui.id().with("new-instance"),
                        pill,
                        &Button::primary("New +"),
                    )
                    .clicked()
                    {
                        self.open_new_modal();
                    }

                    ui.add_space(12.0);
                    let mut query = self.state.query.clone();

                    input(

                        ui,

                        &mut query,

                        ui.available_width().min(320.0),

                        "Search installed instances",

                    );

                    if query != self.state.query {
                        self.state.query = query;
                    }

                    if let Some(text) = self.state.flash_text().map(|s| s.to_string()) {
                        ui.add_space(10.0);
                        note_colour(ui, &text, Color32::from_rgb(0xff, 0xb4, 0xb8));
                    }

                    ui.add_space(14.0);
                    self.instance_cards(ui);
                    ui.add_space(28.0);
                });
        });

        self.instance_side(ui, side);
    }

    fn instance_cards(&mut self, ui: &mut Ui) {
        let ids = self.state.matches();
        if ids.is_empty() {
            note_colour(
                ui,
                &format!("No instance matches “{}”.", self.state.query),
                MUTED,
            );
            return;
        }
        let width = ui.available_width();
        let gap = 12.0;
        let min = 212.0;
        let columns = ((width + gap) / (min + gap)).floor().max(1.0) as usize;
        let card_w = (width - gap * (columns - 1) as f32) / columns as f32;
        let mut row_height = 0.0f32;
        let rows: Vec<&[u64]> = ids.chunks(columns).collect();
        for chunk in rows {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 260.0), Sense::hover());
            row_height = rect.height();
            for (slot, id) in chunk.iter().enumerate() {
                let x = rect.min.x + slot as f32 * (card_w + gap);
                let card = Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(card_w, 250.0));
                self.instance_card(ui, *id, card);
            }
            ui.add_space(gap);
        }
        let _ = row_height;
    }

    fn instance_card(&mut self, ui: &mut Ui, id: u64, rect: Rect) {
        let instance = match self.state.instance(id) {
            Some(value) => value.clone(),
            None => return,
        };
        let pinned = self.state.pins.contains(&id);
        let is_current = self.state.current == id;
        let version = self.state.list[self.state.version(&instance.version)].clone();
        let art = instance.image.clone().unwrap_or_else(|| version.poster());
        let loader_art = format!("tabs/{}@1x.png", instance.loader.to_lowercase());
        let response = ui.interact(rect, ui.id().with("icard").with(id), Sense::hover());
        let t = ui.ctx().animate_bool_with_time(
            ui.id().with("icard-hover").with(id),
            response.hovered(),
            0.14,
        );
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            CornerRadius::same(RADIUS),
            mix(CARD, CARD_HOVER, t),
        );
        painter.rect_stroke(rect, CornerRadius::same(RADIUS), Stroke::new(1.0, mix(BORDER, Color32::from_rgb(0x4a, 0x4a, 0x55), t)), egui::StrokeKind::Inside);
        let clipped = painter.with_clip_rect(rect);
        let cover = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 116.0));
        let ctx = ui.ctx().clone();
        if let Some(texture) = self.assets.id(&ctx, &art) {
            let aspect = self.assets.aspect(&ctx, &art);
            cover_image(&clipped, cover, texture, aspect, 0);
        } else {
            clipped.rect_filled(cover, CornerRadius::ZERO, Color32::from_rgb(0x11, 0x11, 0x11));
        }
        if let Some(texture) = self.assets.id(&ctx, &loader_art) {
            let badge = Rect::from_min_size(
                Pos2::new(rect.max.x - 8.0 - 54.0, rect.min.y + 8.0),
                Vec2::new(54.0, 16.0),
            );
            clipped.add(
                egui::epaint::RectShape::filled(badge, CornerRadius::same(2), Color32::WHITE)
                    .with_texture(texture, uv()),
            );
        }

        let pad_x = 12.0;
        let mut y = cover.max.y + 10.0;
        let name_galley = ctx.fonts(|f| {
            f.layout(
                instance.name.clone(),
                bold(14.0),
                TEXT,
                rect.width() - pad_x * 2.0,
            )
        });
        clipped.galley(Pos2::new(rect.min.x + pad_x, y), name_galley, TEXT);
        y += 17.0 + 5.0;
        let meta = format!(
            "{}{}",
            instance.version,
            if instance.source == "import" {
                " · imported"
            } else {
                ""
            }
        );
        clipped.text(
            Pos2::new(rect.min.x + pad_x, y),
            Align2::LEFT_TOP,
            meta.clone(),
            regular(12.0),
            MUTED,
        );
        let meta_w = text_width(&ctx, &meta, &regular(12.0));
        if let Some(texture) = self.assets.id(&ctx, &loader_art) {
            let tag = Rect::from_min_size(
                Pos2::new(rect.min.x + pad_x + meta_w + 6.0, y + 1.0),
                Vec2::new(54.0, 16.0),
            );
            clipped.add(
                egui::epaint::RectShape::filled(tag, CornerRadius::same(2), Color32::WHITE)
                    .with_texture(texture, uv()),
            );
        }

        // two wide buttons: everything about editing lives in the Edit popup,
        // and the servers now live in the Directory.
        let labels = ["EDIT", "DIRECTORY"];
        let row_top = rect.max.y - 12.0 - 28.0;
        let bw = (rect.width() - pad_x * 2.0 - 6.0) / 2.0;
        for (slot, label) in labels.iter().enumerate() {
            let btn = Rect::from_min_size(
                Pos2::new(
                    rect.min.x + pad_x + slot as f32 * (bw + 6.0),
                    row_top,
                ),
                Vec2::new(bw, 28.0),
            );
            let style = Button::ghost(*label).font(bold(9.0));
            let clicked = button(ui, ui.id().with("iact").with(id).with(slot), btn, &style).clicked();
            if clicked {
                match slot {
                    0 => self.open_edit(id),
                    _ => {
                        self.state.current = id;
                        self.state.save();
                        self.set_page(Page::Directory);
                    }
                }
            }
        }
    }

    fn instance_side(&mut self, ui: &mut Ui, rect: Rect) {
        let instance = self.state.current_instance().clone();
        let version = self.state.list[self.state.version(&instance.version)].clone();
        let art = instance.image.clone().unwrap_or_else(|| version.poster());
        let ctx = ui.ctx().clone();
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(RADIUS), Color32::from_rgb(0x11, 0x11, 0x11));
        let clipped = painter.with_clip_rect(rect);
        if let Some(texture) = self.assets.id(&ctx, &art) {
            let aspect = self.assets.aspect(&ctx, &art);
            cover_image(&clipped, rect, texture, aspect, RADIUS);
        }
        fill_v(
            &clipped,
            rect,
            Color32::from_rgba_unmultiplied(0, 0, 0, 13),
            Color32::from_rgba_unmultiplied(0, 0, 0, 209),
        );
        let bottom = rect.max.y - 18.0;
        let left = rect.min.x + 18.0;
        let label = self.transient_label("save", "SAVE", "SAVED", 0.8);
        let save_rect = Rect::from_min_size(
            Pos2::new(left, bottom - 38.0),
            Vec2::new(text_width(&ctx, &label, &bold(13.0)) + 36.0, 38.0),
        );
        let mut y = save_rect.min.y - 18.0;
        let info = format!(
            "{} · {} · {} mods · {} servers",
            instance.version,
            if instance.source == "import" { "Imported" } else { "New" },
            instance.mods.len(),
            instance.servers.len()
        );
        let info_galley = ctx.fonts(|f| f.layout(info.clone(), regular(13.0), DIM, rect.width() - 36.0));
        y -= info_galley.size().y;
        clipped.galley(Pos2::new(left, y), info_galley, DIM);
        y -= 8.0;
        let strong_galley = ctx.fonts(|f| {
            f.layout(
                instance.name.clone(),
                bold(22.0),
                TEXT,
                rect.width() - 36.0,
            )
        });
        y -= strong_galley.size().y;
        clipped.galley(Pos2::new(left, y), strong_galley, TEXT);
        y -= 8.0 + 16.0;
        let loader_art = format!("tabs/{}@1x.png", instance.loader.to_lowercase());
        if let Some(texture) = self.assets.id(&ctx, &loader_art) {
            clipped.add(
                egui::epaint::RectShape::filled(
                    Rect::from_min_size(Pos2::new(left, y), Vec2::new(54.0, 16.0)),
                    CornerRadius::same(2),
                    Color32::WHITE,
                )
                .with_texture(texture, uv()),
            );
            y -= 6.0 + 14.0;
        }
        kicker(&clipped, Pos2::new(left, y), &instance.loader.to_uppercase());

        if button(
            ui,
            ui.id().with("save-instance"),
            save_rect,
            &Button::primary(label),
        )
        .clicked()
        {
            self.state.save();
            self.transient_set("save", 0.8);
        }
    }

    /* ------------------------------------------------------------------ mods */

    pub fn page_mods(&mut self, ui: &mut Ui) {
        let instance = self.state.current_instance().clone();
        let version = self.state.list[self.state.version(&instance.version)].clone();
        let ctx = ui.ctx().clone();

        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.add_space(22.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 11.0), Sense::hover());
                kicker(
                    ui.painter(),
                    Pos2::new(rect.min.x + 28.0, rect.min.y),
                    "MODS",
                );
                ui.add_space(3.0);
                let inner = ui.max_rect();
                ui.allocate_ui_at_rect(
                    Rect::from_min_max(
                        Pos2::new(inner.min.x + 28.0, ui.cursor().min.y),
                        Pos2::new(inner.max.x - 28.0, ui.cursor().min.y + 29.0),
                    ),
                    |ui| heading(ui, &instance.name, 28.0),
                );
                ui.add_space(3.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 56.0, 18.0), Sense::hover());
                let text = format!(
                    "{} · {}{}",
                    version.version,
                    instance.loader,
                    if instance.source == "import" { " · Imported" } else { "" }
                );
                ui.painter().galley(
                    Pos2::new(rect.min.x + 28.0, rect.min.y),
                    ctx.fonts(|f| f.layout(text, regular(13.0), MUTED, rect.width())),
                    MUTED,
                );

                // browse row: the field, the chips and the button share one line
                ui.add_space(14.0);
                let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::hover());
                let mut query = self.search.query.clone();
                // `input()` adds 24px of padding and a 2px border around the width it is given,
                // so 234 is what makes the finished box 260 wide, like the web build.
                let field_rect = Rect::from_min_size(
                    Pos2::new(row.min.x + 28.0, row.min.y),
                    Vec2::new(260.0, 36.0),
                );
                let response = input(
                    &mut ui.child_ui(field_rect, egui::Layout::top_down(egui::Align::Min), None),
                    &mut query,
                    234.0,
                    "Search for content",
                );
                // Return starts the search whenever there is something to look for.
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !query.trim().is_empty() {
                    self.start_search();
                }
                if query != self.search.query {
                    self.search.query = query;
                }

                let sources = [
                    ("Modrinth", self.search.source == "modrinth"),
                    ("CurseForge", self.search.source == "curseforge"),
                ];
                let types: Vec<(&str, bool)> = TYPES
                    .iter()
                    .map(|(key, label)| (*label, self.search.kind == *key))
                    .collect();
                let mut x = row.min.x + 28.0 + 260.0 + 10.0;
                for (index, (label, active)) in sources.iter().enumerate() {
                    let font = medium(13.0);
                    let w = text_width(ui.ctx(), label, &font) + 24.0;
                    let rect = Rect::from_min_size(Pos2::new(x, row.min.y + 2.0), Vec2::new(w, 32.0));
                    x += w + 9.0;
                    let style = Button::chip(*label, *active);
                    if button(ui, ui.id().with("src").with(index), rect, &style).clicked() {
                        self.search.source = label.to_lowercase();
                        self.search.results.clear();
                        self.search.status.clear();
                    }
                }
                for (index, (label, active)) in types.iter().enumerate() {
                    let font = medium(13.0);
                    let w = text_width(ui.ctx(), label, &font) + 24.0;
                    if x + w > row.max.x - 120.0 {
                        break;
                    }
                    let rect = Rect::from_min_size(Pos2::new(x, row.min.y + 2.0), Vec2::new(w, 32.0));
                    x += w + 9.0;
                    let style = Button::chip(*label, *active);
                    if button(ui, ui.id().with("type").with(index), rect, &style).clicked() {
                        self.search.kind = TYPES[index].0.to_string();
                        self.search.results.clear();
                        self.search.status.clear();
                    }
                }
                let go = Rect::from_min_size(
                    Pos2::new(x + 8.0, row.min.y + 1.0),
                    Vec2::new(text_width(ui.ctx(), "SEARCH", &bold(12.0)) + 32.0, 34.0),
                );
                if button(ui, ui.id().with("search"), go, &Button::primary("SEARCH").font(bold(12.0))).clicked() {
                    self.start_search();
                }

                let status = self.search.status.clone();
                if !status.is_empty() {
                    ui.add_space(14.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 56.0, 18.0), Sense::hover());
                    ui.painter().galley(
                        Pos2::new(rect.min.x + 28.0, rect.min.y),
                        ctx.fonts(|f| f.layout(status, regular(14.0), MUTED, rect.width())),
                        MUTED,
                    );
                }

                // results
                let hits = self.search.results.clone();
                if !hits.is_empty() {
                    ui.add_space(14.0);
                    let width = ui.available_width() - 56.0;
                    let gap = 10.0;
                    let columns = ((width + gap) / (268.0 + gap)).floor().max(1.0) as usize;
                    let card_w = (width - gap * (columns - 1) as f32) / columns as f32;
                    for chunk in hits.chunks(columns) {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(width, 116.0), Sense::hover());
                        for (slot, hit) in chunk.iter().enumerate() {
                            let x = rect.min.x + 28.0 + slot as f32 * (card_w + gap);
                            let card = Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(card_w, 106.0));
                            self.hit_card(ui, card, hit, slot);
                        }
                        ui.add_space(gap);
                    }
                }

                // installed list
                ui.add_space(18.0);
                let title = format!("INSTALLED ON {}", instance.name.to_uppercase());
                field_label(ui, &title);
                let mods = instance.mods.clone();
                if mods.is_empty() {
                    note(ui, "No mods yet.");
                } else {
                    for (index, entry) in mods.iter().enumerate() {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width() - 56.0, 44.0), Sense::hover());
                        let item = Rect::from_min_max(
                            Pos2::new(rect.min.x + 28.0, rect.min.y),
                            Pos2::new(rect.max.x - 28.0, rect.min.y + 44.0),
                        );
                        ui.painter().rect_filled(item, CornerRadius::same(RADIUS), CARD);
                        ui.painter().text(
                            Pos2::new(item.min.x + 12.0, item.center().y),
                            Align2::LEFT_CENTER,
                            entry.name.clone(),
                            regular(13.0),
                            if entry.enabled { TEXT } else { FAINT },
                        );
                        let w = text_width(ui.ctx(), "REMOVE", &bold(11.0)) + 20.0;
                        let btn = Rect::from_min_size(
                            Pos2::new(item.max.x - 12.0 - w, item.center().y - 13.0),
                            Vec2::new(w, 26.0),
                        );
                        let style = Button::ghost("REMOVE").font(bold(10.0));
                        if button(ui, ui.id().with("mod-remove").with(index), btn, &style).clicked() {
                            self.state.current_instance_mut().mods.remove(index);
                            self.state.save();
                        }
                        ui.add_space(8.0);
                    }
                }
                ui.add_space(28.0);
            });
    }

    fn hit_card(&mut self, ui: &mut Ui, rect: Rect, hit: &crate::net::Hit, slot: usize) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(RADIUS), CARD);
        let clipped = painter.with_clip_rect(rect);
        let icon_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 10.0, rect.min.y + 10.0),
            Vec2::new(46.0, 46.0),
        );
        let ctx = ui.ctx().clone();
        let key = hit.icon_url.clone().unwrap_or_default();
        let texture = if !key.is_empty() {
            match hit.icon.clone() {
                Some(bytes) => self.assets.store(&ctx, &key, &bytes),
                None => None,
            }
        } else {
            self.assets.id(&ctx, "brand/mark.png")
        };
        if let Some(texture) = texture {
            clipped.add(
                egui::epaint::RectShape::filled(icon_rect, CornerRadius::same(RADIUS), Color32::WHITE)
                    .with_texture(texture, uv()),
            );
        } else {
            clipped.rect_filled(icon_rect, CornerRadius::same(RADIUS), Color32::from_rgb(0x11, 0x11, 0x11));
        }
        let text_x = icon_rect.max.x + 10.0;
        let wrap = rect.width() - (text_x - rect.min.x) - 10.0;
        let title = hit.title.clone();
        let galley = ctx.fonts(|f| f.layout(title, bold(13.0), TEXT, wrap));
        clipped.galley(Pos2::new(text_x, icon_rect.min.y), galley, TEXT);
        let body = if hit.description.chars().count() > 120 {
            format!("{}…", hit.description.chars().take(120).collect::<String>())
        } else {
            hit.description.clone()
        };
        let body_galley = ctx.fonts(|f| f.layout(body, regular(12.0), MUTED, wrap));
        clipped.galley(Pos2::new(text_x, icon_rect.min.y + 20.0), body_galley, MUTED);
        let downloads = format!("{} downloads", comma(hit.downloads));
        clipped.text(
            Pos2::new(text_x, icon_rect.min.y + 48.0),
            Align2::LEFT_TOP,
            downloads,
            regular(12.0),
            MUTED,
        );
        let label = if hit.added { "ADDED" } else { "ADD" };
        let w = text_width(&ctx, label, &bold(10.0)) + 20.0;
        let btn = Rect::from_min_size(
            Pos2::new(text_x, icon_rect.min.y + 66.0),
            Vec2::new(w, 28.0),
        );
        let style = Button::ghost(label).font(bold(10.0)).on(hit.added);
        if button(ui, ui.id().with("hit-add").with(slot).with(&hit.title), btn, &style).clicked() {
            let title = hit.title.clone();
            let description = hit.description.clone();
            let icon = hit.icon_url.clone();
            let slug = hit.slug.clone();
            let source = self.search.source.clone();
            let version = hit.version.clone();
            let mut added = false;
            {
                let instance = self.state.current_instance_mut();
                if !instance.mods.iter().any(|m| m.name == title) {
                    instance.mods.push(crate::data::Entry {
                        name: title.clone(),
                        slug: slug.clone(),
                        version,
                        enabled: true,
                        source,
                        description: Some(description),
                        icon,
                    });
                    added = true;
                }
            }
            if let Some(found) = self.search.results.iter_mut().find(|h| h.title == title) {
                found.added = true;
            }
            if added {
                match slug {
                    Some(value) => {
                        self.fetch_deps(&value, &title, crate::data::Kind::Mods, true);
                        self.state
                            .flash(format!("{title} added. Looking for dependencies…"));
                    }
                    None => self.state.flash(format!("{title} added.")),
                }
            }
            self.state.save();
        }
    }

    /* ------------------------------------------------------------- directory */

    /// Everything installed on the selected instance, in four drawers.
    pub fn page_directory(&mut self, ui: &mut Ui) {
        let instance = self.state.current_instance().clone();
        let version = self.state.list[self.state.version(&instance.version)].clone();
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.add_space(22.0);
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::hover());
                kicker(
                    ui.painter(),
                    Pos2::new(rect.min.x + 28.0, rect.min.y),
                    "DIRECTORY",
                );
                ui.add_space(8.0);
                let inner = ui.max_rect();
                ui.allocate_ui_at_rect(
                    Rect::from_min_max(
                        Pos2::new(inner.min.x + 28.0, ui.cursor().min.y),
                        Pos2::new(inner.max.x - 28.0, ui.cursor().min.y + 34.0),
                    ),
                    |ui| heading(ui, &instance.name, 28.0),
                );
                ui.add_space(8.0);
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width() - 56.0, 18.0),
                    Sense::hover(),
                );
                let text = format!(
                    "{} · {}{}",
                    version.version,
                    instance.loader,
                    if instance.source == "import" { " · Imported" } else { "" }
                );
                ui.painter().galley(
                    Pos2::new(rect.min.x + 28.0, rect.min.y),
                    ctx.fonts(|f| f.layout(text, regular(13.0), MUTED, rect.width())),
                    MUTED,
                );

                // drawers
                ui.add_space(18.0);
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::hover());
                let mut x = row.min.x + 28.0;
                for kind in Kind::ALL {
                    let label = kind.tab();
                    let count = instance.count(kind);
                    let text = format!("{label} · {count}");
                    let w = text_width(ui.ctx(), &text, &medium(13.0)) + 24.0;
                    let rect = Rect::from_min_size(Pos2::new(x, row.min.y + 2.0), Vec2::new(w, 32.0));
                    x += w + 9.0;
                    let style = Button::chip(&text, self.drawer == kind);
                    if button(ui, ui.id().with("drawer").with(label), rect, &style).clicked() {
                        self.drawer = kind;
                    }
                }

                // add row
                ui.add_space(16.0);
                let kind = self.drawer;
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width() - 56.0, 36.0), Sense::hover());
                let field_w = if kind == Kind::Servers { 220.0 } else { 320.0 };
                let mut name = self.dir_name.clone();
                let response = input(
                    &mut ui.child_ui(
                        Rect::from_min_size(Pos2::new(row.min.x + 28.0, row.min.y), Vec2::new(field_w, 36.0)),
                        egui::Layout::top_down(egui::Align::Min),
                        None,
                    ),
                    &mut name,
                    field_w - 10.0,
                    match kind {
                        Kind::Servers => "Server name",
                        Kind::ResourcePacks => "Resource pack name",
                        Kind::ShaderPacks => "Shader pack name",
                        Kind::Mods => "Mod name",
                    },
                );
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut address = self.dir_address.clone();
                let mut addr_response = None;
                let mut x = row.min.x + 28.0 + field_w + 8.0;
                if kind == Kind::Servers {
                    addr_response = Some(input(
                        &mut ui.child_ui(
                            Rect::from_min_size(Pos2::new(x, row.min.y), Vec2::new(240.0, 36.0)),
                            egui::Layout::top_down(egui::Align::Min),
                            None,
                        ),
                        &mut address,
                        230.0,
                        "Address",
                    ));
                    x += 248.0;
                }
                if name != self.dir_name {
                    self.dir_name = name;
                }
                if address != self.dir_address {
                    self.dir_address = address;
                }
                let mut lookup: Option<(String, Kind)> = None;
                let add_label = "ADD";
                let w = text_width(ui.ctx(), add_label, &bold(12.0)) + 32.0;
                let add_rect = Rect::from_min_size(Pos2::new(x, row.min.y + 1.0), Vec2::new(w, 34.0));
                let pressed = button(ui, ui.id().with("dir-add"), add_rect, &Button::primary(add_label).font(bold(12.0))).clicked()
                    || (enter && (response.has_focus() || addr_response.map_or(false, |r| r.has_focus())));
                if pressed {
                    let name = self.dir_name.trim().to_string();
                    if !name.is_empty() {
                        let instance = self.state.current_instance_mut();
                        match kind {
                            Kind::Servers => {
                                let address = self.dir_address.trim().to_string();
                                if !address.is_empty() {
                                    instance.servers.push(crate::data::Server {
                                        name: name.clone(),
                                        address,
                                        enabled: true,
                                    });
                                    self.dir_name.clear();
                                    self.dir_address.clear();
                                    self.state.flash(format!("{name} added."));
                                }
                            }
                            other => {
                                let known = instance
                                    .entries(other)
                                    .iter()
                                    .any(|e| e.name.to_lowercase() == name.to_lowercase());
                                if known {
                                    self.state
                                        .flash(format!("{name} is already in this instance."));
                                } else if let Some(list) = instance.entries_mut(other) {
                                    list.push(crate::data::Entry::new(name.clone()));
                                    self.dir_name.clear();
                                    self.state.flash(format!("{name} added. Looking it up…"));
                                    lookup = Some((name.clone(), other));
                                }
                            }
                        }
                        self.state.save();
                    }
                }

                if let Some((name, kind)) = lookup {
                    self.look_up(&name, kind);
                    self.deps_kind = kind;
                }

                if let Some(text) = self.state.flash_text().map(|s| s.to_string()) {
                    ui.add_space(12.0);
                    note_colour(ui, &text, Color32::from_rgb(0xff, 0xb4, 0xb8));
                }

                // rows
                ui.add_space(14.0);
                struct Row {
                    name: String,
                    meta: Option<String>,
                    enabled: bool,
                    slug: Option<String>,
                    description: String,
                    icon: Option<String>,
                    auto: bool,
                }
                let rows: Vec<Row> = match kind {
                    Kind::Servers => instance
                        .servers
                        .iter()
                        .map(|s| Row {
                            name: s.name.clone(),
                            meta: Some(s.address.clone()),
                            enabled: s.enabled,
                            slug: None,
                            description: String::new(),
                            icon: None,
                            auto: false,
                        })
                        .collect(),
                    other => instance
                        .entries(other)
                        .iter()
                        .map(|e| Row {
                            name: e.name.clone(),
                            meta: e.version.clone(),
                            enabled: e.enabled,
                            slug: e.slug.clone(),
                            description: e.summary(),
                            icon: e.icon.clone(),
                            auto: e.source == "dependency",
                        })
                        .collect(),
                };
                let tall = kind != Kind::Servers;
                let row_h = if tall { 62.0 } else { 44.0 };
                // Items from an older save have a slug but no text yet: top them up.
                if tall {
                    if let Some(row) = rows
                        .iter()
                        .find(|row| row.slug.is_some() && row.description.is_empty())
                    {
                        if let Some(slug) = row.slug.clone() {
                            self.fill_in(&row.name, &slug);
                        }
                    }
                }
                if rows.is_empty() {
                    note(ui, kind.empty());
                } else {
                    for (index, row) in rows.iter().enumerate() {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width() - 56.0, row_h),
                            Sense::hover(),
                        );
                        let item = Rect::from_min_max(
                            Pos2::new(rect.min.x + 28.0, rect.min.y),
                            Pos2::new(rect.max.x - 28.0, rect.min.y + row_h),
                        );
                        ui.painter().rect_filled(item, CornerRadius::same(RADIUS), CARD);
                        let clipped = ui.painter().with_clip_rect(item);

                        let sw = Rect::from_min_size(
                            Pos2::new(item.min.x + 12.0, item.center().y - 9.0),
                            Vec2::new(34.0, 18.0),
                        );
                        if switch(ui, ui.id().with("dir-sw").with(index), sw, row.enabled) {
                            self.toggle_entry(kind, index);
                        }

                        let mut text_x = item.min.x + 60.0;
                        if tall {
                            let icon_rect = Rect::from_min_size(
                                Pos2::new(item.min.x + 56.0, item.center().y - 20.0),
                                Vec2::new(40.0, 40.0),
                            );
                            let texture = match row.icon.clone() {
                                Some(url) => match self.icons.get(&url).cloned() {
                                    Some(pixels) => self.assets.store(&ctx, &url, &pixels),
                                    None => {
                                        self.icons.want(&url);
                                        None
                                    }
                                },
                                None => None,
                            };
                            let texture =
                                texture.or_else(|| self.assets.id(&ctx, "brand/mark.png"));
                            if let Some(texture) = texture {
                                clipped.add(
                                    egui::epaint::RectShape::filled(
                                        icon_rect,
                                        CornerRadius::same(RADIUS),
                                        Color32::WHITE,
                                    )
                                    .with_texture(texture, uv()),
                                );
                            } else {
                                clipped.rect_filled(
                                    icon_rect,
                                    CornerRadius::same(RADIUS),
                                    Color32::from_rgb(0x11, 0x11, 0x11),
                                );
                            }
                            text_x = icon_rect.max.x + 12.0;
                        }

                        let label = match (kind, &row.meta) {
                            (Kind::Servers, Some(address)) => format!("{} · {}", row.name, address),
                            (_, Some(version)) => format!("{} · {}", row.name, version),
                            _ => row.name.clone(),
                        };
                        let colour = if row.enabled { TEXT } else { FAINT };
                        let line = if tall && !row.description.is_empty() {
                            item.min.y + 17.0
                        } else {
                            item.center().y
                        };
                        ui.painter().text(
                            Pos2::new(text_x, line),
                            Align2::LEFT_CENTER,
                            label.clone(),
                            regular(13.0),
                            colour,
                        );
                        if row.auto {
                            let w = text_width(ui.ctx(), &label, &regular(13.0));
                            ui.painter().text(
                                Pos2::new(text_x + w + 10.0, line),
                                Align2::LEFT_CENTER,
                                "auto-installed",
                                regular(10.0),
                                DIM,
                            );
                        } else if !tall {
                            ui.painter().text(
                                Pos2::new(text_x, line + 13.0),
                                Align2::LEFT_CENTER,
                                if row.enabled { "Enabled" } else { "Disabled" },
                                regular(10.0),
                                if row.enabled { DIM } else { FAINT },
                            );
                        }
                        if tall && !row.description.is_empty() {
                            let body = if row.description.chars().count() > 96 {
                                format!(
                                    "{}…",
                                    row.description.chars().take(96).collect::<String>()
                                )
                            } else {
                                row.description.clone()
                            };
                            let wrap = (item.max.x - 240.0 - text_x).max(80.0);
                            let galley = ctx
                                .fonts(|f| f.layout(body, regular(12.0), MUTED, wrap));
                            clipped.galley(Pos2::new(text_x, item.min.y + 34.0), galley, MUTED);
                        }

                        let mut right = item.max.x - 12.0;
                        let del_w = text_width(ui.ctx(), "DELETE", &bold(11.0)) + 20.0;
                        right -= del_w;
                        let del = Rect::from_min_size(
                            Pos2::new(right, item.center().y - 13.0),
                            Vec2::new(del_w, 26.0),
                        );
                        if button(
                            ui,
                            ui.id().with("dir-del").with(index),
                            del,
                            &Button::ghost("DELETE").font(bold(10.0)),
                        )
                        .clicked()
                        {
                            self.remove_entry(kind, index);
                        }
                        if kind != Kind::Servers {
                            let up_w = text_width(ui.ctx(), "UPDATE", &bold(11.0)) + 20.0;
                            right -= up_w + 8.0;
                            let up = Rect::from_min_size(
                                Pos2::new(right, item.center().y - 13.0),
                                Vec2::new(up_w, 26.0),
                            );
                            let busy = self.updates.busy;
                            if button(
                                ui,
                                ui.id().with("dir-up").with(index),
                                up,
                                &Button::ghost("UPDATE").font(bold(10.0)).on(busy),
                            )
                            .clicked()
                            {
                                match &row.slug {
                                    Some(value) if !busy => {
                                        self.updates.check(
                                            &row.name,
                                            value,
                                            &instance.loader,
                                            &instance.version,
                                        );
                                        self.state.flash(format!(
                                            "Looking for a newer {}…",
                                            row.name
                                        ));
                                    }
                                    Some(_) => {}
                                    None => self.state.flash(format!(
                                        "{} was added by hand, so there is nothing to check.",
                                        row.name
                                    )),
                                }
                            }
                        }
                        ui.add_space(8.0);
                    }
                }

                ui.add_space(28.0);
            });
    }

    fn toggle_entry(&mut self, kind: Kind, index: usize) {
        let id = self.state.current;
        let instance = self.state.instance_mut(id);
        match kind {
            Kind::Servers => {
                if let Some(server) = instance.servers.get_mut(index) {
                    server.enabled = !server.enabled;
                }
            }
            other => {
                if let Some(list) = instance.entries_mut(other) {
                    if let Some(entry) = list.get_mut(index) {
                        entry.enabled = !entry.enabled;
                    }
                }
            }
        }
        self.state.save();
    }

    fn remove_entry(&mut self, kind: Kind, index: usize) {
        let id = self.state.current;
        let name = match kind {
            Kind::Servers => self
                .state
                .instance(id)
                .and_then(|i| i.servers.get(index))
                .map(|s| s.name.clone()),
            other => self
                .state
                .instance(id)
                .and_then(|i| i.entries(other).get(index))
                .map(|e| e.name.clone()),
        };
        let instance = self.state.instance_mut(id);
        match kind {
            Kind::Servers => {
                instance.servers.remove(index);
            }
            other => {
                if let Some(list) = instance.entries_mut(other) {
                    list.remove(index);
                }
            }
        }
        if let Some(name) = name {
            self.state.flash(format!("{name} removed."));
        }
        self.state.save();
    }

    /* -------------------------------------------------------------- settings */

    pub fn page_settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.add_space(28.0);
                let width = ui.available_width().min(640.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::hover());
                ui.painter().text(
                    Pos2::new(rect.min.x + 36.0, rect.min.y),
                    Align2::LEFT_TOP,
                    "Settings",
                    bold(28.0),
                    TEXT,
                );
                ui.add_space(8.0);

                // memory
                let picked = self.settings_row(ui, width, "Memory", |ui, app| {
                    let mut picked = None;
                    for gb in [2u32, 4, 6, 8] {
                        let label = format!("{gb} GB");
                        let w = text_width(ui.ctx(), &label, &medium(13.0)) + 24.0;
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
                        let style = Button::filled(&label).on(app.state.settings.memory == gb);
                        if button(ui, ui.id().with("mem").with(gb), rect, &style).clicked() {
                            picked = Some(gb);
                        }
                    }
                    picked
                });
                if let Some(gb) = picked {
                    self.state.settings.memory = gb;
                    self.state.save();
                }

                self.settings_row(ui, width, "Game directory", |ui, app| {
                    path_chip(ui, &app.state.settings.directory.clone());
                    None::<u32>
                });
                self.settings_row(ui, width, "Language", |ui, _app| {
                    path_chip(ui, "English");
                    None::<u32>
                });

                let picked = self.settings_row(ui, width, "Skin source", |ui, app| {
                    let mut picked = None;
                    for source in ["local", "microsoft", "tlauncher"] {
                        let label = match source {
                            "local" => "Local",
                            "microsoft" => "Microsoft",
                            _ => "TLauncher",
                        };
                        let w = text_width(ui.ctx(), label, &medium(13.0)) + 24.0;
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
                        let style =
                            Button::filled(label).on(app.state.settings.skin == source);
                        if button(ui, ui.id().with("skin").with(source), rect, &style).clicked() {
                            picked = Some(source.to_string());
                        }
                    }
                    picked
                });
                if let Some(source) = picked {
                    self.state.settings.skin = source;
                    self.state.settings.account = None;
                    self.state.save();
                }

                let connect = self.settings_row(ui, width, "Account", |ui, app| {
                    let label = match app.state.settings.account.clone() {
                        Some(who) => format!("Connected as {who}"),
                        None => match app.state.settings.skin.as_str() {
                            "microsoft" => "Microsoft account".to_string(),
                            "tlauncher" => "TLauncher account".to_string(),
                            _ => "Local skin file".to_string(),
                        },
                    };
                    path_chip(ui, &label);
                    let connected = app.state.settings.account.is_some();
                    let local = app.state.settings.skin == "local";
                    let text = if connected { "DISCONNECT" } else { "CONNECT" };
                    let w = text_width(ui.ctx(), text, &bold(10.0)) + 20.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::hover());
                    let style = Button::ghost(text).font(bold(10.0));
                    if !local
                        && button(ui, ui.id().with("skin-connect"), rect, &style).clicked()
                    {
                        return Some(true);
                    }
                    None
                });
                if let Some(true) = connect {
                    if self.state.settings.account.is_some() {
                        self.state.settings.account = None;
                    } else {
                        self.state.settings.account = Some(if self.state.settings.skin == "microsoft" {
                            "Obsifox".into()
                        } else {
                            "Obsifox_TL".into()
                        });
                    }
                    self.state.save();
                }

                let clicked = self.settings_row(ui, width, "CurseForge API key", |ui, app| {
                    let label = if app.state.settings.cf_key.is_empty() {
                        "Not set".to_string()
                    } else {
                        "Key saved".to_string()
                    };
                    path_chip(ui, &label);
                    let w = text_width(ui.ctx(), "SET KEY", &bold(10.0)) + 20.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::hover());
                    let style = Button::ghost("SET KEY").font(bold(10.0));
                    if button(ui, ui.id().with("cf-set"), rect, &style).clicked() {
                        return Some(true);
                    }
                    None
                });
                if let Some(true) = clicked {
                    self.open_cf_key();
                }

                let toggled = self.settings_row(ui, width, "Close launcher when the game starts", |ui, app| {
                    let on = app.state.settings.close_on_play;
                    let w = 52.0f32.max(text_width(ui.ctx(), if on { "On" } else { "Off" }, &medium(13.0)) + 20.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
                    let style = Button::filled(if on { "On" } else { "Off" }).on(on);
                    if button(ui, ui.id().with("close-on-play"), rect, &style).clicked() {
                        return Some(true);
                    }
                    None
                });
                if let Some(true) = toggled {
                    self.state.settings.close_on_play = !self.state.settings.close_on_play;
                    self.state.save();
                }
                ui.add_space(28.0);
            });
    }

    fn settings_row<T>(
        &mut self,
        ui: &mut Ui,
        width: f32,
        title: &str,
        body: impl FnOnce(&mut Ui, &mut App) -> Option<T>,
    ) -> Option<T> {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 52.0), Sense::hover());
        let inner = Rect::from_min_max(
            Pos2::new(rect.min.x + 36.0, rect.min.y),
            Pos2::new(rect.min.x + width - 36.0, rect.max.y),
        );
        ui.painter().line_segment(
            [
                Pos2::new(inner.min.x, inner.min.y),
                Pos2::new(inner.max.x, inner.min.y),
            ],
            Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2a, 0x30)),
        );
        ui.painter().text(
            Pos2::new(inner.min.x, inner.center().y),
            Align2::LEFT_CENTER,
            title,
            regular(14.0),
            Color32::from_rgb(0xc5, 0xc6, 0xcc),
        );
        let right = Rect::from_min_max(
            Pos2::new(inner.center().x, inner.min.y),
            inner.max,
        );
        let mut child = ui.child_ui(
            right,
            egui::Layout::right_to_left(egui::Align::Center),
            None,
        );
        child.set_min_size(right.size());
        body(&mut child, self)
    }
}

fn path_chip(ui: &mut Ui, value: &str) {
    let w = (text_width(ui.ctx(), value, &regular(13.0)) + 24.0).max(220.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(RADIUS_SM), FIELD);
    ui.painter().text(
        Pos2::new(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        value,
        regular(13.0),
        Color32::from_rgb(0xb7, 0xb8, 0xbe),
    );
}

fn note_colour(ui: &mut Ui, value: &str, colour: Color32) {
    let ctx = ui.ctx().clone();
    let galley = ctx.fonts(|f| f.layout(value.to_string(), regular(13.0), colour, ui.available_width()));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter().galley(rect.min, galley, colour);
}

pub fn comma(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

impl App {
    pub fn start_search(&mut self) {
        let loader = self.state.current_instance().loader.clone();
        let kind = self.search.kind.clone();
        let query = self.search.query.clone();
        let key = self.state.settings.cf_key.clone();
        self.search.start(&kind, &query, &loader, &key);
    }
}

/// Keeps the asset cache warm and the fonts ready.
pub fn warm(assets: &mut Assets, state: &State, ctx: &Context) {
    let version = state.list[state.version(&state.current_instance().version)].clone();
    let _ = assets.id(ctx, &version.poster());
    let _ = assets.id(ctx, "brand/mark.png");
}

pub fn page_id(page: &Page) -> Id {
    match page {
        Page::Home => Id::new("home"),
        Page::Instance => Id::new("instance"),
        Page::Directory => Id::new("directory"),
        Page::Mods => Id::new("mods"),
        Page::Settings => Id::new("settings"),
    }
}
