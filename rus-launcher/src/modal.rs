use egui::{Align2, Color32, Context, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::assets::Assets;
use crate::data::{family_groups, family_label, State, Version};
use crate::theme::*;
use crate::pages::field_label;
use crate::App;

#[derive(Clone)]
pub enum ModalKind {
    New {
        name: String,
        mode: String,
        folder: String,
        loader: String,
        version: String,
        family: String,
    },
    Rename {
        id: u64,
        name: String,
    },
    Edit {
        id: u64,
        name: String,
        custom: String,
    },
    Image {
        id: u64,
        custom: String,
    },
    Servers {
        id: u64,
        name: String,
        address: String,
    },
    CfKey {
        key: String,
    },
}

pub struct ModalHost {
    pub kind: Option<ModalKind>,
    pub open: bool,
    focus: bool,
}

impl Default for ModalHost {
    fn default() -> Self {
        Self {
            kind: None,
            open: false,
            focus: false,
        }
    }
}

impl ModalHost {
    pub fn open(&mut self, kind: ModalKind) {
        self.kind = Some(kind);
        self.open = true;
        self.focus = true;
    }
    pub fn close(&mut self) {
        self.open = false;
    }
}

impl App {
    pub fn open_new_modal(&mut self) {
        let name = format!("Client {}", self.state.instances.len() + 1);
        let loader = self.state.current_instance().loader.clone();
        let version = self.state.current_version().version.to_string();
        let family = crate::data::family_of(&version);
        self.modal.open(ModalKind::New {
            name,
            mode: "new".into(),
            folder: "~/Games/Rus/import".into(),
            loader,
            version,
            family,
        });
    }

    pub fn open_edit(&mut self, id: u64) {
        let (name, custom) = self
            .state
            .instance(id)
            .map(|i| (i.name.clone(), i.image.clone().unwrap_or_default()))
            .unwrap_or_default();
        self.edit_confirm = false;
        self.modal.open(ModalKind::Edit { id, name, custom });
    }

    pub fn open_rename(&mut self, id: u64) {
        let name = self.state.instance(id).map(|i| i.name.clone()).unwrap_or_default();
        self.modal.open(ModalKind::Rename { id, name });
    }

    pub fn open_image(&mut self, id: u64) {
        let custom = self
            .state
            .instance(id)
            .and_then(|i| i.image.clone())
            .unwrap_or_default();
        self.modal.open(ModalKind::Image { id, custom });
    }

    pub fn open_servers(&mut self, id: u64) {
        self.modal.open(ModalKind::Servers {
            id,
            name: String::new(),
            address: String::new(),
        });
    }

    pub fn open_cf_key(&mut self) {
        let key = self.state.settings.cf_key.clone();
        self.modal.open(ModalKind::CfKey { key });
    }

    pub fn draw_modal(&mut self, ctx: &Context) {
        let Some(mut kind) = self.modal.kind.take() else {
            return;
        };
        let t = ctx.animate_bool_with_time(Id::new("modal-fade"), self.modal.open, 0.16);
        if !self.modal.open && t < 0.01 {
            self.modal.kind = None;
            return;
        }

        let screen = ctx.screen_rect();
        let mut close = false;
        let mut created: Option<(String, String, String, String)> = None;
        let mut apply_image: Option<(u64, Option<String>)> = None;
        let mut rename: Option<(u64, String)> = None;
        let mut cf: Option<String> = None;
        let mut edit: Option<(u64, String, String)> = None;
        let mut action: Option<u8> = None;
        let mut confirm = self.edit_confirm;
        let mut state = std::mem::replace(&mut self.state, State::new());

        egui::Area::new(Id::new("modal-area"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (mask, painter) = ui.allocate_painter(screen.size(), Sense::click());
                painter.rect_filled(screen, CornerRadius::ZERO, fade(crate::theme::mask(), t));
                if mask.clicked() {
                    close = true;
                }

                let width = screen.width().min(860.0);
                // the editor carries a name field, the poster grid and the
                // action row, so it needs more height than the other popups
                let height = if matches!(kind, ModalKind::Edit { .. }) {
                    screen.height().min(790.0)
                } else {
                    screen.height().min(680.0)
                };
                let centre = screen.center() + Vec2::new(0.0, (1.0 - t) * 12.0);
                let rect = Rect::from_center_size(centre, Vec2::new(width, height));
                ui.allocate_ui_at_rect(rect, |ui| {
                    ui.set_opacity(t);
                    let painter = ui.painter();
                    painter.rect_filled(rect, CornerRadius::same(RADIUS), PANEL);
                    painter.rect_stroke(rect, CornerRadius::same(RADIUS), Stroke::new(1.0, BORDER_SOFT), egui::StrokeKind::Inside);

                    // header
                    let title = match &kind {
                        ModalKind::New { .. } => "New instance".to_string(),
                        ModalKind::Rename { .. } => "Rename instance".to_string(),
                        ModalKind::Edit { id, .. } => {
                            let name = state.instance(*id).map(|i| i.name.clone()).unwrap_or_default();
                            format!("Edit · {name}")
                        }
                        ModalKind::Image { id, .. } => {
                            let name = state.instance(*id).map(|i| i.name.clone()).unwrap_or_default();
                            format!("Card image · {name}")
                        }
                        ModalKind::Servers { id, .. } => {
                            let name = state.instance(*id).map(|i| i.name.clone()).unwrap_or_default();
                            format!("Servers · {name}")
                        }
                        ModalKind::CfKey { .. } => "CurseForge API key".to_string(),
                    };
                    let header = Rect::from_min_size(rect.min, Vec2::new(width, 51.0));
                    ui.painter().text(
                        Pos2::new(rect.min.x + 20.0, header.center().y),
                        Align2::LEFT_CENTER,
                        title.clone(),
                        bold(17.0),
                        TEXT,
                    );
                    let x = Rect::from_min_size(
                        Pos2::new(rect.max.x - 20.0 - 28.0, header.min.y + 11.0),
                        Vec2::new(28.0, 28.0),
                    );
                    let hovered = ui
                        .interact(x, ui.id().with("modal-x"), Sense::click())
                        .hovered();
                    if hovered {
                        ui.painter()
                            .rect_filled(x, CornerRadius::same(RADIUS_SM), Color32::from_rgb(0x2a, 0x2a, 0x30));
                    }
                    ui.painter().text(
                        x.center(),
                        Align2::CENTER_CENTER,
                        "✕",
                        regular(15.0),
                        if hovered { TEXT } else { MUTED },
                    );
                    if ui
                        .interact(x, ui.id().with("modal-x-hit"), Sense::click())
                        .clicked()
                    {
                        close = true;
                    }
                    ui.painter().line_segment(
                        [
                            Pos2::new(rect.min.x, header.max.y),
                            Pos2::new(rect.max.x, header.max.y),
                        ],
                        Stroke::new(1.0, BORDER_SOFT),
                    );

                    // footer
                    let footer_h = 63.0;
                    let footer = Rect::from_min_max(
                        Pos2::new(rect.min.x, rect.max.y - footer_h),
                        rect.max,
                    );
                    let buttons: Vec<(&str, bool)> = match &kind {
                        ModalKind::Image { .. } => {
                            vec![("RESET", false), ("CANCEL", false), ("APPLY", true)]
                        }
                        ModalKind::CfKey { .. } => {
                            vec![("CLEAR", false), ("CANCEL", false), ("SAVE", true)]
                        }
                        ModalKind::Servers { .. } => vec![("DONE", true)],
                        ModalKind::Edit { .. } => vec![("CANCEL", false), ("SAVE", true)],
                        _ => vec![("CANCEL", false), ("CREATE", true)],
                    };
                    let picked = footer_buttons(ui, footer, &buttons);

                    // body
                    let body = Rect::from_min_max(
                        Pos2::new(rect.min.x, header.max.y),
                        Pos2::new(rect.max.x, footer.min.y),
                    );
                    ui.allocate_ui_at_rect(body.shrink2(Vec2::new(20.0, 16.0)), |ui| {
                        match &mut kind {
                            ModalKind::New {
                                name,
                                mode,
                                folder,
                                loader,
                                version,
                                family,
                            } => {
                                new_body(
                                    ui,
                                    &mut self.assets,
                                    &state,
                                    name,
                                    mode,
                                    folder,
                                    loader,
                                    version,
                                    family,
                                    self.modal.focus,
                                );
                            }
                            ModalKind::Rename { id, name } => {
                                ui.add_space(6.0);
                                field_label(ui, "NAME");
                                field_input(ui, name, ui.available_width().min(420.0), "Instance name", Id::new("rename-input"), self.modal.focus);
                                rename = Some((*id, name.clone()));
                            }
                            ModalKind::Image { id, custom } => {
                                image_body(ui, &mut self.assets, &state, *id, custom);
                            }
                            ModalKind::Edit {
                                id,
                                name,
                                custom,
                            } => {
                                edit_body(
                                    ui,
                                    &mut self.assets,
                                    &state,
                                    *id,
                                    name,
                                    custom,
                                    &mut action,
                                    &mut confirm,
                                );
                                edit = Some((*id, name.clone(), custom.clone()));
                            }
                            ModalKind::Servers { id, name, address } => {
                                servers_body(ui, &mut state, *id, name, address);
                            }
                            ModalKind::CfKey { key } => {
                                ui.add_space(6.0);
                                note_text(ui, "Paste the key from console.curseforge.com. It is kept for this session only.");
                                ui.add_space(10.0);
                                field_label(ui, "API KEY");
                                field_input(ui, key, ui.available_width().min(420.0), "API key", Id::new("cf-input"), self.modal.focus);
                                cf = Some(key.clone());
                            }
                        }
                    });

                    if let Some(index) = picked {
                        match &kind {
                            ModalKind::Image { id, custom } => match index {
                                0 => apply_image = Some((*id, None)),
                                1 => close = true,
                                _ => {
                                    let value = custom.trim().to_string();
                                    apply_image = Some((
                                        *id,
                                        if value.is_empty() { None } else { Some(value) },
                                    ))
                                }
                            },
                            ModalKind::CfKey { key } => match index {
                                0 => cf = Some(String::new()),
                                1 => close = true,
                                _ => cf = Some(key.clone()),
                            },
                            ModalKind::Servers { .. } => close = true,
                            ModalKind::Edit { .. } => {
                                if index == 0 {
                                    close = true;
                                } else if let Some((id, name, image)) = edit.clone() {
                                    state.rename(id, name.trim().to_string());
                                    let image = image.trim().to_string();
                                    state.instance_mut(id).image = if image.is_empty() {
                                        None
                                    } else {
                                        Some(image)
                                    };
                                    state.save();
                                    close = true;
                                }
                            }
                            ModalKind::Rename { .. } => {
                                if index == 0 {
                                    close = true;
                                } else if let Some((id, value)) = rename.clone() {
                                    state.rename(id, value.trim().to_string());
                                    close = true;
                                }
                            }
                            ModalKind::New {
                                name,
                                mode,
                                loader,
                                version,
                                ..
                            } => {
                                if index == 0 {
                                    close = true;
                                } else {
                                    created = Some((
                                        name.clone(),
                                        mode.clone(),
                                        loader.clone(),
                                        version.clone(),
                                    ));
                                }
                            }
                        }
                    }
                });
            });

        self.modal.focus = false;

        if let Some(slot) = action {
            if edit.is_some() {
                if let Some(id) = match &kind {
                    ModalKind::Edit { id, .. } => Some(*id),
                    _ => None,
                } {
                    match slot {
                        0 => {
                            state.duplicate(id);
                            state.save();
                        }
                        1 => {
                            state.toggle_pin(id);
                            state.save();
                        }
                        _ => {
                            if state.instances.len() < 2 {
                                state.flash("The last instance cannot be deleted.".to_string());
                            } else {
                                let name = state
                                    .instance(id)
                                    .map(|i| i.name.clone())
                                    .unwrap_or_default();
                                state.instances.retain(|i| i.id != id);
                                state.pins.retain(|p| *p != id);
                                if state.current == id {
                                    state.current = state.instances.first().map(|i| i.id).unwrap_or(0);
                                }
                                state.flash(format!("{name} deleted."));
                                state.save();
                            }
                        }
                    }
                    close = true;
                }
            }
        }

        if confirm != self.edit_confirm {
            self.edit_confirm = confirm;
        }

        self.state = state;

        if close {
            self.modal.close();
        }
        if let Some((name, mode, loader, version)) = created {
            let mut name = name.trim().to_string();
            if name.is_empty() {
                name = format!("Client {}", self.state.instances.len() + 1);
            }
            if self.state.instances.iter().any(|i| i.name == name) {
                let mut n = 2;
                let base = name.clone();
                while self.state.instances.iter().any(|i| i.name == name) {
                    name = format!("{base} {n}");
                    n += 1;
                }
            }
            let source = if mode == "import" { "import" } else { "new" };
            let id = self.state.add(&name, &version, &loader, source);
            self.state.current = id;
            self.state
                .flash(format!("{name} created on {loader} {version}."));
            self.state.save();
            self.modal.close();
        }
        if let Some((id, value)) = apply_image {
            let name = self
                .state
                .instance(id)
                .map(|i| i.name.clone())
                .unwrap_or_default();
            self.state.instance_mut(id).image = value.clone();
            self.state.flash(match value {
                Some(path) => format!("{name} uses {path}."),
                None => format!("{name} uses the default art again."),
            });
            self.state.save();
            self.modal.close();
        }
        if let Some(key) = cf {
            self.state.settings.cf_key = key;
            self.state.save();
            self.modal.close();
        }
        if self.modal.open {
            self.modal.kind = Some(kind);
        } else if self.modal.kind.is_none() {
            // closing: keep the kind until the fade finishes
            self.modal.kind = Some(kind);
            if t < 0.02 {
                self.modal.kind = None;
            }
        }
    }
}

fn footer_buttons(ui: &mut Ui, rect: Rect, buttons: &[(&str, bool)]) -> Option<usize> {
    let mut picked = None;
    let mut x = rect.max.x - 20.0;
    for (index, (label, primary)) in buttons.iter().enumerate() {
        let font = bold(12.0);
        let w = crate::pages::text_width(ui.ctx(), label, &font) + 28.0;
        x -= w;
        let btn = Rect::from_min_size(
            Pos2::new(x, rect.min.y + 14.0),
            Vec2::new(w, 34.0),
        );
        x -= 8.0;
        let style = if *primary {
            Button::primary(*label).font(font)
        } else {
            Button::ghost(*label).font(font)
        };
        if button(ui, ui.id().with("footer").with(index), btn, &style).clicked() {
            picked = Some(index);
        }
    }
    let _ = rect;
    picked
}

fn note_text(ui: &mut Ui, value: &str) {
    let ctx = ui.ctx().clone();
    let galley = ctx.fonts(|f| f.layout(value.to_string(), regular(13.0), MUTED, ui.available_width()));
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter().galley(rect.min, galley, MUTED);
}

fn field_input(ui: &mut Ui, value: &mut String, width: f32, hint: &str, id: Id, focus: bool) {
    egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            let response = ui.add_sized(
                Vec2::new(width - 24.0, 36.0),
                egui::TextEdit::singleline(value)
                    .hint_text(egui::RichText::new(hint).color(FAINT).font(regular(13.0)))
                    .font(regular(13.0))
                    .text_color(TEXT)
                    .frame(false)
                    .vertical_align(egui::Align::Center),
            );
            if focus {
                response.request_focus();
                let mut state = egui::TextEdit::load_state(ui.ctx(), response.id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(value.chars().count()),
                )));
                egui::TextEdit::store_state(ui.ctx(), response.id, state);
            }
            let _ = response;
        });
}

