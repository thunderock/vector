//! Two-atlas (mono + color) wgpu texture store with bounded LRU eviction.
//! D-40, D-43, Pitfall 2; consumed by Plan 03-03 compositor and Plan 03-05 DPR clear (D-48).

use std::collections::{HashMap, VecDeque};

use etagere::{size2, AllocId, AtlasAllocator};
use vector_fonts::{BitmapKind, RasterizedGlyph};
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

const ATLAS_DIM: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub character: char,
    pub dpr_bucket: u8,
}

#[derive(Debug, Clone, Copy)]
struct SlotEntry {
    alloc_id: AllocId,
    uv: [f32; 4],
    size_px: [u32; 2],
    offset_px: [i32; 2],
}

#[derive(Debug, Clone, Copy)]
pub enum AtlasSlot {
    Mono {
        uv: [f32; 4],
        size_px: [u32; 2],
        offset_px: [i32; 2],
    },
    Color {
        uv: [f32; 4],
        size_px: [u32; 2],
        offset_px: [i32; 2],
    },
    Fallback,
}

struct AtlasTexture {
    texture: Texture,
    view: TextureView,
    allocator: AtlasAllocator,
    slots: HashMap<GlyphKey, SlotEntry>,
    lru: VecDeque<GlyphKey>,
    width: u32,
    height: u32,
}

impl AtlasTexture {
    fn new(device: &Device, label: &str, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let alloc_w = i32::try_from(width).unwrap_or(i32::MAX);
        let alloc_h = i32::try_from(height).unwrap_or(i32::MAX);
        Self {
            texture,
            view,
            allocator: AtlasAllocator::new(size2(alloc_w, alloc_h)),
            slots: HashMap::new(),
            lru: VecDeque::new(),
            width,
            height,
        }
    }

    fn contains(&self, key: GlyphKey) -> bool {
        self.slots.contains_key(&key)
    }

    fn touch(&mut self, key: GlyphKey) {
        if let Some(pos) = self.lru.iter().position(|k| *k == key) {
            self.lru.remove(pos);
            self.lru.push_back(key);
        }
    }

    fn evict_one(&mut self) -> bool {
        if let Some(victim) = self.lru.pop_front() {
            if let Some(entry) = self.slots.remove(&victim) {
                self.allocator.deallocate(entry.alloc_id);
                return true;
            }
        }
        false
    }

    fn insert(
        &mut self,
        queue: &Queue,
        key: GlyphKey,
        glyph: &RasterizedGlyph,
        bytes_per_pixel: u32,
        bitmap: &[u8],
    ) -> Option<([f32; 4], [u32; 2], [i32; 2])> {
        if glyph.width > self.width || glyph.height > self.height {
            return None;
        }
        let w = i32::try_from(glyph.width).ok()?.max(1);
        let h = i32::try_from(glyph.height).ok()?.max(1);
        let alloc = loop {
            if let Some(a) = self.allocator.allocate(size2(w, h)) {
                break a;
            }
            if !self.evict_one() {
                return None;
            }
        };
        let origin_x = u32::try_from(alloc.rectangle.min.x).unwrap_or(0);
        let origin_y = u32::try_from(alloc.rectangle.min.y).unwrap_or(0);
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: origin_x,
                    y: origin_y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            bitmap,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(glyph.width * bytes_per_pixel),
                rows_per_image: Some(glyph.height),
            },
            Extent3d {
                width: glyph.width,
                height: glyph.height,
                depth_or_array_layers: 1,
            },
        );
        let uv = [
            f32_div(origin_x, self.width),
            f32_div(origin_y, self.height),
            f32_div(origin_x + glyph.width, self.width),
            f32_div(origin_y + glyph.height, self.height),
        ];
        let size_px = [glyph.width, glyph.height];
        let offset_px = [glyph.left, glyph.top];
        self.slots.insert(
            key,
            SlotEntry {
                alloc_id: alloc.id,
                uv,
                size_px,
                offset_px,
            },
        );
        self.lru.push_back(key);
        Some((uv, size_px, offset_px))
    }

    fn clear(&mut self) {
        let alloc_w = i32::try_from(self.width).unwrap_or(i32::MAX);
        let alloc_h = i32::try_from(self.height).unwrap_or(i32::MAX);
        self.allocator = AtlasAllocator::new(size2(alloc_w, alloc_h));
        self.slots.clear();
        self.lru.clear();
    }
}

