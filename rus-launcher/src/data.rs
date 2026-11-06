use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub const LOADERS: [&str; 5] = ["Vanilla", "Fabric", "Forge", "NeoForge", "Quilt"];

pub const TYPES: [(&str, &str); 4] = [
    ("mod", "Mods"),
    ("resourcepack", "Resource Packs"),
    ("shader", "Shaders"),
    ("modpack", "Modpacks"),
];

pub const CF_CLASS: [(&str, u32); 4] = [
    ("mod", 6),
    ("resourcepack", 12),
    ("shader", 6),
    ("modpack", 4471),
];

#[derive(Clone)]
pub struct Version {
    pub version: &'static str,
    pub name: &'static str,
    pub file: &'static str,
    pub installed: bool,
    pub blurb: &'static str,
}

impl Version {
    pub fn poster(&self) -> String {
        format!("posters/{}", self.file.replace(".png", ".jpg"))
    }
    pub fn icon(&self) -> String {
        format!("icons/{}", self.file)
    }
}

pub fn versions() -> Vec<Version> {
    #[rustfmt::skip]
    let rows = [
        ("26.3", "Cushion", "26.3_cushion.png", true, "The latest release. Ready to play."),
        ("26.2", "Sulfur Cube", "26.2_sulfur_cube.png", false, "A release just behind the current one."),
        ("26.1", "Chick", "26.1_chick.png", false, "Open this release from the version list."),
        ("1.21.11", "Nautilus", "1.21.11_nautilus.png", true, "Installed for the Creative instance."),
        ("1.21.9", "Copper Golem", "1.21.9_copper_golem.png", false, "An earlier 1.21 release."),
        ("1.21.6", "Happy Ghast", "1.21.6_happy_ghast.png", false, "Keep it for an older world."),
        ("1.21.5", "Firefly Bush", "1.21.5_firefly_bush.png", false, "Not installed on this machine."),
        ("1.21.4", "Creaking", "1.21.4_creaking.png", false, "Not installed on this machine."),
        ("1.21.2", "Bundle", "1.21.2_bundle.png", false, "Not installed on this machine."),
        ("1.21", "Mace", "1.21_mace.png", false, "The first 1.21 release."),
        ("1.20.5", "Wolf Armor", "1.20.5_wolf_armor.png", false, "A 1.20 release."),
        ("1.20.3", "Decorated Pot", "1.20.3_decorated_pot.png", false, "A 1.20 release."),
        ("1.20", "Cherry Grove", "1.20_cherry_grove.png", false, "A 1.20 release."),
        ("1.19", "Warden", "1.19_warden.png", false, "An older release."),
        ("1.18", "Lush Caves", "1.18_lush_caves.png", false, "An older release."),
        ("1.17", "Axolotl", "1.17_axolotl.png", false, "An older release."),
        ("1.16", "Netherite", "1.16_netherite.png", false, "An older release."),
        ("1.15", "Bee", "1.15_bee.png", false, "An older release."),
        ("1.14", "Fox", "1.14_fox.png", true, "Installed for the Legacy instance."),
        ("1.13", "Dolphin", "1.13_dolphin.png", false, "An older release."),
        ("1.12", "Concrete", "1.12_concrete.png", false, "An older release."),
        ("1.11", "Shulker Box", "1.11_shulker_box.png", false, "An older release."),
        ("1.10", "Polar Bear", "1.10_polar_bear.png", false, "An older release."),
        ("1.9", "Elytra", "1.9_elytra.png", false, "An older release."),
        ("1.8", "Slime Block", "1.8_slime_block.png", false, "The oldest release in the list."),
    ];
    rows.iter()
        .map(|(version, name, file, installed, blurb)| Version {
            version,
            name,
            file,
            installed: *installed,
            blurb,
        })
        .collect()
}

pub fn version_index(list: &[Version], version: &str) -> usize {
    list.iter()
        .position(|v| v.version == version)
        .unwrap_or(0)
}

/// 26.3 -> 26, 1.21.11 -> 1.21
pub fn family_of(version: &str) -> String {
    if version.starts_with("26.") {
        return "26".to_string();
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

pub fn family_label(items: &[Version]) -> String {
    if items.len() > 1 {
        format!("{}.x", family_of(items[0].version))
    } else {
        items[0].version.to_string()
    }
}

pub fn family_groups(list: &[Version]) -> Vec<(String, Vec<usize>)> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, item) in list.iter().enumerate() {
        let key = family_of(item.version);
        if !map.contains_key(&key) {
            order.push(key.clone());
        }
        map.entry(key).or_default().push(i);
    }
    order.into_iter().map(|k| (k.clone(), map[&k].clone())).collect()
}