fn subtabs(ui: &mut Ui, items: &[(&str, bool)]) -> Option<String> {
    let mut picked = None;
    ui.horizontal(|ui| {
        for (index, (label, active)) in items.iter().enumerate() {
            let font = bold(13.0);
            let w = crate::pages::text_width(ui.ctx(), label, &font) + 28.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
            let fill = if *active { RED } else { CARD };
            let hover = if *active { RED_HI } else { CARD_HOVER };
            let style = Button {
                label: label.to_string(),
                fill,
                fill_hover: hover,
                text: if *active { Color32::WHITE } else { MUTED },
                text_hover: TEXT,
                border: Color32::TRANSPARENT,
                border_hover: Color32::TRANSPARENT,
                font,
                radius: RADIUS_SM,
                spacing: 0.0,
            };
            if button(ui, ui.id().with("subtab").with(index), rect, &style).clicked() {
                picked = Some(label.to_string());
            }
        }
    });
    picked
}

#[allow(clippy::too_many_arguments)]
fn new_body(
    ui: &mut Ui,
    assets: &mut Assets,
    state: &State,
    name: &mut String,
    mode: &mut String,
    folder: &mut String,
    loader: &mut String,
    version: &mut String,
    family: &mut String,
    focus: bool,
) {
    let picked = subtabs(ui, &[("New", mode == "new"), ("Import", mode == "import")]);
    if let Some(value) = picked {
        *mode = if value == "New" { "new".into() } else { "import".into() };
    }
    ui.add_space(14.0);
    field_label(ui, "NAME");
    field_input(ui, name, ui.available_width().min(420.0), "Instance name", Id::new("draft-name"), focus);
    if mode == "import" {
        ui.add_space(14.0);
        field_label(ui, "FOLDER");
        field_input(ui, folder, ui.available_width().min(420.0), "Folder", Id::new("draft-folder"), false);
    }
    ui.add_space(14.0);
    field_label(ui, "LOADER");
    let loaders: Vec<(&str, bool)> = crate::data::LOADERS
        .iter()
        .map(|l| (*l, *l == loader.as_str()))
        .collect();
    let mut picked_loader = None;
    ui.horizontal_wrapped(|ui| {
        for (index, (label, active)) in loaders.iter().enumerate() {
            let font = medium(13.0);
            let w = crate::pages::text_width(ui.ctx(), label, &font) + 24.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::hover());
            let style = Button::chip(*label, *active);
            if button(ui, ui.id().with("draft-loader").with(index), rect, &style).clicked() {
                picked_loader = Some(label.to_string());
            }
        }
    });
    if let Some(value) = picked_loader {
        *loader = value;
    }

    // version box
    ui.add_space(14.0);
    let width = ui.available_width();
    let box_height = (ui.available_height() - 150.0).clamp(180.0, 320.0);
    let (outer, _) = ui.allocate_exact_size(Vec2::new(width, box_height), Sense::hover());
    let inner = Rect::from_min_max(
        outer.min,
        Pos2::new(outer.max.x, outer.max.y),
    );
    let clipped = ui.painter().with_clip_rect(inner);
    clipped.rect_filled(inner, CornerRadius::same(RADIUS), VBOX);
    clipped.rect_stroke(inner, CornerRadius::same(RADIUS), Stroke::new(1.0, BORDER_SOFT), egui::StrokeKind::Inside);

    let mut child = ui.child_ui(inner.shrink(10.0), egui::Layout::top_down(egui::Align::Min), None);
    child.set_clip_rect(child.max_rect());
    egui::ScrollArea::vertical()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut child, |ui| {
            let groups = family_groups(&state.list);
            let columns = 5usize;
            let gap = 10.0;
            let card_w = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
            let rows = (groups.len() + columns - 1) / columns;
            for row in 0..rows {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 104.0), Sense::hover());
                for slot in 0..columns {
                    let index = row * columns + slot;
                    let Some((key, items)) = groups.get(index) else { break };
                    let x = rect.min.x + slot as f32 * (card_w + gap);
                    let card = Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(card_w, 104.0));
                    let active = *key == *family;
                    if family_card(ui, assets, state, card, items, loader, active).clicked() {
                        *family = key.clone();
                    }
                }
                ui.add_space(gap);
            }

            let open = groups
                .iter()
                .find(|(key, _)| *key == *family)
                .cloned()
                .unwrap_or_else(|| groups[0].clone());
            ui.add_space(4.0);
            let label = format!("{} RELEASES", open.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 29.0), Sense::hover());
            tracked(
                ui.painter(),
                Pos2::new(rect.min.x, rect.min.y + 10.0),
                &label,
                bold(11.0),
                MUTED,
                Align2::LEFT_TOP,
                1.76,
            );
            for index in open.1.iter() {
                let item = state.list[*index].clone();
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
                let active = state.list[*index].version == version.as_str();
                pick_row(ui, assets, rect, &item, active, loader);
                if ui
                    .interact(rect, ui.id().with("pick").with(index), Sense::click())
                    .clicked()
                {
                    *version = item.version.to_string();
                }
                ui.add_space(6.0);
            }
        });

    ui.add_space(12.0);
    let installing = state
        .list
        .iter()
        .find(|v| v.version == version.as_str())
        .map(|v| !v.installed)
        .unwrap_or(false);
    let text = format!(
        "Creating {} on {} with {}.{}",
        name,
        loader,
        version,
        if installing {
            " That version downloads on the first launch."
        } else {
            ""
        }
    );
    note_text(ui, &text);
}

