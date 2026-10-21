//! Bounded SIXEL decoding and cell-row anchored image storage for TerminalVelocity.
use std::sync::Arc;

pub const MAX_SIDE: usize = 4096;
pub const MAX_PIXELS: usize = 2 * 1024 * 1024;
pub const MAX_INPUT: usize = 4 * 1024 * 1024;
const MAX_TILES: usize = 4096;
const MAX_BYTES: usize = 16 * 1024 * 1024;
pub type Palette = [[u8; 4]; 256];

pub struct State {
    pub enabled: bool,
    pub active: Store,
    pub inactive: Store,
    pub pending: Option<(Vec<u8>, u16, u16)>,
    pub colors: Palette,
    pub private_colors: bool,
    pub scrolling: bool,
    pub cell_width: usize,
    pub cell_height: usize,
    pub rejected: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            active: Store::default(),
            inactive: Store::default(),
            pending: None,
            colors: palette(),
            private_colors: true,
            scrolling: true,
            cell_width: 9,
            cell_height: 18,
            rejected: 0,
        }
    }
}
impl State {
    pub fn clear(&mut self) {
        self.active.clear();
        self.inactive.clear();
        self.pending = None;
    }
    pub fn geometry(&mut self, width: usize, height: usize) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) != (self.cell_width, self.cell_height) {
            self.clear();
        }
        self.cell_width = width;
        self.cell_height = height;
    }
}

pub fn palette() -> Palette {
    let mut colors = [[0, 0, 0, 255]; 256];
    let base = [
        [0, 0, 0],
        [51, 51, 204],
        [204, 33, 33],
        [51, 204, 51],
        [204, 51, 204],
        [51, 204, 204],
        [204, 204, 51],
        [135, 135, 135],
        [66, 66, 66],
        [84, 84, 255],
        [255, 84, 84],
        [84, 255, 84],
        [255, 84, 255],
        [84, 255, 255],
        [255, 255, 84],
        [255, 255, 255],
    ];
    for (slot, rgb) in colors.iter_mut().zip(base) {
        *slot = [rgb[0], rgb[1], rgb[2], 255];
    }
    colors
}

#[derive(Debug)]
pub struct Image {
    pub id: u64,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

fn numbers(data: &[u8], offset: &mut usize) -> Result<Vec<usize>, &'static str> {
    let mut result = vec![0usize];
    while let Some(&byte) = data.get(*offset) {
        match byte {
            b'0'..=b'9' => {
                let last = result.last_mut().unwrap();
                *last = last
                    .checked_mul(10)
                    .and_then(|v| v.checked_add((byte - b'0') as usize))
                    .filter(|v| *v <= 1_000_000)
                    .ok_or("numeric limit")?;
            }
            b';' if result.len() < 5 => result.push(0),
            b';' => return Err("parameter limit"),
            _ => break,
        }
        *offset += 1;
    }
    Ok(result)
}

fn dimensions(width: usize, height: usize) -> Result<(), &'static str> {
    if width > MAX_SIDE || height > MAX_SIDE || width.saturating_mul(height) > MAX_PIXELS {
        Err("image limit")
    } else {
        Ok(())
    }
}

