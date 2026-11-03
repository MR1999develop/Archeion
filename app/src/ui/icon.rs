//! Archeion's icon, drawn in code so no image file or decoder is needed:
//! on a blue rounded square, an archive box inside a circular arrow that
//! turns back, as the history icon of Windows (going back to an earlier
//! backup). Used for the window, the notification-area icon and the
//! executable.

/// The accent blue of the interface, lighter at the top.
const BLUE_TOP: [f64; 3] = [0x1A as f64, 0x86 as f64, 0xE0 as f64];
const BLUE_BOTTOM: [f64; 3] = [0x00 as f64, 0x5F as f64, 0xB8 as f64];
const WHITE: [f64; 3] = [255.0, 255.0, 255.0];
/// Samples per pixel side, for smooth edges.
const SAMPLES: u32 = 4;

/// RGBA pixels of the icon, `size` × `size`, rows from the top.
#[must_use]
pub fn rgba(size: u32) -> Vec<u8> {
    let n = usize::try_from(size).unwrap_or(0);
    let mut out = Vec::with_capacity(n * n * 4);
    let step = 1.0 / f64::from(size * SAMPLES);
    let total = f64::from(SAMPLES * SAMPLES);
    for y in 0..size {
        for x in 0..size {
            let (mut rgb, mut covered) = ([0.0f64; 3], 0.0f64);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = (f64::from(x * SAMPLES + sx) + 0.5) * step;
                    let py = (f64::from(y * SAMPLES + sy) + 0.5) * step;
                    let colour = match shade(px, py) {
                        Shade::None => continue,
                        Shade::Background => background(py),
                        Shade::Mark => WHITE,
                    };
                    for (sum, c) in rgb.iter_mut().zip(colour) {
                        *sum += c;
                    }
                    covered += 1.0;
                }
            }
            for c in rgb {
                // Average of the covered samples; transparent pixels keep blue.
                let v = if covered > 0.0 { c / covered } else { BLUE_BOTTOM[0] };
                out.push(byte(v));
            }
            out.push(byte(covered / total * 255.0));
        }
    }
    out
}

/// The background at height `y` (0 at the top).
fn background(y: f64) -> [f64; 3] {
    let mut c = [0.0; 3];
    for ((out, top), bottom) in c.iter_mut().zip(BLUE_TOP).zip(BLUE_BOTTOM) {
        *out = top + (bottom - top) * y;
    }
    c
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "rounded and clamped to 0..=255")]
fn byte(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shade {
    None,
    Background,
    Mark,
}

/// Centre of the arrow and the box.
const CX: f64 = 0.5;
const CY: f64 = 0.5;
/// Radius and half thickness of the circular arrow.
const RING: f64 = 0.33;
const RING_HALF: f64 = 0.055;
/// The arrow's opening, at the top right, in degrees counterclockwise
/// from the right: the arrowhead sits at `GAP_FROM`, pointing up, and the
/// circle starts again at `GAP_TO`.
const GAP_FROM: f64 = 15.0;
const GAP_TO: f64 = 80.0;

/// What covers the point (`x`, `y`) of the unit square.
fn shade(x: f64, y: f64) -> Shade {
    if !in_rounded_square(x, y, 0.22) {
        return Shade::None;
    }
    if in_ring(x, y) || in_arrowhead(x, y) || in_box(x, y) { Shade::Mark } else { Shade::Background }
}

/// The circle of the arrow, open at the top right.
fn in_ring(x: f64, y: f64) -> bool {
    let (dx, dy) = (x - CX, CY - y);
    let r = dx.hypot(dy);
    if (r - RING).abs() > RING_HALF {
        return false;
    }
    let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);
    !(GAP_FROM..GAP_TO).contains(&angle)
}

/// The arrowhead at the end of the circle (at `GAP_FROM`), pointing
/// counterclockwise: back in time.
fn in_arrowhead(x: f64, y: f64) -> bool {
    let a = GAP_FROM.to_radians();
    // Across the ring (outwards) and along it (counterclockwise), on screen.
    let (ox, oy) = (a.cos(), -a.sin());
    let (tx, ty) = (-a.sin(), -a.cos());
    let base = (CX + RING * ox, CY + RING * oy);
    let (half, length) = (RING_HALF * 2.5, RING_HALF * 3.2);
    let tip = (base.0 + tx * length, base.1 + ty * length);
    let b1 = (base.0 + ox * half, base.1 + oy * half);
    let b2 = (base.0 - ox * half, base.1 - oy * half);
    in_triangle((x, y), tip, b1, b2)
}