fn family_card(
    ui: &mut Ui,
    assets: &mut Assets,
    state: &State,
    rect: Rect,
    items: &[usize],
    loader: &str,
    active: bool,
) -> egui::Response {
    let face = state.list[items[0]].clone();
    let response = ui.interact(rect, ui.id().with("family").with(&face.version), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(ui.id().with("family-hover").with(&face.version), response.hovered(), 0.12);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS), Color32::from_rgb(0x14, 0x14, 0x19));
    let clipped = painter.with_clip_rect(rect);
    let ctx = ui.ctx().clone();
    let poster = face.poster();
    if let Some(texture) = assets.id(&ctx, &poster) {
        let aspect = assets.aspect(&ctx, &poster);
        let art = cover(rect, aspect);
        clipped.add(
            egui::epaint::RectShape::filled(art, CornerRadius::ZERO, fade(Color32::WHITE, 0.9 + 0.1 * t))
                .with_texture(texture, uv()),
        );
    }
    let scrim = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 48.0));
    fill_v(
        &clipped,
        scrim,
        crate::theme::scrim(),
        Color32::from_rgba_unmultiplied(44, 29, 91, 0),
    );
    let icon_path = face.icon();
    let mut x = rect.min.x + 8.0;
    if let Some(texture) = assets.id(&ctx, &icon_path) {
        let icon = Rect::from_min_size(Pos2::new(x, rect.min.y + 9.0), Vec2::new(15.0, 15.0));
        clipped.add(
            egui::epaint::RectShape::filled(icon, CornerRadius::ZERO, Color32::WHITE).with_texture(texture, uv()),
        );
    }
    x += 15.0 + 6.0;
    clipped.text(
        Pos2::new(x, rect.min.y + 9.0),
        Align2::LEFT_TOP,
        family_label(&items.iter().map(|i| state.list[*i].clone()).collect::<Vec<_>>()),
        bold(12.0),
        Color32::WHITE,
    );
    let tag_path = format!("tabs/{}@1x.png", loader.to_lowercase());
    if let Some(texture) = assets.id(&ctx, &tag_path) {
        let tag = Rect::from_min_size(
            Pos2::new(rect.max.x - 8.0 - 46.0, rect.min.y + 10.0),
            Vec2::new(46.0, 14.0),
        );
        clipped.add(
            egui::epaint::RectShape::filled(tag, CornerRadius::same(RADIUS_SM), fade(Color32::WHITE, 0.96))
                .with_texture(texture, uv()),
        );
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(
            1.0,
            if active {
                RED
            } else {
                mix(BORDER, Color32::from_rgb(0x4a, 0x4a, 0x55), t)
            },
        ), egui::StrokeKind::Inside);
    if active {
        painter.rect_stroke(
            rect.shrink(1.0),
            CornerRadius::same(RADIUS),
            Stroke::new(1.0, RED), egui::StrokeKind::Inside);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn pick_row(ui: &mut Ui, assets: &mut Assets, rect: Rect, item: &Version, active: bool, loader: &str) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS), CARD);
    let ctx = ui.ctx().clone();
    let icon = item.icon();
    if let Some(texture) = assets.id(&ctx, &icon) {
        let box_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 8.0, rect.center().y - 13.0),
            Vec2::new(26.0, 26.0),
        );
        painter.add(
            egui::epaint::RectShape::filled(box_rect, CornerRadius::ZERO, Color32::WHITE).with_texture(texture, uv()),
        );
    }
    let mut x = rect.min.x + 8.0 + 26.0 + 10.0;
    let version_text = item.version.to_string();
    painter.text(
        Pos2::new(x, rect.center().y),
        Align2::LEFT_CENTER,
        version_text.clone(),
        bold(13.0),
        TEXT,
    );
    x += crate::pages::text_width(&ctx, &version_text, &bold(13.0)) + 10.0;
    painter.text(
        Pos2::new(x, rect.center().y),
        Align2::LEFT_CENTER,
        item.name,
        regular(12.0),
        MUTED,
    );
    let label = if item.installed {
        "INSTALLED"
    } else {
        "DOWNLOADS ON LAUNCH"
    };
    let w = crate::pages::text_width(&ctx, label, &bold(10.0)) + 16.0;
    let chip = Rect::from_min_size(
        Pos2::new(rect.max.x - 8.0 - w, rect.center().y - 13.0),
        Vec2::new(w, 26.0),
    );
    painter.rect_filled(chip, CornerRadius::same(RADIUS_SM), Color32::TRANSPARENT);
    painter.rect_stroke(
        chip,
        CornerRadius::same(RADIUS_SM),
        Stroke::new(1.0, if item.installed { GREEN_LINE } else { BORDER_HI }), egui::StrokeKind::Inside);
    tracked(
        painter,
        Pos2::new(chip.center().x, chip.center().y),
        label,
        bold(10.0),
        if item.installed { GREEN } else { MUTED },
        Align2::CENTER_CENTER,
        0.6,
    );
    if active {
        painter.rect_stroke(rect, CornerRadius::same(RADIUS), Stroke::new(1.0, RED), egui::StrokeKind::Inside);
    }
    let _ = loader;
}

