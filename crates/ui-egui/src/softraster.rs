//! Draws egui's output on the CPU, with no graphics driver.
//!
//! egui turns a frame into textured triangles with premultiplied vertex
//! colours. This takes those triangles and fills them into a pixel buffer,
//! sampling the font texture, clipping to each primitive's rectangle and
//! blending the way egui's own renderers do (premultiplied "over", in gamma
//! space). It is simple and not fast, which is fine for pictures of the
//! window and for machines with no working graphics driver.

use std::collections::HashMap;

use eframe::egui::epaint::{ImageData, ImageDelta, Primitive};
use eframe::egui::{self, ClippedPrimitive, Color32, TextureId};

/// Largest picture drawn, so a bad size cannot ask for gigabytes.
pub const MAX_SIDE: usize = 8192;

#[derive(Clone, Debug)]
struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
}

impl Texture {
    /// Bilinear sample at `uv` (0 to 1), edge pixels repeated.
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let [w, h] = self.size;
        if w == 0 || h == 0 {
            return [0.0; 4];
        }
        let x = u * w as f32 - 0.5;
        let y = v * h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let get = |xi: f32, yi: f32| -> [f32; 4] {
            let xc = (xi.max(0.0) as usize).min(w - 1);
            let yc = (yi.max(0.0) as usize).min(h - 1);
            self.pixels
                .get(yc * w + xc)
                .map_or([0.0; 4], |c| c.to_array().map(f32::from))
        };
        let (a, b) = (get(x0, y0), get(x0 + 1.0, y0));
        let (c, d) = (get(x0, y0 + 1.0), get(x0 + 1.0, y0 + 1.0));
        lerp(lerp(a, b, fx), lerp(c, d, fx), fy)
    }
}

/// The textures egui has asked to be kept, such as the font atlas.
#[derive(Default)]
pub struct Textures {
    map: HashMap<TextureId, Texture>,
}

impl Textures {
    pub fn apply(&mut self, delta: &egui::TexturesDelta) {
        for (id, image) in &delta.set {
            self.set(*id, image);
        }
        for id in &delta.free {
            self.map.remove(id);
        }
    }

    fn set(&mut self, id: TextureId, delta: &ImageDelta) {
        let (size, pixels): ([usize; 2], Vec<Color32>) = match &delta.image {
            ImageData::Color(image) => (image.size, image.pixels.clone()),
            ImageData::Font(image) => (image.size, image.srgba_pixels(None).collect()),
        };
        match delta.pos {
            None => {
                self.map.insert(id, Texture { size, pixels });
            }
            Some([px, py]) => {
                let Some(texture) = self.map.get_mut(&id) else {
                    return;
                };
                let tw = texture.size[0];
                for row in 0..size[1] {
                    for col in 0..size[0] {
                        let (x, y) = (px + col, py + row);
                        if x >= tw || y >= texture.size[1] {
                            continue;
                        }
                        if let (Some(dst), Some(src)) = (
                            texture.pixels.get_mut(y * tw + x),
                            pixels.get(row * size[0] + col),
                        ) {
                            *dst = *src;
                        }
                    }
                }
            }
        }
    }
}

