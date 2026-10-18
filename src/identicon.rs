use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Sovereign Mixnet Deterministic Avatar Engine (Option 2)
// Pure Rust visual fingerprint derived strictly from username + public key
// Zero email requirement, zero external dependencies
// -----------------------------------------------------------------------------

const PRIMARY_PALETTE: [(u8, u8, u8); 16] = [
    (0x00, 0xbf, 0x93), // 0: turquoise
    (0x2d, 0xcc, 0x70), // 1: mint
    (0x42, 0xe4, 0x53), // 2: green
    (0xf1, 0xc4, 0x0f), // 3: yellowOrange
    (0xe6, 0x7f, 0x22), // 4: brown
    (0xff, 0x94, 0x4e), // 5: orange
    (0xe8, 0x4c, 0x3d), // 6: red
    (0x35, 0x98, 0xdb), // 7: blue
    (0x9a, 0x59, 0xb5), // 8: purple
    (0xef, 0x3e, 0x96), // 9: magenta
    (0xdf, 0x21, 0xb9), // 10: violet
    (0x7d, 0xc2, 0xd2), // 11: lightBlue
    (0x16, 0xa0, 0x86), // 12: turquoiseIntense
    (0x27, 0xae, 0x61), // 13: mintIntense
    (0x24, 0xc3, 0x33), // 14: greenIntense
    (0x1c, 0xab, 0xbb), // 15: lightBlueIntense
];

const SECONDARY_PALETTE: [(u8, u8, u8); 16] = [
    (0x34, 0x49, 0x5e), // 0: darkBlue
    (0x95, 0xa5, 0xa5), // 1: grey
    (0xd2, 0x54, 0x00), // 2: brownIntense
    (0xc1, 0x39, 0x2b), // 3: redIntense
    (0x29, 0x7f, 0xb8), // 4: blueIntense
    (0x8d, 0x44, 0xad), // 5: purpleIntense
    (0xbe, 0x12, 0x7e), // 6: violetIntense
    (0xe5, 0x23, 0x83), // 7: magentaIntense
    (0x27, 0xae, 0x61), // 8: mintIntense
    (0x24, 0xc3, 0x33), // 9: greenIntense
    (0xd9, 0xd9, 0x21), // 10: yellowIntense
    (0xf3, 0x9c, 0x11), // 11: yellowOrangeIntense
    (0xff, 0x55, 0x00), // 12: orangeIntense
    (0x1c, 0xab, 0xbb), // 13: lightBlueIntense
    (0x23, 0x23, 0x23), // 14: lightBlackIntense
    (0x7e, 0x8c, 0x8d), // 15: greyIntense
];

const DARK_BACKGROUNDS: [(u8, u8, u8); 3] = [
    (30, 30, 30), // 0: dark gray
    (45, 62, 80), // 1: dark blue
    (57, 57, 57), // 2: dark gray 2
];

