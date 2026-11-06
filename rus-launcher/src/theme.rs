pub use egui::CornerRadius;

use egui::{
    epaint::{Mesh, RectShape, Vertex},
    Align2, Color32, Context, FontFamily, FontId, Painter, Pos2, Rect, TextureId, Vec2,
};

pub const APP_BG: Color32 = Color32::from_rgb(0x10, 0x11, 0x14);
pub const TITLEBAR_BG: Color32 = Color32::from_rgb(0x0c, 0x0c, 0x0e);
pub const RAIL_BG: Color32 = Color32::from_rgb(0x0c, 0x0c, 0x0e);
pub const MAIN_BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
pub const CARD: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1f);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x22, 0x22, 0x28);
pub const RED: Color32 = Color32::from_rgb(0xe2, 0x3b, 0x45);
pub const RED_HI: Color32 = Color32::from_rgb(0xff, 0x4d, 0x57);
pub const TEXT: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);
pub const MUTED: Color32 = Color32::from_rgb(0x9b, 0x9c, 0xa3);
pub const DIM: Color32 = Color32::from_rgb(0xd7, 0xd8, 0xde);
pub const FAINT: Color32 = Color32::from_rgb(0x5c, 0x5d, 0x66);
pub const BORDER: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x32);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x23, 0x23, 0x2a);
pub const BORDER_HI: Color32 = Color32::from_rgb(0x34, 0x34, 0x3c);
pub const FIELD: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1f);
pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
pub const VBOX: Color32 = Color32::from_rgb(0x10, 0x10, 0x13);
pub fn scrim() -> Color32 {
    Color32::from_rgba_unmultiplied(44, 29, 91, 247)
}
pub const TAG_TEXT: Color32 = Color32::from_rgb(0xbc, 0xb8, 0xcf);
pub const GREEN: Color32 = Color32::from_rgb(0x7f, 0xd1, 0x8d);
pub const GREEN_LINE: Color32 = Color32::from_rgb(0x2f, 0x6b, 0x45);
pub fn mask() -> Color32 {
    Color32::from_rgba_unmultiplied(5, 5, 7, 189)
}

pub const RADIUS: u8 = 4;
pub const RADIUS_SM: u8 = 2;
pub const RADIUS_PIN: u8 = 10;

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        ch(a.r(), b.r()),
        ch(a.g(), b.g()),
        ch(a.b(), b.b()),
        ch(a.a(), b.a()),
    )
}

pub fn fade(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * alpha.clamp(0.0, 1.0)).round() as u8,
    )
}

/// Vertical gradient.
pub fn fill_v(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: egui::epaint::WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: egui::epaint::WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: bottom,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: bottom,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));
}

/// Horizontal gradient.
pub fn fill_h(painter: &Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: egui::epaint::WHITE_UV,
        color: left,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: egui::epaint::WHITE_UV,
        color: right,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: right,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: left,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));
}

/// An image fitted like `object-fit: cover`, lined up like `object-position: center 40%`.
pub fn cover(rect: Rect, aspect: f32) -> Rect {
    if aspect <= 0.0 {
        return rect;
    }
    let box_aspect = rect.width() / rect.height().max(1.0);
    let size = if box_aspect > aspect {
        Vec2::new(rect.width(), rect.width() / aspect)
    } else {
        Vec2::new(rect.height() * aspect, rect.height())
    };
    let extra = (size.y - rect.height()).max(0.0);
    // Matches the web build's `object-fit: cover; object-position: center 40%`:
    // 40% of the overflow sits above the box.
    let shift = extra * 0.4;
    Rect::from_min_size(
        Pos2::new(rect.center().x - size.x * 0.5, rect.top() - shift),
        size,
    )
}

pub fn image(painter: &Painter, rect: Rect, id: TextureId, radius: u8) {
    painter.add(RectShape::filled(rect, CornerRadius::same(radius), Color32::WHITE).with_texture(id, uv()));
}

/// An image that fills `rect` like a cover, clipped to it.
pub fn cover_image(painter: &Painter, rect: Rect, id: TextureId, aspect: f32, radius: u8) {
    let clipped = painter.with_clip_rect(rect);
    clipped.add(
        RectShape::filled(cover(rect, aspect), CornerRadius::same(radius), Color32::WHITE)
            .with_texture(id, uv()),
    );
}

pub fn text(
    painter: &Painter,
    pos: Pos2,
    value: &str,
    font: FontId,
    color: Color32,
    anchor: Align2,
) {
    painter.text(pos, anchor, value, font, color);
}