/// A picture being drawn: premultiplied RGBA bytes, row by row.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color32>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, background: Color32) -> Canvas {
        let (width, height) = (width.clamp(1, MAX_SIDE), height.clamp(1, MAX_SIDE));
        Canvas {
            width,
            height,
            pixels: vec![background; width * height],
        }
    }

    /// Draws egui's primitives, scaled by `pixels_per_point`.
    pub fn draw(
        &mut self,
        primitives: &[ClippedPrimitive],
        textures: &Textures,
        pixels_per_point: f32,
    ) {
        for primitive in primitives {
            let Primitive::Mesh(mesh) = &primitive.primitive else {
                continue; // paint callbacks need a graphics API
            };
            let clip = ClipBox::new(&primitive.clip_rect, pixels_per_point, self);
            let texture = textures.map.get(&mesh.texture_id);
            for tri in mesh.indices.chunks_exact(3) {
                let vertex = |i: u32| mesh.vertices.get(i as usize);
                let (Some(a), Some(b), Some(c)) = (
                    tri.first().and_then(|i| vertex(*i)),
                    tri.get(1).and_then(|i| vertex(*i)),
                    tri.get(2).and_then(|i| vertex(*i)),
                ) else {
                    continue;
                };
                let corners = [
                    Corner::new(a, pixels_per_point),
                    Corner::new(b, pixels_per_point),
                    Corner::new(c, pixels_per_point),
                ];
                self.triangle(&corners, texture, &clip);
            }
        }
    }

    fn triangle(&mut self, v: &[Corner; 3], texture: Option<&Texture>, clip: &ClipBox) {
        let [a, b, c] = v;
        let area = edge(a.x, a.y, b.x, b.y, c.x, c.y);
        if area.abs() < 1e-6 || !area.is_finite() {
            return;
        }
        let min_x = a.x.min(b.x).min(c.x).floor().max(clip.min_x as f32) as usize;
        let max_x = (a.x.max(b.x).max(c.x).ceil() as usize).min(clip.max_x);
        let min_y = a.y.min(b.y).min(c.y).floor().max(clip.min_y as f32) as usize;
        let max_y = (a.y.max(b.y).max(c.y).ceil() as usize).min(clip.max_y);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = edge(b.x, b.y, c.x, c.y, px, py) / area;
                let w1 = edge(c.x, c.y, a.x, a.y, px, py) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let mut src = [0.0f32; 4];
                for ((s, x), (y, z)) in src
                    .iter_mut()
                    .zip(a.color.iter())
                    .zip(b.color.iter().zip(c.color.iter()))
                {
                    *s = x * w0 + y * w1 + z * w2;
                }
                if let Some(t) = texture {
                    let u = a.u * w0 + b.u * w1 + c.u * w2;
                    let vv = a.v * w0 + b.v * w1 + c.v * w2;
                    let tex = t.sample(u, vv);
                    for (s, t) in src.iter_mut().zip(tex.iter()) {
                        *s = *s * t / 255.0;
                    }
                }
                if let Some(dst) = self.pixels.get_mut(y * self.width + x) {
                    *dst = over(src, *dst);
                }
            }
        }
    }
}

/// `src` (premultiplied, 0 to 255) over `dst`.
fn over(src: [f32; 4], dst: Color32) -> Color32 {
    let [sr, sg, sb, sa] = src;
    let [dr, dg, db, da] = dst.to_array().map(f32::from);
    let keep = 1.0 - sa / 255.0;
    let mix = |s: f32, d: f32| (s + d * keep).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(mix(sr, dr), mix(sg, dg), mix(sb, db), mix(sa, da))
}

/// `a` to `b` by `t` (0 to 1), per channel.
fn lerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *o = x + (y - x) * t;
    }
    out
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

struct Corner {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    color: [f32; 4],
}

impl Corner {
    fn new(vertex: &egui::epaint::Vertex, ppp: f32) -> Corner {
        Corner {
            x: vertex.pos.x * ppp,
            y: vertex.pos.y * ppp,
            u: vertex.uv.x,
            v: vertex.uv.y,
            color: vertex.color.to_array().map(f32::from),
        }
    }
}

/// The part of the canvas a primitive may touch, in pixels.
struct ClipBox {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

impl ClipBox {
    fn new(rect: &egui::Rect, ppp: f32, canvas: &Canvas) -> ClipBox {
        let cut = |value: f32, high: usize| (value.max(0.0) as usize).min(high);
        ClipBox {
            min_x: cut((rect.min.x * ppp).floor(), canvas.width),
            max_x: cut((rect.max.x * ppp).ceil(), canvas.width),
            min_y: cut((rect.min.y * ppp).floor(), canvas.height),
            max_y: cut((rect.max.y * ppp).ceil(), canvas.height),
        }
    }
}

/// A PNG of the canvas, with the image data stored uncompressed (valid and
/// readable everywhere, a little large).
pub fn encode_png(width: usize, height: usize, pixels: &[Color32]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(height.saturating_mul(width.saturating_mul(4) + 1));
    for row in pixels.chunks(width.max(1)).take(height) {
        raw.push(0); // filter: none
        for pixel in row {
            // the canvas is premultiplied; the window is opaque, so this is exact
            raw.extend_from_slice(&pixel.to_srgba_unmultiplied());
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65_535).peekable();
    if blocks.peek().is_none() {
        zlib.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        let last = u8::from(blocks.peek().is_none());
        let len = block.len() as u16;
        zlib.push(last);
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]); // 8 bits, RGBA
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);
    png
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::epaint::{Mesh, Vertex};
    use eframe::egui::{pos2, Rect};