#[derive(Clone)]
pub struct Server {
    pub name: String,
    pub address: String,
    pub enabled: bool,
}

/// One installed thing in the Directory: a mod, a resource pack, a shader pack.
#[derive(Clone)]
pub struct Entry {
    pub name: String,
    /// Modrinth slug, when the item came from a search, so it can be updated.
    pub slug: Option<String>,
    pub version: Option<String>,
    pub enabled: bool,
    pub source: String,
    /// One line about the item, shown under its name.
    pub description: Option<String>,
    /// Icon address. The pixels are fetched in the background and cached by URL.
    pub icon: Option<String>,
}

/// The four drawers of the Directory.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mods,
    ResourcePacks,
    ShaderPacks,
    Servers,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Mods, Kind::ResourcePacks, Kind::ShaderPacks, Kind::Servers];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Mods => "Mods",
            Kind::ResourcePacks => "Resource packs",
            Kind::ShaderPacks => "Shader packs",
            Kind::Servers => "Servers",
        }
    }

    pub fn tab(self) -> &'static str {
        match self {
            Kind::Mods => "MODS",
            Kind::ResourcePacks => "RESOURCE PACKS",
            Kind::ShaderPacks => "SHADER PACKS",
            Kind::Servers => "SERVERS",
        }
    }

    /// The name used in the save file.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Mods => "mods",
            Kind::ResourcePacks => "resourcepacks",
            Kind::ShaderPacks => "shaderpacks",
            Kind::Servers => "servers",
        }
    }

    /// The value Modrinth uses for this kind of project.
    pub fn facet(self) -> &'static str {
        match self {
            Kind::Mods => "mod",
            Kind::ResourcePacks => "resourcepack",
            Kind::ShaderPacks => "shader",
            Kind::Servers => "mod",
        }
    }

    pub fn empty(self) -> &'static str {
        match self {
            Kind::Mods => "No mods here yet. Add them from the Mods page or by hand.",
            Kind::ResourcePacks => "No resource packs yet.",
            Kind::ShaderPacks => "No shader packs yet.",
            Kind::Servers => "No servers yet.",
        }
    }
}

impl Entry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slug: None,
            version: None,
            enabled: true,
            source: "local".into(),
            description: None,
            icon: None,
        }
    }

    /// An entry built from what a service told us about a project.
    pub fn found(name: &str, slug: &str, description: &str, icon: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            slug: Some(slug.to_string()),
            version: None,
            enabled: true,
            source: "modrinth".into(),
            description: Some(description.to_string()),
            icon,
        }
    }

    /// Dependencies install themselves, so they are marked as such.
    pub fn dependency(name: &str, slug: &str, description: &str, icon: Option<String>) -> Self {
        Self {
            source: "dependency".into(),
            ..Self::found(name, slug, description, icon)
        }
    }

    pub fn summary(&self) -> String {
        self.description.clone().unwrap_or_default()
    }
}

#[derive(Clone)]
pub struct Instance {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub source: String,
    pub image: Option<String>,
    pub mods: Vec<Entry>,
    pub resourcepacks: Vec<Entry>,
    pub shaderpacks: Vec<Entry>,
    pub servers: Vec<Server>,
}

impl Instance {
    pub fn entries(&self, kind: Kind) -> &Vec<Entry> {
        match kind {
            Kind::Mods => &self.mods,
            Kind::ResourcePacks => &self.resourcepacks,
            Kind::ShaderPacks => &self.shaderpacks,
            Kind::Servers => &self.mods,
        }
    }

    pub fn entries_mut(&mut self, kind: Kind) -> Option<&mut Vec<Entry>> {
        match kind {
            Kind::Mods => Some(&mut self.mods),
            Kind::ResourcePacks => Some(&mut self.resourcepacks),
            Kind::ShaderPacks => Some(&mut self.shaderpacks),
            Kind::Servers => None,
        }
    }

