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
