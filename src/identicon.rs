use serde::{Deserialize, Serialize};

/// Deterministic 5x5 Bilateral Symmetric Identicon Generator
/// Generates unique visual avatars from (username + recipient/public key).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identicon {
    pub username: String,
    pub pubkey_hex: String,
    pub grid: [[bool; 5]; 5],
    pub fg_color: (u8, u8, u8),
    pub bg_color: (u8, u8, u8),
}

impl Identicon {
    /// Generate a deterministic identicon from username and public key / Box ID
    pub fn new(username: &str, pubkey_hex: &str) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key("katzen-profile-identicon-v1");
        hasher.update(username.trim().as_bytes());
        hasher.update(pubkey_hex.trim().as_bytes());
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();

        // 5 rows x 3 columns = 15 bits for half-grid (bilateral symmetry)
        let mut grid = [[false; 5]; 5];
        for row in 0..5 {
            for col in 0..3 {
                let bit_index = row * 3 + col;
                let byte_idx = bit_index / 8;
                let bit_pos = bit_index % 8;
                let is_filled = ((bytes[byte_idx] >> bit_pos) & 1) == 1;

                grid[row][col] = is_filled;
                grid[row][4 - col] = is_filled; // Mirror symmetry
            }
        }

        // Derive pleasant foreground color:
        // Use HSL-like derivation from bytes[12..14] to ensure vibrant, visible avatars
        let h = (bytes[12] as f32) / 255.0; // Hue: 0.0 .. 1.0
        let s = 0.65 + ((bytes[13] as f32) / 255.0) * 0.30; // Saturation: 65% - 95%
        let l = 0.50 + ((bytes[14] as f32) / 255.0) * 0.25; // Lightness: 50% - 75%
        let fg_color = hsl_to_rgb(h, s, l);

        let bg_color = (22, 26, 36); // #161a24 deep dark slate

        Self {
            username: username.to_string(),
            pubkey_hex: pubkey_hex.to_string(),
            grid,
            fg_color,
            bg_color,
        }
    }

    /// Render terminal ANSI colored block graphic (5x5 grid with quiet border)
    pub fn render_terminal(&self) -> String {
        let (fr, fg, fb) = self.fg_color;
        let (br, bg, bb) = self.bg_color;

        let fg_block = format!("\x1b[38;2;{};{};{}m██\x1b[0m", fr, fg, fb);
        let bg_block = format!("\x1b[38;2;{};{};{}m  \x1b[0m", br, bg, bb);

        let mut out = String::new();
        out.push_str("   ┌──────────┐\n");
        for row in 0..5 {
            out.push_str("   │");
            for col in 0..5 {
                if self.grid[row][col] {
                    out.push_str(&fg_block);
                } else {
                    out.push_str(&bg_block);
                }
            }
            out.push_str("│\n");
        }
        out.push_str("   └──────────┘");
        out
    }

    /// Render clean SVG vector string for GUI or web display
    pub fn render_svg(&self, size: u32) -> String {
        let (fr, fg, fb) = self.fg_color;
        let (br, bg, bb) = self.bg_color;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 7 7" width="{}" height="{}" shape-rendering="crispEdges">"#,
            size, size
        );
        svg.push_str(&format!(r#"<rect width="7" height="7" fill="rgb({},{},{})"/>"#, br, bg, bb));

        let fill = format!("rgb({},{},{})", fr, fg, fb);
        for row in 0..5 {
            for col in 0..5 {
                if self.grid[row][col] {
                    svg.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="1" height="1" fill="{}"/>"#,
                        col + 1,
                        row + 1,
                        fill
                    ));
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

/// Convert HSL (0..1) to RGB (0..255)
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s == 0.0 {
        let val = (l * 255.0).round() as u8;
        return (val, val, val);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identicon_determinism_and_symmetry() {
        let id1 = Identicon::new("alice", "a1b2c3d4e5f678901234567890abcdef1234567890abcdef1234567890abcdef");
        let id2 = Identicon::new("alice", "a1b2c3d4e5f678901234567890abcdef1234567890abcdef1234567890abcdef");

        // Identical input must produce identical grid and color
        assert_eq!(id1.grid, id2.grid);
        assert_eq!(id1.fg_color, id2.fg_color);

        // Check horizontal mirror symmetry for all rows
        for row in 0..5 {
            assert_eq!(id1.grid[row][0], id1.grid[row][4]);
            assert_eq!(id1.grid[row][1], id1.grid[row][3]);
        }
    }

    #[test]
    fn test_different_recipient_produces_distinct_identicon() {
        let id_alice = Identicon::new("alice", "1111111111111111111111111111111111111111111111111111111111111111");
        let id_bob = Identicon::new("bob", "2222222222222222222222222222222222222222222222222222222222222222");

        assert_ne!(id_alice.grid, id_bob.grid);
        assert_ne!(id_alice.fg_color, id_bob.fg_color);
    }
}