    pub fn count(&self, kind: Kind) -> usize {
        match kind {
            Kind::Servers => self.servers.len(),
            other => self.entries(other).len(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Page {
    Home,
    Instance,
    Directory,
    Mods,
    Settings,
}

#[derive(Clone)]
pub struct Settings {
    pub memory: u32,
    pub directory: String,
    pub language: String,
    pub skin: String,
    /// The nickname typed for the skin source, and the picture it wears.
    pub skin_name: String,
    pub skin_url: String,
    pub skin_model: String,
    pub skin_uuid: String,
    /// A skin chosen straight off the disk.
    pub skin_local: String,
    pub account: Option<String>,
    pub cf_key: String,
    pub close_on_play: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory: 4,
            directory: "~/Games/Rus".into(),
            language: "English".into(),
            skin: "local".into(),
            skin_name: String::new(),
            skin_url: String::new(),
            skin_model: String::new(),
            skin_uuid: String::new(),
            skin_local: String::new(),
            account: None,
            cf_key: String::new(),
            close_on_play: true,
        }
    }
}

pub const PIN_MAX: usize = 5;

pub struct State {
    pub list: Vec<Version>,
    pub instances: Vec<Instance>,
    pub current: u64,
    pub pins: Vec<u64>,
    pub page: Page,
    pub query: String,
    pub flash: Option<String>,
    pub flash_at: Instant,
    pub settings: Settings,
    next_id: u64,
}

impl State {
    pub fn new() -> Self {
        let mut state = Self {
            list: versions(),
            instances: Vec::new(),
            current: 1,
            pins: Vec::new(),
            page: Page::Instance,
            query: String::new(),
            flash: None,
            flash_at: Instant::now(),
            settings: Settings::default(),
            next_id: 1,
        };
        if !state.load() {
            state.seed();
        }
        state
    }

    fn seed(&mut self) {
        let survival = self.add("Survival", "26.3", "Fabric", "new");
        let creative = self.add("Creative", "1.21.11", "Vanilla", "new");
        let legacy = self.add("Legacy", "1.14", "Forge", "import");
        self.instance_mut(survival).servers.push(Server {
            name: "Hypixel".into(),
            address: "mc.hypixel.net".into(),
            enabled: true,
        });
        let survival = self.instances[0].id;
        let pack = self.instance_mut(survival);
        pack.mods = vec![
            Entry {
                name: "Sodium".into(),
                slug: Some("sodium".into()),
                version: Some("mc1.21-0.6.13".into()),
                enabled: true,
                source: "modrinth".into(),
                description: Some(
                    "A rendering rewrite for Fabric and NeoForge that lifts frame rates.".into(),
                ),
                icon: None,
            },
            Entry {
                name: "Iris Shaders".into(),
                slug: Some("iris".into()),
                version: None,
                enabled: true,
                source: "modrinth".into(),
                description: Some(
                    "Shader support for Fabric that runs most OptiFine shader packs.".into(),
                ),
                icon: None,
            },
            Entry {
                name: "JEI".into(),
                slug: None,
                version: None,
                enabled: false,
                source: "local".into(),
                description: Some(
                    "Lists every item and recipe so you can look them up while playing.".into(),
                ),
                icon: None,
            },
        ];
        pack.resourcepacks = vec![
            Entry {
                description: Some("The classic Faithful look, redrawn at 64x.".into()),
                ..Entry::new("Faithful 64x")
            },
            Entry {
                description: Some("Small optional tweaks that keep the vanilla look.".into()),
                ..Entry::new("Vanilla Tweaks")
            },
        ];
        pack.shaderpacks = vec![Entry {
            description: Some("A soft, cinematic shader pack that stays close to vanilla.".into()),
            ..Entry::new("Complementary Reimagined")
        }];
        self.instance_mut(legacy).servers.push(Server {
            name: "Legacy Realm".into(),
            address: "play.legacy.net".into(),
            enabled: true,
        });
        self.current = survival;
        self.pins = vec![survival, creative];
        let _ = self.pins.pop();
        self.pins = vec![survival];
    }

    pub fn add(&mut self, name: &str, version: &str, loader: &str, source: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.instances.push(Instance {
            id,
            name: name.to_string(),
            version: version.to_string(),
            loader: loader.to_string(),
            source: source.to_string(),
            image: None,
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            servers: Vec::new(),
        });
        id
    }

    pub fn instance(&self, id: u64) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn instance_mut(&mut self, id: u64) -> &mut Instance {
        let fallback = self.instances[0].id;
        let found = self
            .instances
            .iter()
            .position(|i| i.id == id)
            .unwrap_or_else(|| self.instances.iter().position(|i| i.id == fallback).unwrap_or(0));
        &mut self.instances[found]
    }

    pub fn current_instance(&self) -> &Instance {
        let id = self.current;
        self.instance(id)
            .unwrap_or_else(|| &self.instances[0])
    }

    pub fn current_instance_mut(&mut self) -> &mut Instance {
        let id = self.current;
        self.instance_mut(id)
    }

    pub fn version(&self, name: &str) -> usize {
        version_index(&self.list, name)
    }

    pub fn current_version(&self) -> &Version {
        let index = self.version(&self.current_instance().version.clone());
        &self.list[index]
    }

    pub fn flash(&mut self, text: impl Into<String>) {
        self.flash = Some(text.into());
        self.flash_at = Instant::now();
    }

    pub fn flash_text(&self) -> Option<&str> {
        match &self.flash {
            Some(text) if self.flash_at.elapsed() < std::time::Duration::from_secs(6) => {
                Some(text.as_str())
            }
            _ => None,
        }
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(at) = self.pins.iter().position(|p| *p == id) {
            let name = self.instance(id).map(|i| i.name.clone()).unwrap_or_default();
            self.pins.remove(at);
            self.flash(format!("{name} unpinned."));
            return;
        }
        if self.pins.len() >= PIN_MAX {
            self.flash(format!("The rail holds {PIN_MAX} pins. Unpin one first."));
            return;
        }
        let name = self.instance(id).map(|i| i.name.clone()).unwrap_or_default();
        self.pins.push(id);
        self.flash(format!("{name} pinned to the rail."));
    }

    pub fn duplicate(&mut self, id: u64) {
        let source = match self.instance(id) {
            Some(instance) => instance.clone(),
            None => return,
        };
        let mut name = format!("Copy of {}", source.name);
        let mut n = 2;
        while self.instances.iter().any(|i| i.name == name) {
            name = format!("Copy of {} {}", source.name, n);
            n += 1;
        }
        let copy = self.add(&name, &source.version, &source.loader, &source.source);
        let cloned = self.instance_mut(copy);
        cloned.image = source.image.clone();
        cloned.mods = source.mods.clone();
        cloned.resourcepacks = source.resourcepacks.clone();
        cloned.shaderpacks = source.shaderpacks.clone();
        cloned.servers = source.servers.clone();
        self.current = copy;
        let original = source.name.clone();
        self.flash(format!("{original} duplicated as {name}."));
    }

    pub fn rename(&mut self, id: u64, next: String) {
        let old = match self.instance(id) {
            Some(instance) => instance.name.clone(),
            None => return,
        };
        if next.trim().is_empty() || next == old {
            return;
        }
        if self.instances.iter().any(|i| i.name == next) {
            self.flash(format!("{next} is already taken."));
            return;
        }
        self.instance_mut(id).name = next.clone();
        self.flash(format!("{old} renamed to {next}."));
    }

    pub fn matches(&self) -> Vec<u64> {
        let query = self.query.trim().to_lowercase();
        self.instances
            .iter()
            .filter(|i| query.is_empty() || i.name.to_lowercase().contains(&query))
            .map(|i| i.id)
            .collect()
    }

    /* ---------- persistence ---------- */

    pub fn path() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
            Some(base.join("RusLauncher"))
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME").map(PathBuf::from)?;
            Some(home.join("Library/Application Support/Rus Launcher"))
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let base = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
            Some(base.join("rus-launcher"))
        }
    }