fn image_body(ui: &mut Ui, assets: &mut Assets, state: &State, id: u64, custom: &mut String) {
    let current = state.instance(id).and_then(|i| i.image.clone());
    note_text(ui, "Pick an update poster, or paste a path or URL to your own PNG.");
    ui.add_space(10.0);
    let width = ui.available_width();
    let columns = 5usize;
    let gap = 8.0;
    let card_w = (width - gap * (columns - 1) as f32) / columns as f32;
    let cell = 56.0;
    let rows = (state.list.len() + columns - 1) / columns;
    for row in 0..rows {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, cell), Sense::hover());
        for slot in 0..columns {
            let index = row * columns + slot;
            let Some(item) = state.list.get(index) else { break };
            let x = rect.min.x + slot as f32 * (card_w + gap);
            let card = Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(card_w, cell));
            let poster = item.poster();
            let ctx = ui.ctx().clone();
            let selected = current.as_deref() == Some(poster.as_str());
            let response = ui.interact(card, ui.id().with("picker").with(index), Sense::click());
            let clipped = ui.painter().with_clip_rect(card);
            if let Some(texture) = assets.id(&ctx, &poster) {
                let aspect = assets.aspect(&ctx, &poster);
                cover_image(&clipped, card, texture, aspect, 3);
            }
            ui.painter().rect_stroke(
                card,
                CornerRadius::same(3),
                Stroke::new(
                    1.0,
                    if selected {
                        RED
                    } else if response.hovered() {
                        Color32::from_rgb(0x4a, 0x4a, 0x55)
                    } else {
                        BORDER
                    },
                ), egui::StrokeKind::Inside);
            if response.clicked() {
                *custom = poster;
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        ui.add_space(gap);
    }
    ui.add_space(12.0);
    field_label(ui, "CUSTOM PNG PATH OR URL");
    field_input(
        ui,
        custom,
        ui.available_width().min(520.0),
        "~/Games/Rus/cover.png",
        Id::new("image-path"),
        false,
    );
}