pub struct Atlas {
    mono: AtlasTexture,
    color: AtlasTexture,
}

impl Atlas {
    pub fn new(device: &Device) -> Self {
        Self {
            mono: AtlasTexture::new(device, "vector-atlas-mono", ATLAS_DIM, ATLAS_DIM),
            color: AtlasTexture::new(device, "vector-atlas-color", ATLAS_DIM, ATLAS_DIM),
        }
    }

    /// Test-only: build tiny atlases so the LRU eviction path can be exercised quickly.
    #[doc(hidden)]
    pub fn new_with_dims(device: &Device, mono_dim: u32, color_dim: u32) -> Self {
        Self {
            mono: AtlasTexture::new(device, "vector-atlas-mono", mono_dim, mono_dim),
            color: AtlasTexture::new(device, "vector-atlas-color", color_dim, color_dim),
        }
    }

    pub fn mono_view(&self) -> &TextureView {
        &self.mono.view
    }

    pub fn color_view(&self) -> &TextureView {
        &self.color.view
    }

    pub fn contains(&self, key: GlyphKey) -> bool {
        self.mono.contains(key) || self.color.contains(key)
    }

    pub fn slot_for(&mut self, queue: &Queue, key: GlyphKey, glyph: &RasterizedGlyph) -> AtlasSlot {
        match &glyph.bitmap {
            BitmapKind::Mono(bytes) => {
                if let Some(entry) = self.mono.slots.get(&key).copied() {
                    self.mono.touch(key);
                    return AtlasSlot::Mono {
                        uv: entry.uv,
                        size_px: entry.size_px,
                        offset_px: entry.offset_px,
                    };
                }
                let mono_bytes = expand_rgb_to_rgba(bytes, glyph.width, glyph.height);
                match self.mono.insert(queue, key, glyph, 4, &mono_bytes) {
                    Some((uv, size_px, offset_px)) => AtlasSlot::Mono {
                        uv,
                        size_px,
                        offset_px,
                    },
                    None => AtlasSlot::Fallback,
                }
            }
            BitmapKind::Color(bytes) => {
                if let Some(entry) = self.color.slots.get(&key).copied() {
                    self.color.touch(key);
                    return AtlasSlot::Color {
                        uv: entry.uv,
                        size_px: entry.size_px,
                        offset_px: entry.offset_px,
                    };
                }
                match self.color.insert(queue, key, glyph, 4, bytes) {
                    Some((uv, size_px, offset_px)) => AtlasSlot::Color {
                        uv,
                        size_px,
                        offset_px,
                    },
                    None => AtlasSlot::Fallback,
                }
            }
        }
    }

    /// Plan 03-05's ScaleFactorChanged handler calls this (D-48).
    pub fn clear_all(&mut self) {
        self.mono.clear();
        self.color.clear();
    }
}

/// crossfont mono = 3-channel RGB alphamask. Expand to RGBA (alpha = max(r,g,b)) for Rgba8Unorm.
fn expand_rgb_to_rgba(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width as usize) * (height as usize);
    let mut out = Vec::with_capacity(pixel_count * 4);
    for px in rgb.chunks_exact(3).take(pixel_count) {
        let red = px[0];
        let green = px[1];
        let blue = px[2];
        let alpha = red.max(green).max(blue);
        out.extend_from_slice(&[red, green, blue, alpha]);
    }
    out
}

#[allow(clippy::cast_precision_loss)]
fn f32_div(num: u32, den: u32) -> f32 {
    num as f32 / den as f32
}