    pub fn save(&self) {
        let Some(dir) = Self::path() else { return };
        let _ = std::fs::create_dir_all(&dir);
        let instances: Vec<serde_json::Value> = self
            .instances
            .iter()
            .map(|i| {
                serde_json::json!({
                    "id": i.id,
                    "name": i.name,
                    "version": i.version,
                    "loader": i.loader,
                    "source": i.source,
                    "image": i.image,
                    "mods": entries_json(&i.mods),
                    "resourcepacks": entries_json(&i.resourcepacks),
                    "shaderpacks": entries_json(&i.shaderpacks),
                    "servers": i.servers.iter().map(|s| serde_json::json!({"name": s.name, "address": s.address, "enabled": s.enabled})).collect::<Vec<_>>(),
                })
            })
            .collect();
        let value = serde_json::json!({
            "current": self.current,
            "pins": self.pins,
            "installed": self.list.iter().map(|v| v.installed).collect::<Vec<_>>(),
            "settings": {
                "memory": self.settings.memory,
                "directory": self.settings.directory,
                "language": self.settings.language,
                "skin": self.settings.skin,
                "skin_name": self.settings.skin_name,
                "skin_url": self.settings.skin_url,
                "skin_model": self.settings.skin_model,
                "skin_uuid": self.settings.skin_uuid,
                "skin_local": self.settings.skin_local,
                "account": self.settings.account,
                "close_on_play": self.settings.close_on_play,
            },
            "instances": instances,
        });
        let _ = std::fs::write(dir.join("state.json"), value.to_string());
    }