/// Name, card image and the three things you can do to an instance.
fn edit_body(
    ui: &mut Ui,
    assets: &mut Assets,
    state: &State,
    id: u64,
    name: &mut String,
    custom: &mut String,
    action: &mut Option<u8>,
    confirm: &mut bool,
) {
    // the poster grid is tall, so the whole editor scrolls
    egui::ScrollArea::vertical()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.add_space(6.0);
            field_label(ui, "NAME");
            field_input(
                ui,
                name,
                ui.available_width().min(420.0),
                "Instance name",
                Id::new("edit-name"),
                false,
            );
            ui.add_space(12.0);
            image_body(ui, assets, state, id, custom);
            ui.add_space(14.0);
            field_label(ui, "ACTIONS");
            ui.add_space(8.0);
            let pinned = state.pins.contains(&id);
            let labels = [
        "DUPLICATE",
        if pinned { "UNPIN" } else { "PIN" },
        if *confirm { "SURE?" } else { "DELETE" },
    ];
            let (row, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
            let mut x = row.min.x;
            for (slot, label) in labels.iter().enumerate() {
                let w = crate::pages::text_width(ui.ctx(), label, &bold(12.0)) + 28.0;
                let rect = Rect::from_min_size(Pos2::new(x, row.min.y), Vec2::new(w, 34.0));
                x += w + 8.0;
                let style = if slot == 2 {
                    Button::danger(*label).font(bold(11.0))
                } else {
                    Button::ghost(*label).font(bold(11.0)).on(slot == 1 && pinned)
                };
                if button(ui, ui.id().with("edit-act").with(slot), rect, &style).clicked() {
                    if slot == 2 && !*confirm {
                        // deleting an instance is the one thing here that cannot be undone
                        *confirm = true;
                    } else {
                        *confirm = false;
                        *action = Some(slot as u8);
                    }
                }
            }
            ui.add_space(10.0);
        });
}

