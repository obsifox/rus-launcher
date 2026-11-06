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
    updates: net::Updates,
    details: net::Details,
    deps: net::Deps,
    icons: net::Icons,
    /// Which drawer the running dependency lookup belongs to.
    deps_kind: crate::data::Kind,
    /// Which drawer of the Directory is open.
    pub drawer: crate::data::Kind,
    /// Text typed into the Directory's add row.
    pub dir_name: String,
    pub dir_address: String,
    pub edit_confirm: bool,
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
            updates: net::Updates::default(),
            details: net::Details::default(),
            deps: net::Deps::default(),
            icons: net::Icons::default(),
            deps_kind: crate::data::Kind::Mods,
            drawer: crate::data::Kind::Mods,
            dir_name: String::new(),
            dir_address: String::new(),
            edit_confirm: false,
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

    /// Asks for the text, art and dependencies of the things that were added.
    pub fn look_up(&mut self, name: &str, kind: crate::data::Kind) {
        self.details.look_up(name, kind.facet());
    }

    /// Fills in the text and art of an item we already know the slug of.
    pub fn fill_in(&mut self, name: &str, slug: &str) {
        self.details.fill(name, slug);
    }

    /// Starts a dependency lookup for `slug`, remembering where it belongs.
    pub fn fetch_deps(&mut self, slug: &str, name: &str, kind: crate::data::Kind) {
        let instance = self.state.current_instance().clone();
        self.deps_kind = kind;
        self.deps
            .fetch(slug, name, &instance.loader, &instance.version);
    }

    /// Collects everything the worker threads came back with.
    fn apply_lookups(&mut self) {
        if self.icons.pump() {
            self.keep_animating(0.05);
        }

        // An item typed in by hand: give it the text and art of the real project.
        if let Some((name, found)) = self.details.poll() {
            let chained = self.details.chained();
            if let Some(project) = found {
                let slug = project.slug.clone();
                let description = project.description.clone();
                let icon = project.icon_url.clone();
                let mut filled = false;
                let id = self.state.current;
                let instance = self.state.instance_mut(id);
                {
                    for list in [
                        &mut instance.mods,
                        &mut instance.resourcepacks,
                        &mut instance.shaderpacks,
                    ] {
                        if let Some(entry) = list.iter_mut().find(|e| e.name == name) {
                            entry.slug = Some(slug.clone());
                            entry.description = Some(description.clone());
                            entry.icon = icon.clone();
                            if entry.source == "local" {
                                entry.source = "modrinth".into();
                            }
                            filled = true;
                            break;
                        }
                    }
                }
                if filled {
                    self.state.save();
                    if chained {
                        let kind = self.deps_kind;
                        self.fetch_deps(&slug, &name, kind);
                    }
                }
            }
        }

        // Dependencies: the required ones install themselves, the rest are offered.
        if let Some((slug, parent, found)) = self.deps.poll() {
            match found {
                Some(deps) => {
                    let kind = self.deps_kind;
                    let id = self.state.current;
                    let mut installed: Vec<String> = Vec::new();
                    let mut optional: Vec<crate::data::Entry> = Vec::new();
                    let instance = self.state.instance_mut(id);
                    if let Some(list) = instance.entries_mut(kind) {
                            for dep in deps {
                                let project = dep.project;
                                let known = list.iter().any(|e| {
                                    e.slug.as_deref() == Some(project.slug.as_str())
                                        || e.name == project.title
                                });
                                if known {
                                    continue;
                                }
                                if dep.required {
                                    list.push(crate::data::Entry::dependency(
                                        &project.title,
                                        &project.slug,
                                        &project.description,
                                        project.icon_url.clone(),
                                    ));
                                    installed.push(project.title.clone());
                                } else {
                                    optional.push(crate::data::Entry::found(
                                        &project.title,
                                        &project.slug,
                                        &project.description,
                                        project.icon_url.clone(),
                                    ));
                                }
                            }
                    }
                    let instance = self.state.instance_mut(id);
                    instance.offers = optional;
                    instance.offers_kind = kind.key().to_string();
                    instance.offers_for = parent.clone();
                    if !installed.is_empty() {
                        let word = if installed.len() == 1 {
                            "dependency"
                        } else {
                            "dependencies"
                        };
                        self.state.flash(format!(
                            "{parent} needs {} {word}: {}.",
                            installed.len(),
                            installed.join(", ")
                        ));
                    } else if let Some(count) = self
                        .state
                        .instance(id)
                        .map(|i| i.offers.len())
                        .filter(|n| *n > 0)
                    {
                        self.state.flash(format!(
                            "{parent} has {count} optional {}.",
                            if count == 1 { "extra" } else { "extras" }
                        ));
                    }
                    let _ = slug;
                    self.state.save();
                }
                None => self.state.flash(format!(
                    "Could not read the dependencies of {parent}."
                )),
            }
        }
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
        self.updates.poll(&mut self.state);
        self.apply_lookups();

        // Pages fade in, buttons flash their confirmation label and the Modrinth
        // and CurseForge lookups run on a worker thread: all of them need frames
        // to be delivered even when the mouse and keyboard are still.
        let animating = self
            .anim_until
            .map_or(false, |until| Instant::now() < until);
        if self.search.busy
            || self.updates.busy
            || self.details.busy
            || self.deps.busy
            || self.icons.waiting()
            || self.state.flash_text().is_some()
            || animating
        {
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
                    Page::Directory => "page-directory",
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
                    Page::Directory => self.page_directory(&mut child),
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
