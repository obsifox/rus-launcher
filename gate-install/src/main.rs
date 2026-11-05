use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect,
    Sense, Stroke, TextureHandle, Vec2,
};

const SPLASH_PNG: &[u8] = include_bytes!("../assets/splash.png");
const WORDMARK_PNG: &[u8] = include_bytes!("../assets/wordmark.png");
const MARK_PNG: &[u8] = include_bytes!("../assets/mark.png");
const FONT_REGULAR: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");

const SPLASH_SECS: u64 = 6;
const DESIGN: Vec2 = Vec2::new(420.0, 728.0);

enum Phase {
    Splash,
    Launcher,
}

struct GateInstall {
    started: Instant,
    phase: Phase,
    placed: bool,
    advanced: bool,
    splash: TextureHandle,
    wordmark: TextureHandle,
    mark: TextureHandle,
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let decoded = image::load_from_memory(bytes).expect(name).to_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(FONT_REGULAR)),
    );
    fonts.font_data.insert(
        "inter-md".into(),
        Arc::new(FontData::from_static(FONT_MEDIUM)),
    );
    fonts.font_data.insert(
        "inter-sb".into(),
        Arc::new(FontData::from_static(FONT_SEMIBOLD)),
    );
    fonts
        .families
        .insert(FontFamily::Proportional, vec!["inter".into()]);
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec!["inter-md".into()],
    );
    fonts.families.insert(
        FontFamily::Name("semibold".into()),
        vec!["inter-sb".into()],
    );
    ctx.set_fonts(fonts);
}

impl GateInstall {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let ctx = &cc.egui_ctx;
        Self {
            started: Instant::now(),
            phase: Phase::Splash,
            placed: false,
            advanced: false,
            splash: load_texture(ctx, "splash", SPLASH_PNG),
            wordmark: load_texture(ctx, "wordmark", WORDMARK_PNG),
            mark: load_texture(ctx, "mark", MARK_PNG),
        }
    }

    fn place(&mut self, ctx: &egui::Context, size: Vec2) {
        if self.placed {
            return;
        }
        let monitor = ctx.input(|input| input.viewport().monitor_size);
        if let Some(monitor) = monitor {
            let pos = Pos2::new((monitor.x - size.x) * 0.5, (monitor.y - size.y) * 0.5);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            self.placed = true;
        }
    }
}

impl eframe::App for GateInstall {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        match self.phase {
            Phase::Splash => [0.0, 0.0, 0.0, 0.0],
            Phase::Launcher => [0.0, 0.0, 0.0, 0.0],
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.started.elapsed() >= Duration::from_secs(SPLASH_SECS)
            && matches!(self.phase, Phase::Splash)
        {
            self.phase = Phase::Launcher;
            self.placed = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Gate Install".into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(DESIGN));
        }

        match self.phase {
            Phase::Splash => self.paint_splash(ctx),
            Phase::Launcher => self.paint_launcher(ctx),
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl GateInstall {
    fn paint_splash(&mut self, ctx: &egui::Context) {
        let size = Vec2::new(480.0, 148.0);
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        self.place(ctx, size);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().image(
                    self.wordmark.id(),
                    rect,
                    unit_uv(),
                    Color32::WHITE,
                );
            });
    }