fn servers_body(ui: &mut Ui, state: &mut State, id: u64, name: &mut String, address: &mut String) {
    let servers = state.instance(id).map(|i| i.servers.clone()).unwrap_or_default();
    if servers.is_empty() {
        note_text(ui, "No servers yet.");
    } else {
        for (index, server) in servers.iter().enumerate() {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::hover());
            let row = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.max.x, rect.min.y + 42.0),
            );
            ui.painter().rect_filled(row, CornerRadius::same(RADIUS), CARD);
            ui.painter().text(
                Pos2::new(row.min.x + 12.0, row.center().y),
                Align2::LEFT_CENTER,
                &server.name,
                regular(13.0),
                TEXT,
            );
            let name_w = crate::pages::text_width(ui.ctx(), &server.name, &regular(13.0));
            ui.painter().text(
                Pos2::new(row.min.x + 12.0 + name_w + 10.0, row.center().y),
                Align2::LEFT_CENTER,
                &server.address,
                regular(13.0),
                MUTED,
            );
            let w = crate::pages::text_width(ui.ctx(), "REMOVE", &bold(10.0)) + 20.0;
            let btn = Rect::from_min_size(
                Pos2::new(row.max.x - 12.0 - w, row.center().y - 13.0),
                Vec2::new(w, 26.0),
            );
            let style = Button::ghost("REMOVE").font(bold(10.0));
            if button(ui, ui.id().with("srv-remove").with(index), btn, &style).clicked() {
                state.instance_mut(id).servers.remove(index);
                state.save();
                return;
            }
            ui.add_space(8.0);
        }
    }
    ui.add_space(12.0);
    field_label(ui, "ADD A SERVER");
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::hover());
    let half = (rect.width() - 8.0 - 90.0) / 2.0;
    let name_rect = Rect::from_min_size(rect.min, Vec2::new(half, 36.0));
    let addr_rect = Rect::from_min_size(Pos2::new(rect.min.x + half + 8.0, rect.min.y), Vec2::new(half, 36.0));
    let add_rect = Rect::from_min_size(
        Pos2::new(addr_rect.max.x + 8.0, rect.min.y),
        Vec2::new(82.0, 36.0),
    );
    let mut row = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
    field_input(&mut row, name, half, "Name", Id::new("srv-name"), false);
    field_input(&mut row, address, half, "Address", Id::new("srv-addr"), false);
    let add_id = row.id().with("srv-add");
    if button(&mut row, add_id, add_rect, &Button::primary("ADD").font(bold(12.0))).clicked() {
        let entry = crate::data::Server {
            enabled: true,
            name: name.trim().to_string(),
            address: address.trim().to_string(),
        };
        if !entry.name.is_empty() && !entry.address.is_empty() {
            state.instance_mut(id).servers.push(entry);
            state.save();
            name.clear();
            address.clear();
        }
    }
}
