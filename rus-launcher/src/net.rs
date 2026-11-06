use std::io::Read;
use std::sync::mpsc::{channel, Receiver, Sender};

const AGENT: &str = "obsifox-rus-launcher/0.1 (https://github.com/obsifox/rus-launcher)";

#[derive(Clone)]
pub struct Hit {
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub icon: Option<Vec<u8>>,
    pub downloads: u64,
    pub added: bool,
    /// Modrinth slug, when the service gives one. Used to look for updates.
    pub slug: Option<String>,
    pub version: Option<String>,
}

/// True when a version is a finished release rather than a beta.
fn is_release(value: &serde_json::Value) -> bool {
    value.get("version_type").and_then(|v| v.as_str()) == Some("release")
}

fn has_loader(value: &serde_json::Value, loader: &str) -> bool {
    if loader.is_empty() {
        return true;
    }
    value
        .get("loaders")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .any(|item| item.as_str().map(|s| s.to_lowercase()) == Some(loader.to_string()))
        })
        .unwrap_or(false)
}

fn has_game(value: &serde_json::Value, game_version: &str) -> bool {
    if game_version.is_empty() {
        return true;
    }
    value
        .get("game_versions")
        .and_then(|v| v.as_array())
        .map(|list| list.iter().any(|item| item.as_str() == Some(game_version)))
        .unwrap_or(false)
}

/// The version that fits this instance best: right loader, right game version.
fn pick_version<'a>(
    versions: &'a [serde_json::Value],
    loader: &str,
    game_version: &str,
) -> Option<&'a serde_json::Value> {
    versions
        .iter()
        .find(|v| is_release(v) && has_loader(v, loader) && has_game(v, game_version))
        .or_else(|| versions.iter().find(|v| is_release(v) && has_loader(v, loader)))
        .or_else(|| versions.iter().find(|v| is_release(v)))
        .or_else(|| versions.first())
}

/// Asks Modrinth for the newest version of a project that fits this instance.
pub fn latest_version(slug: &str, loader: &str, game_version: &str) -> Result<Option<String>, String> {
    let url = format!("https://api.modrinth.com/v2/project/{slug}/version");
    let mut body = get(&url, None)?;
    let json: serde_json::Value = body
        .read_json()
        .map_err(|_| "Modrinth sent something we could not read.".to_string())?;
    let empty = Vec::new();
    let versions = json.as_array().unwrap_or(&empty);
    Ok(pick_version(versions, &loader.to_lowercase(), game_version)
        .and_then(|v| v.get("version_number"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

pub struct Search {
    pub source: String,
    pub kind: String,
    pub query: String,
    pub results: Vec<Hit>,
    pub status: String,
    pub busy: bool,
    rx: Option<Receiver<Result<Vec<Hit>, String>>>,
    tx: Option<Sender<Result<Vec<Hit>, String>>>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            source: "modrinth".into(),
            kind: "mod".into(),
            query: String::new(),
            results: Vec::new(),
            status: String::new(),
            busy: false,
            rx: None,
            tx: None,
        }
    }
}

/// Background "is there a newer version?" checks started from the Directory.
pub struct Updates {
    pub busy: bool,
    rx: Option<Receiver<Result<(String, Option<String>), String>>>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            busy: false,
            rx: None,
        }
    }
}

impl Updates {
    /// Looks up `slug` and reports back as (entry name, newest version).
    pub fn check(&mut self, name: &str, slug: &str, loader: &str, game_version: &str) {
        if self.busy {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.busy = true;
        let name = name.to_string();
        let slug = slug.to_string();
        let loader = loader.to_string();
        let game_version = game_version.to_string();
        std::thread::spawn(move || {
            let outcome =
                latest_version(&slug, &loader, &game_version).map(|version| (name, version));
            let _ = tx.send(outcome);
        });
    }

    pub fn poll(&mut self, state: &mut crate::data::State) {
        if !self.busy {
            return;
        }
        let mut done = None;
        if let Some(rx) = &self.rx {
            if let Ok(value) = rx.try_recv() {
                done = Some(value);
            }
        }
        if let Some(value) = done {
            self.busy = false;
            self.rx = None;
            let instance = state.current_instance_mut();
            match value {
                Ok((name, version)) => {
                    let target = instance
                        .mods
                        .iter_mut()
                        .chain(instance.resourcepacks.iter_mut())
                        .chain(instance.shaderpacks.iter_mut())
                        .find(|e| e.name == name);
                    match (target, version) {
                        (Some(entry), Some(version)) => {
                            if entry.version.as_deref() == Some(version.as_str()) {
                                state.flash(format!("{name} is up to date."));
                            } else {
                                entry.version = Some(version.clone());
                                state.flash(format!("{name} updated to {version}."));
                            }
                        }
                        (Some(_), None) => state.flash(format!("{name} is up to date.")),
                        (None, _) => {}
                    }
                }
                Err(error) => state.flash(error),
            }
            state.save();
        }
    }
}

impl Search {
    pub fn start(&mut self, kind: &str, query: &str, loader: &str, cf_key: &str) {
        if self.busy {
            return;
        }
        if self.source == "curseforge" && cf_key.trim().is_empty() {
            self.status = "CurseForge needs an API key. Add it in Settings → CurseForge API key.".into();
            self.results.clear();
            return;
        }
        let (tx, rx) = channel();
        self.tx = Some(tx.clone());
        self.rx = Some(rx);
        self.busy = true;
        self.status = "Loading…".into();
        self.results.clear();

        let source = self.source.clone();
        let kind = kind.to_string();
        let query = query.to_string();
        let loader = loader.to_lowercase();
        let key = cf_key.to_string();
        std::thread::spawn(move || {
            let outcome = run(&source, &kind, &query, &loader, &key);
            let _ = tx.send(outcome);
        });
    }

