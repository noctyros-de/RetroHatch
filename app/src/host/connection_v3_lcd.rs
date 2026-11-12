// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Original public replacement symbols; exact user-supplied coverage remains optional.
use crate::core::state::{Decoder, require};
use std::{path::PathBuf, sync::Arc};
#[derive(Clone)]
pub struct Lcd {
    segments: Arc<Vec<Segment>>,
    original_symbols: bool,
}
#[derive(Clone)]
struct Segment {
    ram: usize,
    bit: u8,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    alpha: Vec<u8>,
}
impl Lcd {
    /// Complete public display, with original functional symbols instead of traced artwork.
    pub fn builtin() -> Self {
        let mut segments = Vec::with_capacity(970);
        for &(bit, x, y, w, h) in crate::core::splb32::lcd::RECTS {
            segments.push(Segment {
                ram: (bit / 8) as usize,
                bit: (bit % 8) as u8,
                x: x as usize,
                y: y as usize,
                w: w as usize,
                h: h as usize,
                alpha: vec![255; w as usize * h as usize],
            });
        }
        let data = include_bytes!("../../assets/connection-v3/icons.alpha");
        let bits = [
            (0, 7),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (6, 3),
            (6, 2),
            (6, 1),
            (6, 0),
            (5, 7),
        ];
        for (i, (ram, bit)) in bits.into_iter().enumerate() {
            segments.push(Segment {
                ram,
                bit,
                x: 12 + (i % 5) * 72,
                y: if i < 5 { 12 } else { 310 },
                w: 40,
                h: 40,
                alpha: data[i * 1600..(i + 1) * 1600].to_vec(),
            });
        }
        Self {
            segments: Arc::new(segments),
            original_symbols: false,
        }
    }
    pub fn has_icons(&self) -> bool {
        self.segments.len() == 970
    }
    pub fn load() -> Result<Self, String> {
        let path = std::env::var_os("RETROHATCH_V3_LCD")
            .map(PathBuf::from)
            .or_else(|| {
                super::storage::user_dir()
                    .ok()
                    .map(|p| p.join("display-assets/connection-v3.rhlcd"))
            });
        let bytes = path
            .map(|p| super::storage::read_bounded(&p, 1024 * 1024))
            .transpose()?
            .flatten();
        match bytes {
            Some(b) => Self::decode(&b),
            None => Ok(Self::builtin()),
        }
    }
    pub fn decode(b: &[u8]) -> Result<Self, String> {
        let mut r = Decoder::new(b);
        require(
            &r.array::<8>()? == b"RHV3LCD1" && r.u16()? == 352 && r.u16()? == 363,
            "Unsupported private V3 LCD asset format",
        )?;
        let n = r.u16()?;
        require(
            n == 970,
            "V3 LCD requires 960 matrix pixels and ten symbols",
        )?;
        let mut segments = Vec::new();
        let mut bits = std::collections::HashSet::new();
        for _ in 0..n {
            let ram = r.u16()? as usize;
            let bit = r.u8()?;
            let x = r.u16()? as usize;
            let y = r.u16()? as usize;
            let w = r.u16()? as usize;
            let h = r.u16()? as usize;
            require(
                ram < 512
                    && bit < 8
                    && w > 0
                    && h > 0
                    && x + w <= 352
                    && y + h <= 363
                    && bits.insert((ram, bit)),
                "Invalid V3 LCD segment bounds/mapping",
            )?;
            let count = w * h;
            require(r.at + count <= b.len(), "Truncated LCD segment coverage")?;
            let alpha = b[r.at..r.at + count].to_vec();
            r.at += count;
            segments.push(Segment {
                ram,
                bit,
                x,
                y,
                w,
                h,
                alpha,
            });
        }
        r.finish()?;
        let expected: std::collections::HashSet<_> = crate::core::splb32::lcd::MATRIX
            .iter()
            .flatten()
            .map(|v| ((*v / 8) as usize, (*v % 8) as u8))
            .chain([
                (0, 7),
                (1, 0),
                (1, 1),
                (1, 2),
                (1, 3),
                (6, 3),
                (6, 2),
                (6, 1),
                (6, 0),
                (5, 7),
            ])
            .collect();
        require(
            bits == expected,
            "V3 LCD electrical mapping does not match the reference",
        )?;
        Ok(Self {
            segments: Arc::new(segments),
            original_symbols: true,
        })
    }
    pub fn exact_icons(&self) -> bool {
        self.original_symbols
    }
    pub fn render(&self, lcd: &[u8; 512]) -> Vec<u8> {
        let bg = [215u8, 225, 204];
        let ink = [35u8, 43, 30];
        let mut image = bg.repeat(352 * 363);
        if self.segments.is_empty() {
            for &(bit, x, y, w, h) in crate::core::splb32::lcd::RECTS {
                if lcd[bit as usize / 8] >> (bit % 8) & 1 != 0 {
                    for yy in y..y + h {
                        for xx in x..x + w {
                            let at = (yy as usize * 352 + xx as usize) * 3;
                            image[at..at + 3].copy_from_slice(&ink);
                        }
                    }
                }
            }
        } else {
            for s in self.segments.iter() {
                if lcd[s.ram] >> s.bit & 1 == 0 {
                    continue;
                }
                for yy in 0..s.h {
                    for xx in 0..s.w {
                        let a = s.alpha[yy * s.w + xx] as u32;
                        let at = ((s.y + yy) * 352 + s.x + xx) * 3;
                        for c in 0..3 {
                            image[at + c] =
                                ((bg[c] as u32 * (255 - a) + ink[c] as u32 * a + 127) / 255) as u8;
                        }
                    }
                }
            }
        }
        image
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_only_fallback_renders_actual_bits_without_fabricated_icons() {
        let lcd = Lcd {
            segments: Arc::new(Vec::new()),
            original_symbols: false,
        };
        let mut bits = [0; 512];
        let off = lcd.render(&bits);
        bits[0] = 128;
        assert_eq!(lcd.render(&bits), off);
        let bit = crate::core::splb32::lcd::MATRIX[0][0];
        bits[(bit / 8) as usize] |= 1 << (bit % 8);
        assert_ne!(lcd.render(&bits), off);
        assert_eq!(off.len(), 352 * 363 * 3);
        assert!(!lcd.exact_icons());
    }
    #[test]
    fn private_lcd_pack_requires_all_real_segments_and_valid_bounds() {
        let mut w = crate::core::state::Encoder(Vec::new());
        w.bytes(b"RHV3LCD1");
        w.u16(352);
        w.u16(363);
        w.u16(970);
        let ids = crate::core::splb32::lcd::MATRIX
            .iter()
            .flatten()
            .copied()
            .chain([7, 8, 9, 10, 11, 51, 50, 49, 48, 47]);
        for bit in ids {
            w.u16(bit / 8);
            w.u8((bit % 8) as u8);
            w.u16(0);
            w.u16(0);
            w.u16(1);
            w.u16(1);
            w.u8(255);
        }
        let lcd = Lcd::decode(&w.0).unwrap();
        assert!(lcd.exact_icons());
        let mut bits = [0; 512];
        bits[0] = 128;
        assert_eq!(&lcd.render(&bits)[..3], &[35, 43, 30]);
        let mut invalid = w.0.clone();
        invalid[14..16].copy_from_slice(&600u16.to_le_bytes());
        assert!(Lcd::decode(&invalid).is_err());
        assert!(Lcd::decode(&w.0[..100]).is_err());
    }
}

#[cfg(test)]
mod public_icon_tests {
    use super::*;
    #[test]
    fn public_symbols_are_self_contained_and_follow_every_firmware_bit() {
        let lcd = Lcd::builtin();
        assert!(lcd.has_icons());
        assert!(!lcd.exact_icons());
        let zero = [0; 512];
        let off = lcd.render(&zero);
        let mut unique = std::collections::HashSet::new();
        for segment in lcd.segments.iter() {
            assert!(unique.insert((segment.ram, segment.bit)));
            assert!(segment.x + segment.w <= 352 && segment.y + segment.h <= 363);
        }
        assert_eq!(unique.len(), 970);
        for (i, segment) in lcd.segments.iter().skip(960).enumerate() {
            assert!(segment.alpha.iter().any(|a| *a > 0));
            let mut state = zero;
            state[segment.ram] = 1 << segment.bit;
            let on = lcd.render(&state);
            assert_ne!(on, off, "Missing symbol {i}");
            for y in 0..363 {
                for x in 0..352 {
                    if x < segment.x || x >= segment.x + 40 || y < segment.y || y >= segment.y + 40
                    {
                        let at = (y * 352 + x) * 3;
                        assert_eq!(&on[at..at + 3], &off[at..at + 3]);
                    }
                }
            }
        }
    }
}
