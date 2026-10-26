//! A developer hook: saves a picture of the window to a file, so the
//! interface can be checked without a person looking at the screen.
//!
//! Off unless `RCRAFT_SCREENSHOT` names a PNG file to write. With
//! `RCRAFT_DEMO_FILE` naming a text file, each line in it is run as a typed
//! command first (one at a time, waiting for each to finish). The window
//! closes itself afterwards. Nothing here runs in normal use.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

/// Frames to wait after the last command, so the window has settled.
const SETTLE_FRAMES: u32 = 20;
/// Give up and close after this long.
const GIVE_UP: Duration = Duration::from_secs(60);
/// Most demo lines read from the file.
const MAX_DEMO_LINES: usize = 200;

pub struct Shot {
    path: PathBuf,
    demo: Vec<String>,
    next: usize,
    settled: u32,
    requested: bool,
    started: Instant,
}

impl Shot {
    pub fn from_env() -> Option<Shot> {
        let path = PathBuf::from(std::env::var_os("RCRAFT_SCREENSHOT").filter(|v| !v.is_empty())?);
        let demo = std::env::var_os("RCRAFT_DEMO_FILE")
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|text| {
                text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .take(MAX_DEMO_LINES)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Some(Shot {
            path,
            demo,
            next: 0,
            settled: 0,
            requested: false,
            started: Instant::now(),
        })
    }

    /// Called every frame once R is ready and idle. Returns the next demo
    /// command to run, if there is one.
    pub fn next_command(&mut self) -> Option<String> {
        let line = self.demo.get(self.next).cloned();
        if line.is_some() {
            self.next += 1;
            self.settled = 0;
        }
        line
    }

    /// Called every frame. Asks for the picture when the demo is done and the
    /// window has settled, saves it when it arrives, then closes the window.
    pub fn step(&mut self, ctx: &egui::Context, idle_and_done: bool) {
        ctx.request_repaint_after(Duration::from_millis(50));
        if self.started.elapsed() > GIVE_UP {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let pictures: Vec<_> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
                .collect()
        });
        if let Some(image) = pictures.first() {
            let png = encode_png(image.size, &image.pixels);
            if let Err(e) = std::fs::write(&self.path, png) {
                eprintln!("could not write {}: {e}", self.path.display());
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if idle_and_done && !self.requested {
            self.settled += 1;
            if self.settled >= SETTLE_FRAMES {
                self.requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
        }
    }
}

/// A PNG of RGBA pixels, with the image data stored uncompressed (valid and
/// readable everywhere, a little large; this is a developer aid).
pub fn encode_png(size: [usize; 2], pixels: &[egui::Color32]) -> Vec<u8> {
    let [width, height] = size;
    let mut raw = Vec::with_capacity(height.saturating_mul(width.saturating_mul(4) + 1));
    for row in pixels.chunks(width.max(1)).take(height) {
        raw.push(0); // filter: none
        for pixel in row {
            raw.extend_from_slice(&pixel.to_array());
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

    #[test]
    fn checksums_match_the_published_examples() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_png_has_the_right_shape() {
        let pixels = vec![egui::Color32::from_rgb(10, 20, 30); 6];
        let png = encode_png([3, 2], &pixels);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &3u32.to_be_bytes());
        assert_eq!(&png[20..24], &2u32.to_be_bytes());
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]));
    }

    #[test]
    fn an_empty_image_still_encodes() {
        let png = encode_png([0, 0], &[]);
        assert!(png.len() > 40);
    }

    #[test]
    fn large_images_use_several_blocks() {
        let pixels = vec![egui::Color32::WHITE; 300 * 300];
        let png = encode_png([300, 300], &pixels);
        assert!(png.len() > 300 * 300 * 4);
    }
}
