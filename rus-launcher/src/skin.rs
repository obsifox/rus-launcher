//! The skin has to reach the game without a mod, on any version and with any
//! loader. The only door in is the game's own Yggdrasil service: Minecraft
//! asks it who the player is and what they are wearing. So we run one of our
//! own on 127.0.0.1 for as long as the launcher is open and point the game at
//! it with authlib-injector, which is how HMCL and PCL do offline skins.
//!
//! Everything here follows the authlib-injector specification:
//! https://github.com/yushijinhun/authlib-injector

use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use md5::{Digest as _, Md5};
use rsa::pkcs1::{DecodeRsaPrivateKey as _, EncodeRsaPrivateKey as _, LineEnding as Pkcs1LineEnding};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::{EncodePublicKey as _, LineEnding};
use rsa::rand_core::OsRng;
use rsa::signature::{RandomizedSigner as _, SignatureEncoding as _};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use sha1::Sha1;

/// One texture: the png and the arm width it asks for.
/// `model` is "slim" or "default", the two names the game knows.
#[derive(Clone)]
pub struct Skin {
    pub bytes: Vec<u8>,
    pub model: String,
}

/// The player the game will see.
#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub uuid: String,
    pub skin: Option<Skin>,
}

/// A skin kept ready to serve: re-saved so nothing but the bitmap survives,
/// and named after its own hash, which is how the game caches textures.
struct Texture {
    hash: String,
    png: Vec<u8>,
    model: String,
}

struct Shared {
    profile: Profile,
    texture: Option<Texture>,
    key: RsaPrivateKey,
    public_pem: String,
    token: String,
    client_token: String,
    port: u16,
}

/// The local Yggdrasil service. It answers only on 127.0.0.1 and only while
/// the launcher holds it.
pub struct Server {
    port: u16,
    shared: Arc<Mutex<Shared>>,
    metadata: String,
}

impl Server {
    pub fn start(dir: &Path, profile: Profile) -> Option<Server> {
        let key = key_pair(dir)?;
        let public_pem = RsaPublicKey::from(&key)
            .to_public_key_pem(LineEnding::LF)
            .ok()?;
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        let mut state = Shared {
            profile,
            texture: None,
            key,
            public_pem,
            token: stamp("token"),
            client_token: stamp("client"),
            port,
        };
        if let Some(skin) = state.profile.skin.clone() {
            state.texture = texture(&skin.bytes, &skin.model);
        }
        let shared = Arc::new(Mutex::new(state));
        let metadata = {
            let guard = shared.lock().ok()?;
            metadata_json(&guard)
        };
        let worker = shared.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(_) => continue,
                };
                let state = worker.clone();
                std::thread::spawn(move || handle(&mut stream, &state));
            }
        });
        Some(Server {
            port,
            shared,
            metadata,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The address the game has to be pointed at.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    /// The API metadata, which the launcher hands to the game so that
    /// authlib-injector never has to reach the network to find it.
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }

    /// The two JVM switches that put our service in front of Mojang's.
    pub fn jvm_args(&self, jar: &Path) -> Vec<String> {
        vec![
            format!("-javaagent:{}={}", jar.display(), self.url()),
            format!(
                "-Dauthlibinjector.yggdrasil.prefetched={}",
                BASE64.encode(self.metadata.as_bytes())
            ),
        ]
    }

    /// What the game is told about the player, in place of a Mojang login.
    pub fn game_args(&self) -> Vec<String> {
        let Ok(guard) = self.shared.lock() else {
            return Vec::new();
        };
        vec![
            "--username".into(),
            guard.profile.name.clone(),
            "--uuid".into(),
            guard.profile.uuid.clone(),
            "--accessToken".into(),
            guard.token.clone(),
            "--userType".into(),
            "mojang".into(),
            "--userProperties".into(),
            "{}".into(),
        ]
    }

    pub fn set_skin(&self, skin: Option<Skin>) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.profile.skin = skin.clone();
            guard.texture = skin.and_then(|skin| texture(&skin.bytes, &skin.model));
        }
    }

    pub fn set_profile(&self, name: &str, uuid: &str) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.profile.name = name.to_string();
            guard.profile.uuid = uuid.to_string();
        }
    }
}

/// The uuid every offline launcher and server builds for a name, so the
/// player keeps the same one wherever they go.
pub fn offline_uuid(name: &str) -> String {
    let mut hash = Md5::new();
    hash.update(b"OfflinePlayer:");
    hash.update(name.as_bytes());
    let mut bytes = hash.finalize();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let raw: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &raw[0..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..32]
    )
}

/// A skin straight off the internet can carry more than pixels, and the game
/// reads it as a bitmap, so we keep the bitmap and drop the rest.
fn texture(bytes: &[u8], model: &str) -> Option<Texture> {
    let image = image::load_from_memory(bytes).ok()?;
    let (w, h) = (image.width(), image.height());
    let shape = w > 0 && h > 0 && w % 64 == 0 && h % 32 == 0 && w <= 1024 && h <= 1024;
    if !shape {
        return None;
    }
    let mut png = Vec::new();
    image
        .to_rgba8()
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    let mut hash = sha2::Sha256::new();
    sha2::Digest::update(&mut hash, &png);
    let hash: String = sha2::Digest::finalize(hash)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Some(Texture {
        hash,
        png,
        model: model.to_string(),
    })
}

