#![cfg_attr(windows, windows_subsystem = "windows")]

mod install;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
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

const SPLASH_SECS: u64 = 3;
const DESIGN: Vec2 = Vec2::new(420.0, 728.0);

enum Phase {
    Splash,
    Work,
}

struct GateInstall {
    started: Instant,
    phase: Phase,
    placed: bool,
    splash: TextureHandle,
    wordmark: TextureHandle,
    _mark: TextureHandle,
    log: install::Log,
    done: Arc<AtomicBool>,
    launcher: Arc<Mutex<Option<PathBuf>>>,
    worker: Option<JoinHandle<()>>,
    launched: bool,
    close_at: Option<Instant>,
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
            splash: load_texture(ctx, "splash", SPLASH_PNG),
            wordmark: load_texture(ctx, "wordmark", WORDMARK_PNG),
            _mark: load_texture(ctx, "mark", MARK_PNG),
            log: install::Log::default(),
            done: Arc::new(AtomicBool::new(false)),
            launcher: Arc::new(Mutex::new(None)),
            worker: None,
            launched: false,
            close_at: None,
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

    fn start_work(&mut self) {
        let log = self.log.clone();
        let done = self.done.clone();
        let launcher = self.launcher.clone();
        self.worker = Some(std::thread::spawn(move || {
            let dir = install::ensure_installed(&log);
            let path = install::sync_launcher(&dir, &log);
            *launcher.lock().expect("launcher") = path;
            done.store(true, Ordering::SeqCst);
        }));
    }
}

impl eframe::App for GateInstall {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if matches!(self.phase, Phase::Splash)
            && self.started.elapsed() >= Duration::from_secs(SPLASH_SECS)
        {
            self.phase = Phase::Work;
            self.placed = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Gate Install".into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(DESIGN));
            self.start_work();
        }

        match self.phase {
            Phase::Splash => self.paint_splash(ctx),
            Phase::Work => self.paint_work(ctx),
        }

        if let Some(at) = self.close_at {
            if Instant::now() >= at {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
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
                ui.painter()
                    .image(self.wordmark.id(), rect, unit_uv(), Color32::WHITE);
            });
    }

    fn paint_work(&mut self, ctx: &egui::Context) {
        let finished = self.done.load(Ordering::SeqCst);
        if finished && !self.launched {
            self.launched = true;
            let path = self.launcher.lock().expect("launcher").clone();
            match path {
                Some(path) => {
                    self.log.line("Starting Rus Launcher.");
                    if install::launch(&path) {
                        self.close_at = Some(Instant::now() + Duration::from_millis(900));
                    } else {
                        self.log.line("The launcher did not start.");
                        self.close_at = Some(Instant::now() + Duration::from_millis(1800));
                    }
                }
                None => {
                    self.close_at = Some(Instant::now() + Duration::from_millis(1600));
                }
            }
        }

        self.place(ctx, DESIGN);
        let lines = self.log.lines();
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

                // status block where the install button used to be
                let panel = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + 548.0 * s), rect.max);
                ui.painter().rect_filled(
                    panel,
                    CornerRadius::ZERO,
                    Color32::from_rgba_unmultiplied(10, 12, 18, 235),
                );

                let headline = if finished {
                    "Ready"
                } else {
                    "Installing"
                };
                ui.painter().text(
                    Pos2::new(rect.center().x, rect.top() + 574.0 * s),
                    Align2::CENTER_CENTER,
                    headline,
                    FontId::new(15.0 * s, FontFamily::Name("semibold".into())),
                    Color32::WHITE,
                );

                // indeterminate progress bar
                let bar = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.top() + 602.0 * s),
                    Vec2::new(300.0 * s, 6.0 * s),
                );
                ui.painter()
                    .rect_filled(bar, CornerRadius::same(3), Color32::from_rgb(38, 42, 52));
                if !finished {
                    let span = bar.width();
                    let head = (self.started.elapsed().as_millis() as f32 / 9.0) % (span * 1.6);
                    let start = head - span * 0.3;
                    let from = (bar.left() + start).max(bar.left());
                    let to = (bar.left() + start + span * 0.3).min(bar.right());
                    if to > from {
                        ui.painter().rect_filled(
                            Rect::from_min_max(Pos2::new(from, bar.top()), Pos2::new(to, bar.bottom())),
                            CornerRadius::same(3),
                            Color32::from_rgb(46, 188, 190),
                        );
                    }
                } else {
                    ui.painter().rect_filled(
                        bar,
                        CornerRadius::same(3),
                        Color32::from_rgb(46, 188, 190),
                    );
                }

                let shown: Vec<String> = lines.iter().rev().take(3).cloned().collect();
                for (i, line) in shown.iter().enumerate() {
                    ui.painter().text(
                        Pos2::new(rect.center().x, rect.top() + (634.0 + i as f32 * 20.0) * s),
                        Align2::CENTER_CENTER,
                        line.clone(),
                        FontId::new(12.0 * s, FontFamily::Proportional),
                        Color32::from_rgb(186, 190, 198),
                    );
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

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "/uninstall") {
        let dir = install::app_dir();
        let _ = std::fs::remove_dir_all(&dir);
        return Ok(());
    }

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
