#![cfg_attr(windows, windows_subsystem = "windows")]

mod assets;
mod data;
mod modal;
mod net;
mod pages;
mod shell;
mod theme;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Align2, Context, FontData, FontDefinitions, FontFamily, Id, Pos2, Rect, Vec2};

use assets::Assets;
use data::{Page, State};
use modal::ModalHost;
use theme::*;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const MARK: &[u8] = include_bytes!("../assets/launcher/brand/mark.png");

pub struct App {
    assets: Assets,
    state: State,
    search: net::Search,
    modal: ModalHost,
    page_since: Instant,
    transient: HashMap<String, Instant>,
    /// egui only repaints when something changes, so animations and background
    /// work have to ask for frames until they are done.
    anim_until: Option<Instant>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        Self {
            assets: Assets::new(),
            state: State::new(),
            search: net::Search::default(),
            modal: ModalHost::default(),
            page_since: Instant::now(),
            transient: HashMap::new(),
            anim_until: None,
        }
    }

    fn set_page(&mut self, page: Page) {
        if self.state.page != page {
            self.state.page = page;
            self.page_since = Instant::now();
            self.keep_animating(0.35);
        }
    }

    fn keep_animating(&mut self, secs: f32) {
        let until = Instant::now() + Duration::from_secs_f32(secs);
        self.anim_until = Some(match self.anim_until {
            Some(current) => current.max(until),
            None => until,
        });
    }

    fn transient_set(&mut self, key: &str, secs: f32) {
        self.keep_animating(secs + 0.1);
        self.transient.insert(key.to_string(), Instant::now());
        self.transient.insert(
            format!("{key}:secs"),
            Instant::now() + Duration::from_secs_f32(secs),
        );
    }

    /// Shows `busy` for a moment after an action, then falls back to `idle`.
    fn transient_label(&self, key: &str, idle: &str, busy: &str, secs: f32) -> String {
        let started = self.transient.get(key).copied();
        match started {
            Some(at) if at.elapsed() < Duration::from_secs_f32(secs) => busy.to_string(),
            _ => idle.to_string(),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.063, 0.066, 0.078, 1.0]
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.save();
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.search.poll();

        // Pages fade in, buttons flash their confirmation label and the Modrinth
        // and CurseForge lookups run on a worker thread: all of them need frames
        // to be delivered even when the mouse and keyboard are still.
        let animating = self
            .anim_until
            .map_or(false, |until| Instant::now() < until);
        if self.search.busy || animating {
            ctx.request_repaint();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.modal.close();
        }

        let screen = ctx.screen_rect();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(APP_BG))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                let bar = Rect::from_min_size(full.min, Vec2::new(full.width(), 36.0));
                self.titlebar(ui, bar);
                let body = Rect::from_min_max(
                    Pos2::new(full.min.x, full.min.y + 36.0),
                    full.max,
                );
                let rail = Rect::from_min_size(body.min, Vec2::new(76.0, body.height()));
                self.rail(ui, rail);
                let main = Rect::from_min_max(
                    Pos2::new(body.min.x + 76.0, body.min.y),
                    body.max,
                );
                ui.painter().rect_filled(main, CornerRadius::ZERO, MAIN_BG);

                let fade_in = ((Instant::now() - self.page_since).as_secs_f32() / 0.22).min(1.0);
                let eased = 1.0 - (1.0 - fade_in) * (1.0 - fade_in);
                let salt = match self.state.page {
                    Page::Home => "page-home",
                    Page::Instance => "page-instance",
                    Page::Mods => "page-mods",
                    Page::Settings => "page-settings",
                };
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(main)
                        .layout(egui::Layout::top_down(egui::Align::Min))
                        .id_salt(salt),
                );
                child.set_clip_rect(main);
                child.set_opacity(eased);
                if eased < 1.0 {
                    child.add_space((1.0 - eased) * 8.0);
                    ctx.request_repaint();
                }

                match self.state.page.clone() {
                    Page::Home => self.page_home(&mut child),
                    Page::Instance => self.page_instance(&mut child),
                    Page::Mods => self.page_mods(&mut child),
                    Page::Settings => self.page_settings(&mut child),
                }
            });

        self.draw_modal(ctx);

        // keeps hover and press animations smooth
        if self.modal.kind.is_some() {
            ctx.request_repaint();
        }
        let _ = screen;
        let _ = Align2::LEFT_TOP;
    }
}

fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("inter".into(), Arc::new(FontData::from_static(FONT_REGULAR)));
    fonts
        .font_data
        .insert("inter-md".into(), Arc::new(FontData::from_static(FONT_MEDIUM)));
    fonts
        .font_data
        .insert("inter-sb".into(), Arc::new(FontData::from_static(FONT_SEMIBOLD)));
    fonts
        .families
        .insert(FontFamily::Proportional, vec!["inter".into()]);
    fonts
        .families
        .insert(FontFamily::Monospace, vec!["inter".into()]);
    fonts
        .families
        .insert(FontFamily::Name("medium".into()), vec!["inter-md".into()]);
    fonts
        .families
        .insert(FontFamily::Name("semibold".into()), vec!["inter-sb".into()]);
    ctx.set_fonts(fonts);
}

fn window_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(MARK).ok()?;
    let size = 64u32;
    let image = image.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = image.to_rgba8();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: size,
        height: size,
    })
}

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Rus Launcher")
        .with_inner_size([1360.0, 860.0])
        .with_min_inner_size([1100.0, 680.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_transparent(false)
        .with_taskbar(true);
    let viewport = match window_icon() {
        Some(icon) => viewport.with_icon(icon),
        None => viewport,
    };
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Rus Launcher",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