fn key_pair(dir: &Path) -> Option<RsaPrivateKey> {
    let path = dir.join("yggdrasil-key.pem");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(&text) {
            return Some(key);
        }
    }
    let key = RsaPrivateKey::new(&mut OsRng, 2048).ok()?;
    if let Ok(pem) = key.to_pkcs1_pem(Pkcs1LineEnding::LF) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(&path, pem.as_bytes());
    }
    Some(key)
}

fn stamp(kind: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let mut hash = sha2::Sha256::new();
    sha2::Digest::update(&mut hash, format!("{kind}-{now}").as_bytes());
    sha2::Digest::finalize(hash)
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0)
}

fn metadata_json(state: &Shared) -> String {
    json!({
        "meta": {
            "implementationName": "rus-launcher",
            "implementationVersion": env!("CARGO_PKG_VERSION"),
            "serverName": "RUS",
            "links": { "homepage": "https://github.com/obsifox/rus-launcher" },
            "feature.non_email_login": true
        },
        "skinDomains": ["127.0.0.1", "localhost"],
        "signaturePublickey": state.public_pem
    })
    .to_string()
}

/// Signs a profile value the way authlib checks it: SHA1withRSA over the
/// value's own bytes, base64 on the outside.
fn sign(state: &Shared, value: &str) -> Option<String> {
    let key = SigningKey::<Sha1>::new(state.key.clone());
    let signature = key.sign_with_rng(&mut OsRng, value.as_bytes());
    Some(BASE64.encode(signature.to_bytes()))
}

fn profile_value(state: &Shared, properties: bool) -> Value {
    let mut list = Vec::new();
    if properties {
        if let Some(texture) = &state.texture {
            let mut skin = json!({
                "url": format!("http://127.0.0.1:{}/textures/{}", state.port, texture.hash)
            });
            if texture.model == "slim" {
                skin["metadata"] = json!({ "model": "slim" });
            }
            let textures = json!({
                "timestamp": now_ms(),
                "profileId": state.profile.uuid,
                "profileName": state.profile.name,
                "textures": { "SKIN": skin }
            });
            let value = BASE64.encode(textures.to_string().as_bytes());
            if let Some(signature) = sign(state, &value) {
                list.push(json!({
                    "name": "textures",
                    "value": value,
                    "signature": signature
                }));
            }
        }
    }
    json!({
        "id": state.profile.uuid,
        "name": state.profile.name,
        "properties": list
    })
}

fn auth_value(state: &Shared) -> String {
    json!({
        "accessToken": state.token,
        "clientToken": state.client_token,
        "availableProfiles": [profile_value(state, false)],
        "selectedProfile": profile_value(state, true),
        "user": { "id": state.profile.uuid, "properties": [] }
    })
    .to_string()
}

fn reply(stream: &mut TcpStream, code: u16, reason: &str, kind: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn not_found(stream: &mut TcpStream) {
    let body = json!({
        "error": "Not Found",
        "errorMessage": "This is the RUS skin service, not Mojang's."
    })
    .to_string();
    reply(stream, 404, "Not Found", "application/json; charset=utf-8", body.as_bytes());
}

fn handle(stream: &mut TcpStream, shared: &Arc<Mutex<Shared>>) {
    let mut reader = match stream.try_clone() {
        Ok(clone) => BufReader::new(clone),
        Err(_) => return,
    };
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_ascii_uppercase();
    let target = parts.next().unwrap_or("/").to_string();
    let mut length = 0usize;
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => return,
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    if length > 0 {
        let mut body = vec![0u8; length];
        let _ = reader.read_exact(&mut body);
    }

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (target, String::new()),
    };
    let path = path.trim_end_matches('/').to_string();
    let Ok(state) = shared.lock() else {
        return;
    };

    match (method.as_str(), path.as_str()) {
        ("GET", "") => reply(
            stream,
            200,
            "OK",
            "application/json; charset=utf-8",
            metadata_json(&state).as_bytes(),
        ),
        ("POST", "/authserver/authenticate") | ("POST", "/authserver/refresh") => reply(
            stream,
            200,
            "OK",
            "application/json; charset=utf-8",
            auth_value(&state).as_bytes(),
        ),
        ("POST", "/authserver/validate")
        | ("POST", "/authserver/invalidate")
        | ("POST", "/authserver/signout")
        | ("POST", "/sessionserver/session/minecraft/join") => reply(stream, 204, "No Content", "application/json", b""),
        ("GET", "/sessionserver/session/minecraft/hasJoined") => reply(
            stream,
            200,
            "OK",
            "application/json; charset=utf-8",
            profile_value(&state, true).to_string().as_bytes(),
        ),
        ("POST", "/api/profiles/minecraft") => reply(
            stream,
            200,
            "OK",
            "application/json; charset=utf-8",
            json!([profile_value(&state, false)]).to_string().as_bytes(),
        ),
        _ if method == "GET" && path.starts_with("/sessionserver/session/minecraft/profile/") => {
            // the game asks with unsigned=true by default, but a signature
            // never hurt anyone, so we always send one
            reply(
                stream,
                200,
                "OK",
                "application/json; charset=utf-8",
                profile_value(&state, true).to_string().as_bytes(),
            )
        }
        _ if method == "GET" && path.starts_with("/textures/") => match &state.texture {
            Some(texture) if path.trim_start_matches("/textures/") == texture.hash => reply(
                stream,
                200,
                "OK",
                "image/png",
                &texture.png,
            ),
            _ => not_found(stream),
        },
        _ => not_found(stream),
    }
    let _ = query;
}