/// Text with a letter-spaced look, drawn the way the web version sets `letter-spacing`.
pub fn tracked(
    painter: &Painter,
    pos: Pos2,
    value: &str,
    font: FontId,
    color: Color32,
    anchor: Align2,
    spacing: f32,
) {
    if spacing <= 0.0 {
        painter.text(pos, anchor, value, font, color);
        return;
    }
    let ctx = painter.ctx();
    let width: f32 = value
        .chars()
        .map(|c| {
            ctx.fonts(|f| {
                let mut s = String::new();
                s.push(c);
                f.layout_no_wrap(s, font.clone(), color).size().x
            })
        })
        .sum();
    let total = width + spacing * value.chars().count().saturating_sub(1) as f32;
    let (mut x, y) = match anchor {
        Align2::LEFT_TOP | Align2::LEFT_CENTER | Align2::LEFT_BOTTOM => (pos.x, pos.y),
        Align2::RIGHT_TOP | Align2::RIGHT_CENTER | Align2::RIGHT_BOTTOM => (pos.x - total, pos.y),
        _ => (pos.x - total * 0.5, pos.y),
    };
    let vanchor = match anchor {
        Align2::LEFT_TOP | Align2::CENTER_TOP | Align2::RIGHT_TOP => Align2::LEFT_TOP,
        Align2::LEFT_BOTTOM | Align2::CENTER_BOTTOM | Align2::RIGHT_BOTTOM => Align2::LEFT_BOTTOM,
        _ => Align2::LEFT_CENTER,
    };
    for c in value.chars() {
        let mut s = String::new();
        s.push(c);
        let mut glyph = String::new();
        glyph.push(c);
        painter.text(Pos2::new(x, y), vanchor, glyph, font.clone(), color);
        x += ctx.fonts(|f| f.layout_no_wrap(s.clone(), font.clone(), color).size().x) + spacing;
    }
}

/// Height of one line of text.
pub fn line_height(ctx: &Context, font: &FontId) -> f32 {
    ctx.fonts(|f| f.row_height(font))
}

pub struct Button {
    pub label: String,
    pub fill: Color32,
    pub fill_hover: Color32,
    pub text: Color32,
    pub text_hover: Color32,
    pub border: Color32,
    pub border_hover: Color32,
    pub font: FontId,
    pub radius: u8,
    pub spacing: f32,
}

impl Button {
    pub fn ghost(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            fill: Color32::TRANSPARENT,
            fill_hover: Color32::TRANSPARENT,
            text: DIM,
            text_hover: TEXT,
            border: BORDER_HI,
            border_hover: RED,
            font: medium(10.0),
            radius: RADIUS_SM,
            spacing: 0.6,
        }
    }
    pub fn filled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            fill: CARD,
            fill_hover: CARD_HOVER,
            text: TEXT,
            text_hover: TEXT,
            border: BORDER,
            border_hover: BORDER,
            font: medium(13.0),
            radius: RADIUS_SM,
            spacing: 0.0,
        }
    }
    pub fn primary(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            fill: RED,
            fill_hover: RED_HI,
            text: Color32::WHITE,
            text_hover: Color32::WHITE,
            border: Color32::TRANSPARENT,
            border_hover: Color32::TRANSPARENT,
            font: bold(13.0),
            radius: RADIUS_SM,
            spacing: 0.4,
        }
    }
    /// A chip like the loader or type pickers: a red border when it is picked, never filled.
    pub fn chip(label: impl Into<String>, active: bool) -> Self {
        Self {
            label: label.into(),
            fill: FIELD,
            fill_hover: CARD_HOVER,
            text: if active { TEXT } else { DIM },
            text_hover: TEXT,
            border: if active { RED } else { BORDER },
            border_hover: if active { RED } else { BORDER },
            font: medium(13.0),
            radius: RADIUS_SM,
            spacing: 0.0,
        }
    }
    pub fn on(mut self, on: bool) -> Self {
        if on {
            self.fill = RED;
            self.fill_hover = RED_HI;
            self.text = Color32::WHITE;
            self.border = Color32::TRANSPARENT;
        }
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }
}

/// A painted button with a hover transition. Returns the click.
pub fn button(ui: &mut egui::Ui, id: egui::Id, rect: Rect, style: &Button) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.12);
    let press = ui
        .ctx()
        .animate_bool_with_time(id.with("press"), response.is_pointer_button_down_on(), 0.08);
    let painter = ui.painter();
    let fill = mix(style.fill, style.fill_hover, t);
    let fill = mix(fill, RED_HI, press * 0.25);
    painter.rect_filled(rect, CornerRadius::same(style.radius), fill);
    if style.border != Color32::TRANSPARENT || style.border_hover != Color32::TRANSPARENT {
        painter.rect_stroke(
            rect,
            CornerRadius::same(style.radius),
            egui::Stroke::new(1.0, mix(style.border, style.border_hover, t)), egui::StrokeKind::Inside);
    }
    let color = mix(style.text, style.text_hover, t);
    let mut pos = rect.center();
    pos.y += 0.5;
    if style.spacing > 0.0 {
        tracked(
            painter,
            pos,
            &style.label,
            style.font.clone(),
            color,
            Align2::CENTER_CENTER,
            style.spacing,
        );
    } else {
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            &style.label,
            style.font.clone(),
            color,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