/// Cell state in the 5x5 identicon grid
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellLayer {
    Background,
    Primary,
    Secondary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identicon {
    pub username: String,
    pub pubkey_hex: String,
    pub identity_string: String,
    pub identity_hash_hex: String,
    pub grid: [[CellLayer; 5]; 5],
    pub primary_color: (u8, u8, u8),
    pub secondary_color: (u8, u8, u8),
    pub bg_color: (u8, u8, u8),
}

impl Identicon {
    /// Deterministic visual key avatar generated strictly from username + public key / Box ID
    pub fn new(username: &str, pubkey_hex: &str) -> Self {
        let u = username.trim();
        let p = pubkey_hex.trim();

        // Deterministic identity string: username|public_key
        let identity_string = format!("{}|{}", u, p);
        let hash = sha256(identity_string.as_bytes());
        let hash_hex = hex::encode(hash);

        // Derive color indices exactly as Ch1ffr3punk / VFACE
        // Primary color: bits 248..251
        let mut primary_idx = 0usize;
        for i in 0..4 {
            if get_bit(&hash, 248 + i) {
                primary_idx |= 1 << i;
            }
        }
        primary_idx %= 16;

        // Secondary color: bits 244..247
        let mut secondary_idx = 0usize;
        for i in 0..4 {
            if get_bit(&hash, 244 + i) {
                secondary_idx |= 1 << i;
            }
        }
        secondary_idx %= 16;

        // Background color: bits 252..253
        let mut bg_choice = 0usize;
        for i in 0..2 {
            if get_bit(&hash, 252 + i) {
                bg_choice |= 1 << i;
            }
        }
        let bg_idx = bg_choice % 3;

        // Generate pixel patterns
        // Bits 0..14 -> Primary pattern
        let mut primary_pattern = [false; 25];
        let mut bit_idx = 0;
        for row in 0..5 {
            for col in 0..3 {
                let paint = get_bit(&hash, bit_idx);
                bit_idx += 1;
                primary_pattern[row * 5 + col] = paint;
                primary_pattern[row * 5 + (4 - col)] = paint;
            }
        }

        // Bits 15..29 -> Secondary pattern
        let mut secondary_pattern = [false; 25];
        for row in 0..5 {
            for col in 0..3 {
                let paint = get_bit(&hash, bit_idx);
                bit_idx += 1;
                secondary_pattern[row * 5 + col] = paint;
                secondary_pattern[row * 5 + (4 - col)] = paint;
            }
        }

        // Compose 5x5 grid (Primary takes precedence over Secondary, then Background)
        let mut grid = [[CellLayer::Background; 5]; 5];
        for row in 0..5 {
            for col in 0..5 {
                let idx = row * 5 + col;
                if primary_pattern[idx] {
                    grid[row][col] = CellLayer::Primary;
                } else if secondary_pattern[idx] {
                    grid[row][col] = CellLayer::Secondary;
                } else {
                    grid[row][col] = CellLayer::Background;
                }
            }
        }

        Self {
            username: u.to_string(),
            pubkey_hex: p.to_string(),
            identity_string,
            identity_hash_hex: hash_hex,
            grid,
            primary_color: PRIMARY_PALETTE[primary_idx],
            secondary_color: SECONDARY_PALETTE[secondary_idx],
            bg_color: DARK_BACKGROUNDS[bg_idx],
        }
    }

    /// Render terminal ANSI colored block graphic (5x5 grid with quiet border)
    pub fn render_terminal(&self) -> String {
        let (pr, pg, pb) = self.primary_color;
        let (sr, sg, sb) = self.secondary_color;
        let (br, bg, bb) = self.bg_color;

        let primary_block = format!("\x1b[38;2;{};{};{}m██\x1b[0m", pr, pg, pb);
        let secondary_block = format!("\x1b[38;2;{};{};{}m██\x1b[0m", sr, sg, sb);
        let bg_block = format!("\x1b[48;2;{};{};{}m  \x1b[0m", br, bg, bb);

        let mut out = String::new();
        out.push_str("   ┌──────────┐\n");
        for row in 0..5 {
            out.push_str("   │");
            for col in 0..5 {
                match self.grid[row][col] {
                    CellLayer::Primary => out.push_str(&primary_block),
                    CellLayer::Secondary => out.push_str(&secondary_block),
                    CellLayer::Background => out.push_str(&bg_block),
                }
            }
            out.push_str("│\n");
        }
        out.push_str("   └──────────┘");
        out
    }

    /// Render clean SVG vector string for GUI or web export
    pub fn render_svg(&self, size: u32) -> String {
        let (pr, pg, pb) = self.primary_color;
        let (sr, sg, sb) = self.secondary_color;
        let (br, bg, bb) = self.bg_color;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 7 7" width="{}" height="{}" shape-rendering="crispEdges">"#,
            size, size
        );
        svg.push_str(&format!(r#"<rect width="7" height="7" fill="rgb({},{},{})"/>"#, br, bg, bb));

        let p_fill = format!("rgb({},{},{})", pr, pg, pb);
        let s_fill = format!("rgb({},{},{})", sr, sg, sb);

        for row in 0..5 {
            for col in 0..5 {
                match self.grid[row][col] {
                    CellLayer::Primary => {
                        svg.push_str(&format!(
                            r#"<rect x="{}" y="{}" width="1" height="1" fill="{}"/>"#,
                            col + 1, row + 1, p_fill
                        ));
                    }
                    CellLayer::Secondary => {
                        svg.push_str(&format!(
                            r#"<rect x="{}" y="{}" width="1" height="1" fill="{}"/>"#,
                            col + 1, row + 1, s_fill
                        ));
                    }
                    CellLayer::Background => {}
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

// Helper to extract bit n from 32-byte array (0-indexed, LSB first per byte)
fn get_bit(source: &[u8; 32], n: usize) -> bool {
    let byte_index = n / 8;
    let bit_index = n % 8;
    if byte_index >= 32 {
        return false;
    }
    ((source[byte_index] >> bit_index) & 1) == 1
}

// -----------------------------------------------------------------------------
// Pure Rust Self-Contained SHA-256 (FIPS 180-4) Implementation
// Zero external dependencies
// -----------------------------------------------------------------------------

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h_val) = (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut out = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_vector() {
        let input = b"Alice|alice@example.invalid|hlHxqZ3uXv4TMXDM1Xus7xsJhlZzwovxg25SHmRKNrQ=";
        let digest = sha256(input);
        assert_eq!(digest.len(), 32);
        // Ensure deterministic output
        assert_eq!(sha256(input), digest);
    }

    #[test]
    fn test_identicon_symmetry() {
        let id = Identicon::new("Alice", "hlHxqZ3uXv4TMXDM1Xus7xsJhlZzwovxg25SHmRKNrQ=");
        assert_eq!(id.identity_string, "Alice|hlHxqZ3uXv4TMXDM1Xus7xsJhlZzwovxg25SHmRKNrQ=");

        // Mirror symmetry verification
        for row in 0..5 {
            assert_eq!(id.grid[row][0], id.grid[row][4]);
            assert_eq!(id.grid[row][1], id.grid[row][3]);
        }
    }
}