/// Decode only after a complete DCS; no file, URL or executable references are accepted.
pub fn decode(
    data: &[u8],
    transparent: bool,
    background: [u8; 4],
    aspect: usize,
    colors: &mut Palette,
) -> Result<Image, &'static str> {
    if data.len() > MAX_INPUT {
        return Err("input limit");
    }
    let fill = if transparent { [0; 4] } else { background };
    let mut rows: Vec<Vec<[u8; 4]>> = Vec::new();
    let (mut x, mut y, mut width, mut height, mut offset, mut color, mut work) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let (mut pan, mut pad) = (aspect.max(1), 1usize);
    while let Some(&byte) = data.get(offset) {
        offset += 1;
        match byte {
            b'"' => {
                let p = numbers(data, &mut offset)?;
                if p.len() != 4 {
                    return Err("raster parameters");
                }
                pan = p[0].max(1);
                pad = p[1].max(1);
                if pan > 10 || pad > 10 {
                    return Err("aspect limit");
                }
                width = width.max(p[2]);
                height = height.max(p[3]);
                dimensions(width, height)?;
            }
            b'#' => {
                let p = numbers(data, &mut offset)?;
                color = p[0];
                if color >= 256 {
                    return Err("palette limit");
                }
                if p.len() == 5 {
                    colors[color] = match p[1] {
                        2 if p[2..].iter().all(|v| *v <= 100) => [
                            (p[2] * 255 / 100) as u8,
                            (p[3] * 255 / 100) as u8,
                            (p[4] * 255 / 100) as u8,
                            255,
                        ],
                        1 if p[2] <= 360 && p[3] <= 100 && p[4] <= 100 => {
                            // DEC hue: blue=0, red=120, green=240.
                            let h = ((p[2] + 240) % 360) as f32 / 60.0;
                            let l = p[3] as f32 / 100.0;
                            let s = p[4] as f32 / 100.0;
                            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
                            let z = c * (1.0 - (h % 2.0 - 1.0).abs());
                            let m = l - c / 2.0;
                            let (r, g, b) = match h as u8 {
                                0 => (c, z, 0.0),
                                1 => (z, c, 0.0),
                                2 => (0.0, c, z),
                                3 => (0.0, z, c),
                                4 => (z, 0.0, c),
                                _ => (c, 0.0, z),
                            };
                            [
                                ((r + m) * 255.0).round() as u8,
                                ((g + m) * 255.0).round() as u8,
                                ((b + m) * 255.0).round() as u8,
                                255,
                            ]
                        }
                        _ => return Err("color parameters"),
                    };
                } else if p.len() != 1 {
                    return Err("palette parameters");
                }
            }
            b'$' => x = 0,
            b'-' => {
                x = 0;
                y = y.checked_add(6).ok_or("height limit")?;
                if y > MAX_SIDE {
                    return Err("height limit");
                }
            }
            b'!' | b'?'..=b'~' => {
                let (count, value) = if byte == b'!' {
                    let p = numbers(data, &mut offset)?;
                    if p.len() != 1 {
                        return Err("repeat parameters");
                    }
                    let value = *data.get(offset).ok_or("missing repeat character")?;
                    offset += 1;
                    if !(b'?'..=b'~').contains(&value) {
                        return Err("repeat character");
                    }
                    (p[0].max(1), value - b'?')
                } else {
                    (1, byte - b'?')
                };
                let end = x.checked_add(count).ok_or("width limit")?;
                // Only set bits extend the raster vertically. The final sixel can
                // contain fewer than six rows; declared raster dimensions still apply.
                let used_rows = (8 - value.leading_zeros()) as usize;
                width = width.max(end);
                height = height.max(y + used_rows.max(1));
                dimensions(width, height)?;
                work = work.checked_add(count * 6).ok_or("work limit")?;
                if work > 16 * 1024 * 1024 {
                    return Err("work limit");
                }
                while rows.len() < y + 6 {
                    rows.push(Vec::new());
                }
                for bit in 0..6 {
                    if value & (1 << bit) != 0 {
                        let row = &mut rows[y + bit];
                        row.resize(end.max(row.len()), fill);
                        rows[y + bit][x..end].fill(colors[color]);
                    }
                }
                x = end;
            }
            0..=32 => {}
            _ => return Err("unsupported sixel byte"),
        }
    }
    if width == 0 || height == 0 {
        return Err("empty image");
    }
    let mut divisor = pan;
    let mut b = pad;
    while b != 0 {
        let r = divisor % b;
        divisor = b;
        b = r;
    }
    pan /= divisor;
    pad /= divisor;
    let (out_width, out_height) = (width * pad, height * pan);
    dimensions(out_width, out_height)?;
    let mut rgba = Vec::with_capacity(out_width * out_height * 4);
    for row in 0..out_height {
        for col in 0..out_width {
            rgba.extend_from_slice(
                &rows
                    .get(row / pan)
                    .and_then(|r| r.get(col / pad))
                    .copied()
                    .unwrap_or(fill),
            );
        }
    }

    Ok(Image {
        id: next_image_id(),
        width: out_width,
        height: out_height,
        rgba,
    })
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub image: Arc<Image>,
    pub line: i32,
    pub x: usize,
    pub source_x: usize,
    pub source_y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Default)]
pub struct Store {
    pub tiles: Vec<Tile>,
}
impl Store {
    pub fn clear(&mut self) {
        self.tiles.clear();
    }
    pub fn insert(
        &mut self,
        image: Arc<Image>,
        line: i32,
        x: usize,
        cell_height: usize,
        screen_lines: usize,
        history: usize,
    ) {
        for source_y in (0..image.height).step_by(cell_height.max(1)) {
            let line = line + (source_y / cell_height.max(1)) as i32;
            if line >= -(history as i32) && line < screen_lines as i32 {
                self.tiles.push(Tile {
                    image: image.clone(),
                    line,
                    x,
                    source_x: 0,
                    source_y,
                    width: image.width,
                    height: cell_height.min(image.height - source_y),
                });
            }
        }
        self.limit();
    }
    fn limit(&mut self) {
        loop {
            let mut seen = std::collections::BTreeSet::new();
            let bytes: usize = self
                .tiles
                .iter()
                .filter(|t| seen.insert(t.image.id))
                .map(|t| t.image.rgba.len())
                .sum();
            if bytes <= MAX_BYTES && seen.len() <= 32 && self.tiles.len() <= MAX_TILES {
                break;
            }
            let Some(oldest) = self.tiles.first().map(|t| t.image.id) else {
                break;
            };
            self.tiles.retain(|t| t.image.id != oldest);
        }
    }
    pub fn erase(&mut self, first: i32, last: i32, left: usize, right: usize) {
        let mut result = Vec::new();
        for tile in self.tiles.drain(..) {
            let end = tile.x + tile.width;
            if tile.line < first || tile.line > last || end <= left || tile.x >= right {
                result.push(tile);
                continue;
            }
            if tile.x < left {
                let mut part = tile.clone();
                part.width = left - tile.x;
                result.push(part);
            }
            if end > right {
                let mut part = tile;
                part.source_x += right - part.x;
                part.width = end - right;
                part.x = right;
                result.push(part);
            }
        }
        self.tiles = result;
        self.limit();
    }
    pub fn scroll(&mut self, start: i32, end: i32, delta: i32, history: usize) {
        for tile in &mut self.tiles {
            if tile.line < end && (tile.line >= start || (start == 0 && delta < 0)) {
                tile.line += delta;
                if start > 0 && tile.line < start {
                    tile.width = 0;
                }
                if delta > 0 && tile.line >= end {
                    tile.width = 0;
                }
            }
        }
        self.tiles
            .retain(|t| t.width > 0 && t.line >= -(history as i32));
    }
}

pub fn next_image_id() -> u64 {
    static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
