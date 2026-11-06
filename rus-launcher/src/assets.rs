use egui::{Context, TextureHandle, TextureId};
use include_dir::{include_dir, Dir};
use std::collections::HashMap;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/launcher");

const POSTER_MAX: u32 = 720;
const ICON_MAX: u32 = 96;
const TAB_MAX: u32 = 260;
const MARK_MAX: u32 = 256;

fn max_dim(path: &str) -> u32 {
    if path.starts_with("posters/") {
        POSTER_MAX
    } else if path.starts_with("icons/") {
        ICON_MAX
    } else if path.starts_with("tabs/") {
        TAB_MAX
    } else {
        MARK_MAX
    }
}

pub struct Assets {
    textures: HashMap<String, TextureHandle>,
    aspects: HashMap<String, f32>,
    missing: Vec<String>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            aspects: HashMap::new(),
            missing: Vec::new(),
        }
    }

    pub fn has(&self, path: &str) -> bool {
        ASSETS.get_file(path).is_some()
    }

    fn decode(bytes: &[u8], path: &str) -> Option<(Vec<u8>, u32, u32)> {
        let image = image::load_from_memory(bytes).ok()?;
        let image = image.resize(max_dim(path), max_dim(path), image::imageops::FilterType::Triangle);
        let rgba = image.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        Some((rgba.into_raw(), w, h))
    }

    fn load(&mut self, ctx: &Context, path: &str) -> Option<TextureHandle> {
        if let Some(found) = self.textures.get(path) {
            return Some(found.clone());
        }
        let bytes = ASSETS.get_file(path)?.contents();
        let (pixels, w, h) = Self::decode(bytes, path)?;
        let texture = ctx.load_texture(
            path,
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels),
            egui::TextureOptions::LINEAR,
        );
        self.aspects.insert(path.to_string(), w as f32 / h as f32);
        self.textures.insert(path.to_string(), texture.clone());
        Some(texture)
    }

    /// The texture id for an embedded file, or nothing while it loads.
    pub fn id(&mut self, ctx: &Context, path: &str) -> Option<TextureId> {
        match self.load(ctx, path) {
            Some(handle) => Some(handle.id()),
            None => {
                if !self.missing.contains(&path.to_string()) {
                    self.missing.push(path.to_string());
                }
                None
            }
        }
    }

    pub fn aspect(&mut self, ctx: &Context, path: &str) -> f32 {
        if let Some(value) = self.aspects.get(path) {
            return *value;
        }
        if self.load(ctx, path).is_some() {
            return self.aspects.get(path).copied().unwrap_or(1.0);
        }
        1.0
    }

    /// Stores pixels that came from the network, for example a search result icon.
    pub fn store(&mut self, ctx: &Context, key: &str, bytes: &[u8]) -> Option<TextureId> {
        if let Some(handle) = self.textures.get(key) {
            return Some(handle.id());
        }
        let image = image::load_from_memory(bytes)
            .ok()?
            .resize(96, 96, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let (w, h) = (image.width(), image.height());
        let texture = ctx.load_texture(
            key,
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        self.aspects.insert(key.to_string(), w as f32 / h as f32);
        self.textures.insert(key.to_string(), texture.clone());
        Some(texture.id())
    }
}