    fn paint_launcher(&mut self, ctx: &egui::Context) {
        self.place(ctx, DESIGN);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let s = rect.width() / DESIGN.x;
                let at = |x: f32, y: f32| rect.min + Vec2::new(x * s, y * s);
                {
                    let painter = ui.painter();
                    painter.add(
                        egui::epaint::RectShape::filled(rect, 3, Color32::WHITE)
                            .with_texture(self.splash.id(), unit_uv()),
                    );
                    let logo_h = 52.0 * s;
                    let logo_w = logo_h * self.wordmark.aspect_ratio();
                    painter.image(
                        self.wordmark.id(),
                        Rect::from_center_size(
                            at(DESIGN.x * 0.5, 10.0 + 26.0),
                            Vec2::new(logo_w, logo_h),
                        ),
                        unit_uv(),
                        Color32::WHITE,
                    );
                }

                let drag = ui.interact(
                    Rect::from_min_size(rect.min, Vec2::new(rect.width() - 72.0 * s, 42.0 * s)),
                    ui.id().with("drag"),
                    Sense::drag(),
                );
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                let min_rect = Rect::from_min_size(at(360.0, 8.0), Vec2::splat(26.0 * s));
                if ui
                    .interact(min_rect, ui.id().with("min"), Sense::click())
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                let close = Rect::from_min_size(at(388.0, 8.0), Vec2::splat(26.0 * s));
                if ui
                    .interact(close, ui.id().with("close"), Sense::click())
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                {
                    let painter = ui.painter();
                    let stroke = Stroke::new(1.5 * s, Color32::from_rgb(220, 222, 226));
                    let y = min_rect.center().y;
                    painter.line_segment(
                        [
                            Pos2::new(min_rect.left() + 7.0 * s, y),
                            Pos2::new(min_rect.right() - 7.0 * s, y),
                        ],
                        stroke,
                    );
                    let pad = 7.0 * s;
                    painter.line_segment(
                        [
                            Pos2::new(close.left() + pad, close.top() + pad),
                            Pos2::new(close.right() - pad, close.bottom() - pad),
                        ],
                        stroke,
                    );
                    painter.line_segment(
                        [
                            Pos2::new(close.right() - pad, close.top() + pad),
                            Pos2::new(close.left() + pad, close.bottom() - pad),
                        ],
                        stroke,
                    );
                }

                if self.advanced {
                    let panel =
                        Rect::from_min_max(Pos2::new(rect.left(), rect.top() + 548.0 * s), rect.max);
                    ui.painter().rect_filled(
                        panel,
                        CornerRadius::ZERO,
                        Color32::from_rgba_unmultiplied(10, 12, 18, 235),
                    );
                    ui.painter().text(
                        Pos2::new(rect.center().x, rect.top() + 590.0 * s),
                        Align2::CENTER_CENTER,
                        "Install location",
                        FontId::new(15.0 * s, FontFamily::Name("semibold".into())),
                        Color32::WHITE,
                    );
                    let field = Rect::from_center_size(
                        Pos2::new(rect.center().x, rect.top() + 634.0 * s),
                        Vec2::new(300.0 * s, 36.0 * s),
                    );
                    ui.painter().rect_filled(
                        field,
                        CornerRadius::same(3),
                        Color32::from_rgb(28, 32, 42),
                    );
                    ui.painter().text(
                        field.left_center() + Vec2::new(12.0 * s, 0.0),
                        Align2::LEFT_CENTER,
                        "Choose a folder",
                        FontId::new(13.0 * s, FontFamily::Proportional),
                        Color32::from_rgb(186, 190, 198),
                    );
                    let back = tracked(
                        ui,
                        "BACK",
                        Pos2::new(rect.center().x, rect.top() + 684.0 * s),
                        FontId::new(11.0 * s, FontFamily::Name("medium".into())),
                        Color32::from_rgb(228, 230, 234),
                        1.8 * s,
                    );
                    let hit = ui.interact(
                        back.expand2(Vec2::new(12.0, 10.0)),
                        ui.id().with("back"),
                        Sense::click(),
                    );
                    if hit
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.advanced = false;
                    }
                } else {
                    let button =
                        Rect::from_min_size(at(54.0, 582.0), Vec2::new(312.0 * s, 68.0 * s));
                    let response = ui.interact(button, ui.id().with("install"), Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let fill = if response.hovered() {
                        Color32::from_rgb(70, 208, 210)
                    } else {
                        Color32::from_rgb(46, 188, 190)
                    };
                    ui.painter()
                        .rect_filled(button, CornerRadius::same(3), fill);
                    ui.painter().text(
                        button.center(),
                        Align2::CENTER_CENTER,
                        "Install",
                        FontId::new(22.0 * s, FontFamily::Name("semibold".into())),
                        Color32::from_rgb(10, 16, 28),
                    );
                    let options = tracked(
                        ui,
                        "ADVANCED OPTIONS",
                        Pos2::new(rect.center().x, at(0.0, 668.0).y),
                        FontId::new(11.0 * s, FontFamily::Name("medium".into())),
                        Color32::from_rgb(228, 230, 234),
                        1.8 * s,
                    );
                    let opt = ui.interact(
                        options.expand2(Vec2::new(8.0, 8.0)),
                        ui.id().with("advanced"),
                        Sense::click(),
                    );
                    if opt
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.advanced = true;
                    }
                }

                ui.painter().text(
                    Pos2::new(rect.right() - 14.0 * s, rect.bottom() - 12.0 * s),
                    Align2::RIGHT_BOTTOM,
                    "Gate Install",
                    FontId::new(10.0 * s, FontFamily::Proportional),
                    Color32::from_rgb(148, 154, 166),
                );
            });
    }
}

fn unit_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
}

fn tracked(ui: &egui::Ui, text: &str, center: Pos2, font: FontId, color: Color32, tracking: f32) -> Rect {
    let painter = ui.painter();
    let mut widths = Vec::new();
    for ch in text.chars() {
        let galley = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
        widths.push(galley.size().x);
    }
    let gaps = text.chars().count().saturating_sub(1) as f32;
    let total = widths.iter().sum::<f32>() + tracking * gaps;
    let mut x = center.x - total * 0.5;
    let mut height: f32 = 0.0;
    for (ch, width) in text.chars().zip(widths) {
        let galley = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
        height = height.max(galley.size().y);
        painter.galley(
            Pos2::new(x, center.y - galley.size().y * 0.5),
            galley,
            color,
        );
        x += width + tracking;
    }
    Rect::from_min_size(
        Pos2::new(center.x - total * 0.5, center.y - height * 0.5),
        Vec2::new(total, height),
    )
}

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Gate Install")
        .with_inner_size([480.0, 148.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(true);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Gate Install",
        options,
        Box::new(|cc| Ok(Box::new(GateInstall::new(cc)))),
    )
}
