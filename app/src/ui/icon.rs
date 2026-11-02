//! Archeion's icon, drawn in code so no image file or decoder is needed:
//! a white archive box on a blue rounded square. Used for the window and
//! the notification-area icon.

/// Background blue and the box's white.
const BLUE: [u8; 3] = [0x2B, 0x6C, 0xB0];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
/// Samples per pixel side, for smooth edges.
const SAMPLES: u32 = 4;

/// RGBA pixels of the icon, `size` × `size`, rows from the top.
#[must_use]
pub fn rgba(size: u32) -> Vec<u8> {
    let n = usize::try_from(size).unwrap_or(0);
    let mut out = Vec::with_capacity(n * n * 4);
    let step = 1.0 / f64::from(size * SAMPLES);
    for y in 0..size {
        for x in 0..size {
            let (mut white, mut blue) = (0u32, 0u32);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = (f64::from(x * SAMPLES + sx) + 0.5) * step;
                    let py = (f64::from(y * SAMPLES + sy) + 0.5) * step;
                    match shade(px, py) {
                        Shade::White => white += 1,
                        Shade::Blue => blue += 1,
                        Shade::None => {}
                    }
                }
            }
            let covered = white + blue;
            let total = SAMPLES * SAMPLES;
            for c in 0..3 {
                // Average of the covered samples; transparent pixels keep blue.
                let v = (u32::from(WHITE[c]) * white + u32::from(BLUE[c]) * blue)
                    .checked_div(covered)
                    .unwrap_or(u32::from(BLUE[c]));
                out.push(u8::try_from(v).unwrap_or(u8::MAX));
            }
            out.push(u8::try_from(covered * 255 / total).unwrap_or(u8::MAX));
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shade {
    None,
    Blue,
    White,
}

/// What covers the point (`x`, `y`) of the unit square.
fn shade(x: f64, y: f64) -> Shade {
    if !in_rounded_square(x, y, 0.2) {
        return Shade::None;
    }
    let inside = |x0: f64, y0: f64, x1: f64, y1: f64| x >= x0 && x < x1 && y >= y0 && y < y1;
    // Lid, body, and the handle slot cut in the body.
    let lid = inside(0.16, 0.22, 0.84, 0.38);
    let body = inside(0.22, 0.42, 0.78, 0.84);
    let slot = inside(0.38, 0.50, 0.62, 0.58);
    if (lid || body) && !slot { Shade::White } else { Shade::Blue }
}

/// True inside the unit square with corners rounded by `radius`.
fn in_rounded_square(x: f64, y: f64, radius: f64) -> bool {
    let cx = x.clamp(radius, 1.0 - radius);
    let cy = y.clamp(radius, 1.0 - radius);
    (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_have_the_right_size_and_shape() {
        for size in [16, 32, 64] {
            let px = rgba(size);
            let n = usize::try_from(size).unwrap();
            assert_eq!(px.len(), n * n * 4);
            let at = |x: usize, y: usize| &px[(y * n + x) * 4..(y * n + x) * 4 + 4];
            assert_eq!(at(0, 0)[3], 0, "rounded corner is transparent");
            assert_eq!(at(n / 2, n * 5 / 8), &[0xFF, 0xFF, 0xFF, 0xFF], "the box is white");
            assert_eq!(at(n / 2, n / 10)[..3], BLUE, "above the lid is blue");
        }
    }
}