    #[test]
    fn checksums_match_the_published_examples() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_png_has_the_right_shape() {
        let pixels = vec![Color32::from_rgb(10, 20, 30); 6];
        let png = encode_png(3, 2, &pixels);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &3u32.to_be_bytes());
        assert_eq!(&png[20..24], &2u32.to_be_bytes());
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]));
    }

    #[test]
    fn an_empty_image_still_encodes() {
        assert!(encode_png(0, 0, &[]).len() > 40);
    }

    #[test]
    fn large_images_use_several_blocks() {
        let pixels = vec![Color32::WHITE; 300 * 300];
        assert!(encode_png(300, 300, &pixels).len() > 300 * 300 * 4);
    }

    fn quad(rect: Rect, color: Color32) -> Mesh {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, color);
        mesh
    }

    fn primitive(mesh: Mesh, clip: Rect) -> ClippedPrimitive {
        ClippedPrimitive {
            clip_rect: clip,
            primitive: Primitive::Mesh(mesh),
        }
    }

    #[test]
    fn fills_a_rectangle_and_nothing_else() {
        let mut canvas = Canvas::new(10, 10, Color32::WHITE);
        let red = Color32::from_rgb(255, 0, 0);
        let rect = Rect::from_min_max(pos2(2.0, 2.0), pos2(6.0, 5.0));
        let clip = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        canvas.draw(
            &[primitive(quad(rect, red), clip)],
            &Textures::default(),
            1.0,
        );
        assert_eq!(canvas.pixels.get(3 * 10 + 3), Some(&red));
        assert_eq!(canvas.pixels.first(), Some(&Color32::WHITE));
        assert_eq!(canvas.pixels.get(3 * 10 + 7), Some(&Color32::WHITE));
    }

    #[test]
    fn the_clip_rectangle_limits_drawing() {
        let mut canvas = Canvas::new(10, 10, Color32::WHITE);
        let red = Color32::from_rgb(255, 0, 0);
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let clip = Rect::from_min_max(pos2(0.0, 0.0), pos2(5.0, 10.0));
        canvas.draw(
            &[primitive(quad(rect, red), clip)],
            &Textures::default(),
            1.0,
        );
        assert_eq!(canvas.pixels.get(2), Some(&red));
        assert_eq!(canvas.pixels.get(8), Some(&Color32::WHITE));
    }

    #[test]
    fn half_transparent_colour_blends_over_the_background() {
        let mut canvas = Canvas::new(4, 4, Color32::from_rgb(200, 200, 200));
        let half_black = Color32::from_rgba_premultiplied(0, 0, 0, 128);
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 4.0));
        canvas.draw(
            &[primitive(quad(rect, half_black), rect)],
            &Textures::default(),
            1.0,
        );
        let got = canvas.pixels.first().map(|c| c.r()).unwrap_or(0);
        assert!((98..=102).contains(&got), "{got}");
    }

    #[test]
    fn scaling_by_pixels_per_point_doubles_the_area() {
        let mut canvas = Canvas::new(20, 20, Color32::WHITE);
        let red = Color32::from_rgb(255, 0, 0);
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(5.0, 5.0));
        let clip = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        canvas.draw(
            &[primitive(quad(rect, red), clip)],
            &Textures::default(),
            2.0,
        );
        assert_eq!(canvas.pixels.get(9 * 20 + 9), Some(&red));
        assert_eq!(canvas.pixels.get(11 * 20 + 11), Some(&Color32::WHITE));
    }

    #[test]
    fn hostile_meshes_do_not_panic() {
        let mut canvas = Canvas::new(8, 8, Color32::WHITE);
        let mut mesh = Mesh::default();
        mesh.vertices.push(Vertex {
            pos: pos2(f32::NAN, 1.0),
            uv: pos2(0.0, 0.0),
            color: Color32::RED,
        });
        mesh.vertices.push(Vertex {
            pos: pos2(1e30, -1e30),
            uv: pos2(0.0, 0.0),
            color: Color32::RED,
        });
        mesh.indices = vec![0, 1, 99, 0, 1, 1, 7];
        let clip = Rect::from_min_max(pos2(-1e9, -1e9), pos2(1e9, 1e9));
        canvas.draw(&[primitive(mesh, clip)], &Textures::default(), 1.0);
        assert_eq!(canvas.pixels.len(), 64);
    }

    #[test]
    fn canvas_size_is_capped() {
        let canvas = Canvas::new(usize::MAX, 0, Color32::WHITE);
        assert_eq!(canvas.width, MAX_SIDE);
        assert_eq!(canvas.height, 1);
    }
}