    pub fn poll(&mut self) {
        if !self.busy {
            return;
        }
        let mut done: Option<Result<Vec<Hit>, String>> = None;
        if let Some(rx) = &self.rx {
            if let Ok(value) = rx.try_recv() {
                done = Some(value);
            }
        }
        if let Some(value) = done {
            self.busy = false;
            self.rx = None;
            self.tx = None;
            match value {
                Ok(hits) => {
                    self.results = hits;
                    self.status = String::new();
                }
                Err(message) => {
                    self.results.clear();
                    self.status = message;
                }
            }
        }
    }
}

/// One agent for the whole app: keeps the connection pool, and never lets a
/// request hang forever - a dead network must not leave a spinner on screen.
fn agent() -> ureq::Agent {
    ureq::config::Config::builder()
        .timeout_connect(Some(std::time::Duration::from_secs(8)))
        .timeout_global(Some(std::time::Duration::from_secs(25)))
        .build()
        .new_agent()
}

fn get(url: &str, key: Option<&str>) -> Result<ureq::Body, String> {
    let mut request = agent().get(url).header("User-Agent", AGENT);
    if let Some(key) = key {
        request = request.header("x-api-key", key);
    }
    let out = request
        .call()
        .map(|response| response.into_body())
        .map_err(|error| match error {
            ureq::Error::StatusCode(code) => format!("The service answered with {code}."),
            other => format!("The service could not be reached ({other})."),
        });
    out
}

/// Small files (icons) must never hold up the rest of the request, so they get
/// their own short deadline.
fn bytes(url: &str) -> Option<Vec<u8>> {
    let agent = ureq::config::Config::builder()
        .timeout_connect(Some(std::time::Duration::from_secs(5)))
        .timeout_global(Some(std::time::Duration::from_secs(6)))
        .build()
        .new_agent();
    let mut body = agent
        .get(url)
        .header("User-Agent", AGENT)
        .call()
        .ok()?
        .into_body();
    let mut out = Vec::new();
    body.as_reader().read_to_end(&mut out).ok()?;
    Some(out)
}

fn run(source: &str, kind: &str, query: &str, loader: &str, cf_key: &str) -> Result<Vec<Hit>, String> {
    if source == "curseforge" {
        let class = crate::data::CF_CLASS
            .iter()
            .find(|(key, _)| *key == kind)
            .map(|(_, value)| *value)
            .unwrap_or(6);
        let url = format!(
            "https://api.curseforge.com/v1/mods/search?gameId=432&classId={class}&searchFilter={}&pageSize=12",
            urlencode(query)
        );
        let mut body = get(&url, Some(cf_key))?;
        let json: serde_json::Value = body
            .read_json()
            .map_err(|_| "CurseForge sent something we could not read.".to_string())?;
        let empty = Vec::new();
        let items = json.get("data").and_then(|v| v.as_array()).unwrap_or(&empty);
        Ok(items
            .iter()
            .map(|item| Hit {
                title: item.get("name").and_then(|v| v.as_str()).unwrap_or("Mod").into(),
                description: item
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                slug: item.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string()),
                version: None,
                icon_url: item
                    .get("logo")
                    .and_then(|v| v.get("thumbnailUrl"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                icon: None,
                downloads: item
                    .get("downloadCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                added: false,
            })
            .collect())
    } else {
        let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{kind}")]];
        if kind != "resourcepack" && kind != "shader" && loader != "vanilla" {
            facets.push(vec![format!("categories:{loader}")]);
        }
        let encoded = serde_json::to_string(&facets).unwrap_or_default();
        let url = format!(
            "https://api.modrinth.com/v2/search?query={}&facets={}&limit=12",
            urlencode(query),
            urlencode(&encoded)
        );
        let mut body = get(&url, None)?;
        let json: serde_json::Value = body
            .read_json()
            .map_err(|_| "Modrinth sent something we could not read.".to_string())?;
        let empty = Vec::new();
        let hits = json.get("hits").and_then(|v| v.as_array()).unwrap_or(&empty);
        // Icons are fetched in parallel: a slow CDN must not stall the results.
        let mut out = Vec::new();
        let mut icons = Vec::new();
        for hit in hits {
            let url = hit.get("icon_url").and_then(|v| v.as_str()).map(String::from);
            icons.push({
                let url = url.clone();
                std::thread::spawn(move || url.as_ref().and_then(|value| bytes(value)))
            });
            out.push(Hit {
                title: hit.get("title").and_then(|v| v.as_str()).unwrap_or("Project").into(),
                description: hit
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                icon_url: url,
                icon: None,
                downloads: hit.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                added: false,
                slug: hit.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string()),
                // Modrinth's search answers with a version id here, not a
                // version number, so the real one is fetched on demand.
                version: None,
            });
        }
        for (hit, handle) in out.iter_mut().zip(icons) {
            if let Ok(icon) = handle.join() {
                hit.icon = icon;
            }
        }
        Ok(out)
    }
}

/* ------------------------------------------------------- project details */

/// What a service told us about one project.
#[derive(Clone)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
}

impl Project {
    fn from_json(value: &serde_json::Value) -> Option<Self> {
        let slug = value.get("slug").and_then(|v| v.as_str())?.to_string();
        Some(Self {
            slug,
            title: value
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            description: value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            icon_url: value
                .get("icon_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

/// Something a project needs in order to run. Required ones install themselves,
/// optional ones are only offered.
#[derive(Clone)]
pub struct Dep {
    pub project: Project,
    pub required: bool,
}

/// Everything Modrinth knows about one project.
pub fn project(slug: &str) -> Option<Project> {
    let url = format!("https://api.modrinth.com/v2/project/{}", urlencode(slug));
    let mut body = get(&url, None).ok()?;
    let json: serde_json::Value = body.read_json().ok()?;
    Project::from_json(&json)
}

/// Looks a project up by name, so items added by hand still get their text and art.
pub fn find(kind: &str, name: &str) -> Option<Project> {
    let facet = match kind {
        "resourcepack" => "resourcepack",
        "shader" => "shader",
        _ => "mod",
    };
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&facets=%5B%5B%22project_type%3A{}%22%5D%5D&limit=1",
        urlencode(name),
        facet
    );
    let mut body = get(&url, None).ok()?;
    let json: serde_json::Value = body.read_json().ok()?;
    let hits = json.get("hits").and_then(|v| v.as_array())?;
    let slug = hits.first()?.get("slug").and_then(|v| v.as_str())?;
    project(slug)
}

/// The dependencies of the release that fits this instance.
pub fn dependencies(slug: &str, loader: &str, game_version: &str) -> Option<Vec<Dep>> {
    let url = format!(
        "https://api.modrinth.com/v2/project/{}/version",
        urlencode(slug)
    );
    let mut body = get(&url, None).ok()?;
    let json: serde_json::Value = body.read_json().ok()?;
    let versions = json.as_array()?;
    let empty = Vec::new();
    let picked = pick_version(versions, &loader.to_lowercase(), game_version)?;
    let deps = picked
        .get("dependencies")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let mut ids: Vec<String> = Vec::new();
    let mut required: Vec<bool> = Vec::new();
    for dep in deps {
        let id = match dep.get("project_id").and_then(|v| v.as_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };
        if ids.contains(&id) {
            continue;
        }
        ids.push(id);
        required.push(dep.get("dependency_type").and_then(|v| v.as_str()) == Some("required"));
    }
    if ids.is_empty() {
        return Some(Vec::new());
    }
    let list = ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",");
    let url = format!(
        "https://api.modrinth.com/v2/projects?ids={}",
        urlencode(&format!("[{list}]"))
    );
    let mut body = get(&url, None).ok()?;
    let json: serde_json::Value = body.read_json().ok()?;
    let mut out: Vec<Dep> = Vec::new();
    for item in json.as_array()? {
        if let Some(project) = Project::from_json(item) {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let needed = ids
                .iter()
                .position(|value| value == id)
                .and_then(|index| required.get(index).copied())
                .unwrap_or(false);
            out.push(Dep {
                project,
                required: needed,
            });
        }
    }
    out.sort_by(|a, b| b.required.cmp(&a.required));
    Some(out)
}

/// Looks up one item that was added by hand, on a worker thread.
pub struct Details {
    pub busy: bool,
    /// Whether the caller wants the dependencies chased once the lookup lands.
    chain: bool,
    rx: Option<Receiver<(String, Option<Project>)>>,
}

impl Default for Details {
    fn default() -> Self {
        Self {
            busy: false,
            chain: false,
            rx: None,
        }
    }
}

impl Details {
    pub fn look_up(&mut self, name: &str, kind: &str) {
        if self.busy || name.trim().is_empty() {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.busy = true;
        self.chain = true;
        let name = name.to_string();
        let kind = kind.to_string();
        std::thread::spawn(move || {
            let found = find(&kind, &name);
            let _ = tx.send((name, found));
        });
    }

    /// Fills in an item we already know the slug of, without chasing dependencies.
    pub fn fill(&mut self, name: &str, slug: &str) {
        if self.busy || slug.trim().is_empty() {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.busy = true;
        self.chain = false;
        let name = name.to_string();
        let slug = slug.to_string();
        std::thread::spawn(move || {
            let found = project(&slug);
            let _ = tx.send((name, found));
        });
    }

    /// True when this lookup should be followed by a dependency check.
    pub fn chained(&self) -> bool {
        self.chain
    }

    /// Returns (entry name, what was found) once the lookup lands.
    pub fn poll(&mut self) -> Option<(String, Option<Project>)> {
        let got = self.rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if got.is_some() {
            self.rx = None;
            self.busy = false;
        }
        got
    }
}

/// Resolves the dependencies of a project on a worker thread.
pub struct Deps {
    pub busy: bool,
    rx: Option<Receiver<(String, String, Option<Vec<Dep>>)>>,
}

impl Default for Deps {
    fn default() -> Self {
        Self { busy: false, rx: None }
    }
}

impl Deps {
    pub fn fetch(&mut self, slug: &str, name: &str, loader: &str, game_version: &str) {
        if self.busy || slug.trim().is_empty() {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.busy = true;
        let slug = slug.to_string();
        let name = name.to_string();
        let loader = loader.to_string();
        let game_version = game_version.to_string();
        std::thread::spawn(move || {
            let found = dependencies(&slug, &loader, &game_version);
            let _ = tx.send((slug, name, found));
        });
    }

    /// Returns (slug, project name, dependencies) once the lookup lands.
    pub fn poll(&mut self) -> Option<(String, String, Option<Vec<Dep>>)> {
        let got = self.rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if got.is_some() {
            self.rx = None;
            self.busy = false;
        }
        got
    }
}

/// Icon pixels, fetched one at a time in the background and kept by URL.
pub struct Icons {
    queue: Vec<String>,
    inflight: Option<(String, Receiver<Option<Vec<u8>>>)>,
    pub done: Vec<(String, Vec<u8>)>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            inflight: None,
            done: Vec::new(),
        }
    }
}

impl Icons {
    pub fn want(&mut self, url: &str) {
        if url.is_empty() || self.has(url) || self.queue.iter().any(|u| u == url) {
            return;
        }
        self.queue.push(url.to_string());
    }

    pub fn has(&self, url: &str) -> bool {
        self.done.iter().any(|(known, _)| known == url)
    }

    pub fn get(&self, url: &str) -> Option<&Vec<u8>> {
        self.done
            .iter()
            .find(|(known, _)| known == url)
            .map(|(_, bytes)| bytes)
    }

    pub fn waiting(&self) -> bool {
        !self.queue.is_empty() || self.inflight.is_some()
    }

    /// Starts the next download and stores whatever arrived. True when new
    /// pixels landed, so the caller knows to ask for another frame.
    pub fn pump(&mut self) -> bool {
        if self.inflight.is_none() {
            if !self.queue.is_empty() {
                let url = self.queue.remove(0);
                let (tx, rx) = channel();
                let target = url.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(bytes(&target));
                });
                self.inflight = Some((url, rx));
            }
        }
        let mut fresh = false;
        let mut finished = false;
        if let Some((url, rx)) = &self.inflight {
            match rx.try_recv() {
                Ok(Some(pixels)) => {
                    if !pixels.is_empty() {
                        self.done.push((url.clone(), pixels));
                        fresh = true;
                    }
                    finished = true;
                }
                Ok(None) => finished = true,
                Err(_) => {}
            }
        }
        if finished {
            self.inflight = None;
        }
        fresh
    }
}

fn urlencode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match *byte as char {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(*byte as char),
            ' ' => out.push('+'),
            other => {
                for byte in other.to_string().as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}
