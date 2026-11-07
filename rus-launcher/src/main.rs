#![cfg_attr(windows, windows_subsystem = "windows")]

mod assets;
mod data;
mod modal;
mod net;
mod pages;
mod skin;
mod updater;
mod shell;
mod theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
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
    skins: net::Skins,
    /// Watching GitHub for a newer build of this launcher.
    upgrade: net::Upgrade,
    pub upgrade_found: Option<net::Release>,
    pub upgrade_note: String,
    /// The skin the player wears, once the chosen source has confirmed it.
    pub skin: Option<net::Skin>,
    pub skin_status: String,
    /// The little Yggdrasil service that carries the skin into the game.
    skin_server: Option<skin::Server>,
    /// authlib-injector, fetched the first time the game needs it.
    injector: Option<PathBuf>,
    injector_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    pub injector_status: String,
    /// Which drawer the running dependency lookup belongs to.
    deps_kind: crate::data::Kind,
    /// True when the running lookup should ask before it installs.
    deps_ask: bool,
    /// Lookups that had to wait because another one was still running.
    dep_queue: Vec<(String, String, crate::data::Kind, bool)>,
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
        let state = State::new();
        let mut app = Self {
            assets: Assets::new(),
            state,
            search: net::Search::default(),
            updates: net::Updates::default(),
            details: net::Details::default(),
            deps: net::Deps::default(),
            icons: net::Icons::default(),
            skins: net::Skins::default(),
            upgrade: net::Upgrade::default(),
            upgrade_found: None,
            upgrade_note: String::new(),
            skin: None,
            skin_status: String::new(),
            skin_server: None,
            injector: None,
            injector_rx: None,
            injector_status: String::new(),
            deps_kind: crate::data::Kind::Mods,
            deps_ask: false,
            dep_queue: Vec::new(),
            drawer: crate::data::Kind::Mods,
            dir_name: String::new(),
            dir_address: String::new(),
            edit_confirm: false,
            modal: ModalHost::default(),
            page_since: Instant::now(),
            transient: HashMap::new(),
            anim_until: None,
        };
        app.restore_skin();
        app.upgrade.check();
        app
    }

    /// A skin saved by an earlier session is only an address, so the picture
    /// comes down again in the background while the launcher opens.
    fn restore_skin(&mut self) {
        if self.state.settings.skin == "local" {
            let path = self.state.settings.skin_local.clone();
            if !path.is_empty() {
                self.use_local_skin(&path);
            }
            return;
        }
        let url = self.state.settings.skin_url.clone();
        if url.is_empty() {
            return;
        }
        let model = self.state.settings.skin_model.clone();
        let name = self
            .state
            .settings
            .account
            .clone()
            .unwrap_or_else(|| "Player".to_string());
        let uuid = if self.state.settings.skin_uuid.is_empty() {
            skin::offline_uuid(&name)
        } else {
            self.state.settings.skin_uuid.clone()
        };
        self.skin_status = "Fetching the saved skin…".to_string();
        self.skins.load(&url, &model, &name, &uuid);
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
    /// `ask` opens a popup on the Mods page so the choice is the player's;
    /// everywhere else the required ones simply install themselves.
    pub fn fetch_deps(&mut self, slug: &str, name: &str, kind: crate::data::Kind, ask: bool) {
        if self.deps.busy {
            if !self
                .dep_queue
                .iter()
                .any(|(queued, _, queued_kind, _)| queued == slug && *queued_kind == kind)
            {
                self.dep_queue
                    .push((slug.to_string(), name.to_string(), kind, ask));
            }
            return;
        }
        self.deps_ask = ask;
        self.deps_kind = kind;
        let instance = self.state.current_instance().clone();
        self.deps
            .fetch(slug, name, &instance.loader, &instance.version);
    }

    /// Installs what the player ticked in the dependency popup. Required
    /// entries are always taken; the rest only when they were ticked.
    pub fn install_deps(&mut self, parent: &str, items: &[crate::modal::DepItem]) {
        let kind = self.deps_kind;
        let mut taken: Vec<String> = Vec::new();
        let mut chained: Vec<(String, String)> = Vec::new();
        {
            let id = self.state.current;
            let instance = self.state.instance_mut(id);
            if let Some(list) = instance.entries_mut(kind) {
                for item in items {
                    if !item.required && !item.take {
                        continue;
                    }
                    if list.iter().any(|e| {
                        e.slug.as_deref() == Some(item.slug.as_str()) || e.name == item.title
                    }) {
                        continue;
                    }
                    let entry = if item.required {
                        crate::data::Entry::dependency(
                            &item.title,
                            &item.slug,
                            &item.description,
                            item.icon.clone(),
                        )
                    } else {
                        crate::data::Entry::found(
                            &item.title,
                            &item.slug,
                            &item.description,
                            item.icon.clone(),
                        )
                    };
                    list.push(entry);
                    taken.push(item.title.clone());
                    if !item.required {
                        chained.push((item.slug.clone(), item.title.clone()));
                    }
                }
            }
        }
        self.state.save();
        if taken.is_empty() {
            self.state
                .flash(format!("{parent} installed on its own."));
        } else {
            let word = if taken.len() == 1 { "with" } else { "with" };
            self.state
                .flash(format!("{parent} installed {word} {}.", taken.join(", ")));
        }
        // Anything the player picked drags its own prerequisites along.
        for (slug, name) in chained {
            self.fetch_deps(&slug, &name, kind, false);
        }
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
                        self.fetch_deps(&slug, &name, kind, false);
                    }
                }
            }
        }

        // Dependencies: the Mods page asks before it installs, everywhere
        // else the required ones simply install themselves.
        if let Some((_slug, parent, found)) = self.deps.poll() {
            let ask = self.deps_ask;
            self.deps_ask = false;
            match found {
                Some(deps) => {
                    let kind = self.deps_kind;
                    let id = self.state.current;
                    let mut items: Vec<crate::modal::DepItem> = Vec::new();
                    {
                        let installed = self.state.instance(id).map(|i| i.entries(kind));
                        for dep in deps {
                            let project = dep.project;
                            let known = installed
                                .map(|list| {
                                    list.iter().any(|e| {
                                        e.slug.as_deref() == Some(project.slug.as_str())
                                            || e.name == project.title
                                    })
                                })
                                .unwrap_or(false);
                            if known {
                                continue;
                            }
                            items.push(crate::modal::DepItem {
                                title: project.title,
                                slug: project.slug,
                                description: project.description,
                                icon: project.icon_url,
                                required: dep.required,
                                take: false,
                            });
                        }
                    }
                    if ask {
                        if items.is_empty() {
                            self.state.flash(format!("{parent} needs nothing else."));
                        } else {
                            self.open_deps(&parent, items);
                        }
                    } else {
                        let required: Vec<crate::modal::DepItem> = items
                            .iter()
                            .filter(|item| item.required)
                            .cloned()
                            .collect();
                        if !required.is_empty() {
                            {
                                let instance = self.state.instance_mut(id);
                                if let Some(list) = instance.entries_mut(kind) {
                                    for item in &required {
                                        list.push(crate::data::Entry::dependency(
                                            &item.title,
                                            &item.slug,
                                            &item.description,
                                            item.icon.clone(),
                                        ));
                                    }
                                }
                            }
                            let names: Vec<String> =
                                required.iter().map(|i| i.title.clone()).collect();
                            let word = if names.len() == 1 {
                                "dependency"
                            } else {
                                "dependencies"
                            };
                            self.state.flash(format!(
                                "{parent} needs {} {word}: {}.",
                                names.len(),
                                names.join(", ")
                            ));
                            self.state.save();
                        }
                    }
                }
                None => self
                    .state
                    .flash(format!("Could not read the dependencies of {parent}.")),
            }
        }

        // Skins: TLauncher either has one for the nickname or it does not.
        if let Some(outcome) = self.skins.poll() {
            match outcome {
                Ok(Some(skin)) => {
                    let model = if skin.model == "slim" {
                        "slim"
                    } else {
                        "classic"
                    };
                    self.state.settings.skin_url = skin.url.clone();
                    self.state.settings.skin_model = model.to_string();
                    let name = self.state.settings.skin_name.trim().to_string();
                    if !name.is_empty() {
                        self.state.settings.account = Some(name.clone());
                    }
                    let where_from = if self.state.settings.skin == "microsoft" {
                        "Minecraft"
                    } else {
                        "TLauncher"
                    };
                    self.skin_status = match self.state.settings.account.clone() {
                        Some(who) => format!("{who} wears a {model} skin from {where_from}."),
                        None => format!("A {model} skin came down from {where_from}."),
                    };
                    self.state.settings.skin_uuid = skin.uuid.clone();
                    let worn = skin::Skin {
                        bytes: skin.bytes.clone(),
                        model: model.to_string(),
                    };
                    self.skin = Some(skin);
                    self.wear_skin(Some(worn));
                    self.state.save();
                }
                Ok(None) => {
                    let who = self.state.settings.skin_name.trim().to_string();
                    self.skin = None;
                    self.wear_skin(None);
                    self.state.settings.account = None;
                    self.state.settings.skin_url.clear();
                    self.state.settings.skin_model.clear();
                    self.skin_status = if who.is_empty() {
                        "No skin on TLauncher yet.".to_string()
                    } else {
                        format!("{who} has not uploaded a skin on TLauncher yet.")
                    };
                    self.state.save();
                }
                Err(message) => self.skin_status = message,
            }
        }
    }

    /// Asks the chosen source what the typed name is wearing.
    pub fn check_skin(&mut self) {
        let source = self.state.settings.skin.clone();
        let name = if source == "local" {
            self.state.settings.skin_local.trim().to_string()
        } else {
            self.state.settings.skin_name.trim().to_string()
        };
        if source == "local" {
            self.use_local_skin(&name);
            return;
        }
        if name.is_empty() {
            self.skin_status = "Type the nickname first.".to_string();
            return;
        }
        let where_from = if source == "microsoft" {
            "Minecraft"
        } else {
            "TLauncher"
        };
        self.skin_status = format!("Looking {name} up on {where_from}…");
        self.skins.check(&source, &name);
    }

    /// A skin picked straight off the disk, no service involved.
    pub fn use_local_skin(&mut self, path: &str) {
        let file = path.trim().trim_matches('"');
        if file.is_empty() {
            self.skin_status = "Point to a skin file first.".to_string();
            return;
        }
        match std::fs::read(file) {
            Ok(bytes) => {
                let name = self
                    .state
                    .settings
                    .skin_name
                    .trim()
                    .to_string();
                let name = if name.is_empty() { "Player".to_string() } else { name };
                let uuid = skin::offline_uuid(&name);
                let name = std::path::Path::new(file)
                    .file_stem()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or(name);
                let who = name.clone();
                self.state.settings.skin_local = file.to_string();
                self.state.settings.account = Some(who.clone());
                self.state.settings.skin_url = String::new();
                self.state.settings.skin_model = "default".to_string();
                self.state.settings.skin_uuid = uuid.clone();
                self.skin_status = format!("{who} wears the skin on disk.");
                let worn = skin::Skin {
                    bytes: bytes.clone(),
                    model: "default".to_string(),
                };
                self.skin = Some(net::Skin {
                    url: file.to_string(),
                    model: "classic".to_string(),
                    name: who,
                    uuid,
                    bytes,
                    width: 64,
                    height: 64,
                });
                self.wear_skin(Some(worn));
                self.state.save();
            }
            Err(_) => self.skin_status = format!("{file} is not a picture we can read."),
        }
    }

    /// Hands the skin to the local service, starting it on the first call.
    /// From here the game can wear it without a mod, whatever it runs.
    fn wear_skin(&mut self, worn: Option<skin::Skin>) {
        let (name, uuid) = match self.skin.as_ref() {
            Some(skin) => (skin.name.clone(), skin.uuid.clone()),
            None => ("Player".to_string(), skin::offline_uuid("Player")),
        };
        match self.skin_server.as_ref() {
            Some(server) => {
                server.set_profile(&name, &uuid);
                server.set_skin(worn);
            }
            None => {
                if let Some(dir) = State::path() {
                    let profile = skin::Profile {
                        name,
                        uuid,
                        skin: worn,
                    };
                    self.skin_server = skin::Server::start(&dir, profile);
                }
            }
        }
        self.ensure_injector();
    }

    /// Fetches authlib-injector once, then keeps it beside the state file.
    fn ensure_injector(&mut self) {
        if self.injector.is_some() || self.injector_rx.is_some() {
            return;
        }
        let Some(dir) = State::path() else {
            return;
        };
        let (tx, rx) = channel();
        self.injector_rx = Some(rx);
        self.injector_status = "Fetching authlib-injector…".to_string();
        std::thread::spawn(move || {
            let _ = tx.send(injector_jar(&dir));
        });
    }

    pub fn skin_server_port(&self) -> Option<u16> {
        self.skin_server.as_ref().map(|server| server.port())
    }

    /// Everything the game needs to wear the skin: the offline profile, the
    /// agent and the prefetched metadata. Empty until a skin is being served.
    pub fn skin_launch_args(&self) -> Vec<String> {
        let Some(server) = self.skin_server.as_ref() else {
            return Vec::new();
        };
        let mut out = server.game_args();
        if let Some(jar) = self.injector.as_ref() {
            out.extend(server.jvm_args(jar));
        }
        out
    }

    /// What GitHub published, and the download that follows it.
    fn collect_upgrade(&mut self) {
        if let Some(outcome) = self.upgrade.poll() {
            match outcome {
                Ok(Some(release)) => {
                    let current = env!("CARGO_PKG_VERSION");
                    let skipped = self.state.settings.skip_version == release.tag;
                    if release.asset.is_some()
                        && net::newer(&release.tag, current)
                        && !skipped
                    {
                        self.upgrade_found = Some(release.clone());
                        self.open_update(&release);
                    }
                }
                // no release, or GitHub could not be asked: nothing to say
                Ok(None) | Err(_) => {}
            }
        }
        if let Some(outcome) = self.upgrade.poll_file() {
            match outcome {
                Ok(path) => self.apply_upgrade(&path),
                Err(why) => {
                    self.upgrade_note = why;
                    self.state.flash("The update did not finish.");
                }
            }
        }
    }

    /// Starts bringing the release down, if there is somewhere to put it.
    pub fn start_upgrade(&mut self) {
        let Some(release) = self.upgrade_found.clone() else {
            return;
        };
        let Some(dir) = State::path() else {
            self.upgrade_note = "There is nowhere to keep the download.".to_string();
            return;
        };
        if release.asset.is_none() {
            self.upgrade_note = format!("No build of {} for this machine.", release.tag);
            return;
        }
        self.upgrade_note = String::new();
        self.upgrade.fetch(&release, &dir);
    }

    /// The download is on disk: put it where this copy lives and start it.
    fn apply_upgrade(&mut self, path: &std::path::Path) {
        match updater::replace_and_relaunch(path) {
            Ok(()) => {
                self.state.save();
                std::process::exit(0);
            }
            Err(why) => {
                self.upgrade_note = why;
                self.state.flash("The update could not be installed.");
            }
        }
    }

    /// The player said later: do not ask about this release again.
    pub fn skip_upgrade(&mut self) {
        if let Some(release) = self.upgrade_found.clone() {
            self.state.settings.skip_version = release.tag;
            self.state.save();
        }
    }

    /// Collects authlib-injector when the download answers.
    fn collect_injector(&mut self) {
        let Some(rx) = self.injector_rx.as_ref() else {
            return;
        };
        let got = rx.try_recv().ok();
        if got.is_none() {
            return;
        }
        self.injector_rx = None;
        match got.flatten() {
            Some(path) => {
                self.injector_status = format!("authlib-injector ready ({})", path.display());
                self.injector = Some(path);
            }
            None => self.injector_status =
                "authlib-injector could not be downloaded; the skin stays in the launcher."
                    .to_string(),
        }
    }

    /// Runs the next waiting dependency lookup, if any.
    fn drain_dep_queue(&mut self) {
        if self.deps.busy {
            return;
        }
        if let Some((slug, name, kind, ask)) = self.dep_queue.first().cloned() {
            self.dep_queue.remove(0);
            self.fetch_deps(&slug, &name, kind, ask);
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
        self.collect_injector();
        self.collect_upgrade();
        self.drain_dep_queue();

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
            || self.skins.busy
            || self.injector_rx.is_some()
            || self.upgrade.busy
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

/// authlib-injector is what points the game at our own service. It comes from
/// the project's own download API and stays beside the state file afterwards.
fn injector_jar(dir: &std::path::Path) -> Option<PathBuf> {
    let path = dir.join("authlib-injector.jar");
    if path.is_file() {
        return Some(path);
    }
    let latest = net::download("https://authlib-injector.yushi.moe/artifact/latest.json")?;
    let info: serde_json::Value = serde_json::from_slice(&latest).ok()?;
    let url = info.get("download_url").and_then(|value| value.as_str())?;
    let bytes = net::download(url)?;
    let _ = std::fs::create_dir_all(dir);
    std::fs::write(&path, &bytes).ok()?;
    Some(path)
}