/// The archive box in the middle: a lid, the body, and the handle slot
/// cut in the body.
fn in_box(x: f64, y: f64) -> bool {
    let inside = |x0: f64, y0: f64, x1: f64, y1: f64| x >= x0 && x < x1 && y >= y0 && y < y1;
    let lid = inside(0.335, 0.375, 0.665, 0.455);
    let body = inside(0.365, 0.48, 0.635, 0.645);
    let slot = inside(0.445, 0.525, 0.555, 0.565);
    (lid || body) && !slot
}

fn in_triangle(p: (f64, f64), a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
    let side = |p1: (f64, f64), p2: (f64, f64)| (p.0 - p2.0) * (p1.1 - p2.1) - (p1.0 - p2.0) * (p.1 - p2.1);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

/// True inside the unit square with corners rounded by `radius`.
fn in_rounded_square(x: f64, y: f64, radius: f64) -> bool {
    let cx = x.clamp(radius, 1.0 - radius);
    let cy = y.clamp(radius, 1.0 - radius);
    (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius
}

/// The icon as a Windows `.ico` file, with these sizes (each at most 256),
/// each image a 32-bit bitmap: for the executable.
#[must_use]
#[allow(dead_code, reason = "used by the build script, which embeds it in the executable")]
pub fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = sizes.iter().map(|&s| (s, bitmap(s))).collect();
    let mut out = Vec::new();
    // Header: reserved, type 1 (icon), count.
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&u16::try_from(images.len()).unwrap_or(0).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, data) in &images {
        // 0 stands for 256.
        let side = u8::try_from(*size).unwrap_or(0);
        out.extend_from_slice(&[side, side, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset).unwrap_or(0).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in images {
        out.extend_from_slice(&data);
    }
    out
}

/// One `.ico` image: a BMP header, BGRA rows from the bottom, and the
/// (unused, all zero) AND mask.
#[allow(dead_code, reason = "used by the build script, through `ico`")]
fn bitmap(size: u32) -> Vec<u8> {
    let n = usize::try_from(size).unwrap_or(0);
    let px = rgba(size);
    let mask_row = n.div_ceil(32) * 4;
    let mut out = Vec::with_capacity(40 + n * n * 4 + mask_row * n);
    let i32_le = |v: u32| i32::try_from(v).unwrap_or(0).to_le_bytes();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&i32_le(size));
    // Twice the height: the colour rows, then the mask.
    out.extend_from_slice(&i32_le(size * 2));
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for row in px.chunks(n * 4).rev() {
        for p in row.chunks(4) {
            out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    out.resize(out.len() + mask_row * n, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_have_the_right_size_and_shape() {
        for size in [16, 32, 64, 256] {
            let px = rgba(size);
            let n = usize::try_from(size).unwrap();
            assert_eq!(px.len(), n * n * 4);
            let at = |x: usize, y: usize| &px[(y * n + x) * 4..(y * n + x) * 4 + 4];
            assert_eq!(at(0, 0)[3], 0, "rounded corner is transparent");
            assert_eq!(at(n / 2, n * 6 / 10), &[0xFF, 0xFF, 0xFF, 0xFF], "the box is white");
            assert_eq!(at(n / 2, n * 467 / 1000)[3], 0xFF, "opaque between lid and body");
            assert_ne!(at(n / 2, n * 467 / 1000)[..3], [0xFF; 3], "blue between lid and body");
            assert_eq!(at(n / 2, n / 2 + n * 33 / 100), &[0xFF, 0xFF, 0xFF, 0xFF], "the ring, at the bottom");
        }
    }

    #[test]
    fn the_ico_file_holds_every_size() {
        let ico = ico(&[16, 32, 256]);
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 3, 0]);
        // The third entry stands for 256 with 0 and points past the others.
        let entry = &ico[6 + 32..6 + 48];
        assert_eq!(&entry[..2], &[0, 0]);
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        assert_eq!(offset + len, ico.len());
    }

    /// Writes the icon, as the `.ico` of the executable, to
    /// `target/ui-screens/archeion.ico` to look at.
    #[test]
    #[ignore = "writes an image to target/ui-screens; run on demand"]
    fn icon_preview() {
        let out =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("ui-screens");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("archeion.ico"), ico(&[16, 24, 32, 48, 256])).unwrap();
    }
}