    fn load(&mut self) -> bool {
        let Some(dir) = Self::path() else { return false };
        let Ok(text) = std::fs::read_to_string(dir.join("state.json")) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return false;
        };
        let mut ok = false;
        if let Some(items) = value.get("instances").and_then(|v| v.as_array()) {
            if !items.is_empty() {
                self.instances.clear();
                for item in items {
                    let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Instance")
                        .to_string();
                    let version = item
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("26.3")
                        .to_string();
                    let loader = item
                        .get("loader")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Vanilla")
                        .to_string();
                    let source = item
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("new")
                        .to_string();
                    let image = item
                        .get("image")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let mods = item
                        .get("mods")
                        .and_then(|v| v.as_array())
                        .map(|list| entries_from(list))
                        .unwrap_or_default();
                    let resourcepacks = item
                        .get("resourcepacks")
                        .and_then(|v| v.as_array())
                        .map(|list| entries_from(list))
                        .unwrap_or_default();
                    let shaderpacks = item
                        .get("shaderpacks")
                        .and_then(|v| v.as_array())
                        .map(|list| entries_from(list))
                        .unwrap_or_default();
                    let servers = item
                        .get("servers")
                        .and_then(|v| v.as_array())
                        .map(|list| {
                            list.iter()
                                .filter_map(|s| {
                                    Some(Server {
                                        name: s.get("name")?.as_str()?.to_string(),
                                        address: s.get("address")?.as_str()?.to_string(),
                                        enabled: s.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    self.next_id = self.next_id.max(id + 1);
                    self.instances.push(Instance {
                        id,
                        name,
                        version,
                        loader,
                        source,
                        image,
                        mods,
                        resourcepacks,
                        shaderpacks,
                        servers,
                    });
                }
                ok = true;
            }
        }
        if !ok {
            return false;
        }
        self.current = value.get("current").and_then(|v| v.as_u64()).unwrap_or(0);
        if self.instance(self.current).is_none() {
            self.current = self.instances[0].id;
        }
        self.pins = value
            .get("pins")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|v| v.as_u64())
                    .filter(|id| self.instance(*id).is_some())
                    .take(PIN_MAX)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(flags) = value.get("installed").and_then(|v| v.as_array()) {
            for (i, flag) in flags.iter().enumerate() {
                if let Some(item) = self.list.get_mut(i) {
                    item.installed = flag.as_bool().unwrap_or(item.installed);
                }
            }
        }
        if let Some(settings) = value.get("settings") {
            self.settings.memory = settings
                .get("memory")
                .and_then(|v| v.as_u64())
                .unwrap_or(4) as u32;
            if let Some(v) = settings.get("directory").and_then(|v| v.as_str()) {
                self.settings.directory = v.to_string();
            }
            if let Some(v) = settings.get("language").and_then(|v| v.as_str()) {
                self.settings.language = v.to_string();
            }
            if let Some(v) = settings.get("skin").and_then(|v| v.as_str()) {
                self.settings.skin = v.to_string();
            }
            if let Some(v) = settings.get("skin_name").and_then(|v| v.as_str()) {
                self.settings.skin_name = v.to_string();
            }
            if let Some(v) = settings.get("skin_url").and_then(|v| v.as_str()) {
                self.settings.skin_url = v.to_string();
            }
            if let Some(v) = settings.get("skin_model").and_then(|v| v.as_str()) {
                self.settings.skin_model = v.to_string();
            }
            if let Some(v) = settings.get("skin_uuid").and_then(|v| v.as_str()) {
                self.settings.skin_uuid = v.to_string();
            }
            if let Some(v) = settings.get("skin_local").and_then(|v| v.as_str()) {
                self.settings.skin_local = v.to_string();
            }
            self.settings.account = settings
                .get("account")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            self.settings.close_on_play = settings
                .get("close_on_play")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
        }
        true
    }
}

fn entries_json(list: &[Entry]) -> Vec<serde_json::Value> {
    list.iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "slug": e.slug,
                "version": e.version,
                "enabled": e.enabled,
                "source": e.source,
                "description": e.description,
                "icon": e.icon,
            })
        })
        .collect()
}

/// Reads a drawer. Older saves stored plain names, so both forms are accepted.
fn entries_from(list: &[serde_json::Value]) -> Vec<Entry> {
    list.iter()
        .map(|item| {
            if let Some(name) = item.as_str() {
                return Entry::new(name);
            }
            Entry {
                name: item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Item")
                    .to_string(),
                slug: item
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                version: item
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                enabled: item.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                source: item
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local")
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                icon: item
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }
        })
        .collect()
}
