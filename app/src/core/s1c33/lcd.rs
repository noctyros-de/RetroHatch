// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation and integration)
// Derived from maragotchi/tamaemu; retain upstream GPL notices and provenance.
//! LCD command/data behavior from tamaemu/src/lcd.c; no packaged artwork.
pub const GRAM_W: usize = 132;
pub const GRAM_H: usize = 162;
pub const PANEL_W: usize = 128;
pub const PANEL_H: usize = 128;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lcd {
    pub gram: Vec<u16>,
    pub command: u8,
    pub argc: u32,
    pub args: [u8; 16],
    pub xs: u32,
    pub xe: u32,
    pub ys: u32,
    pub ye: u32,
    pub x: u32,
    pub y: u32,
    pub madctl: u8,
    pub colmod: u8,
    pub sleep_out: bool,
    pub display_on: bool,
    pub inverted: bool,
    pub gamma_positive: [u8; 16],
    pub gamma_negative: [u8; 16],
    pub gamma_counts: [u8; 2],
    pub power_c0: [u8; 2],
    pub power_c2: u8,
    pub vcom_c5: u8,
    pub vcom_c6: u8,
    pub power_counts: [u8; 4],
    pub pending_pixel: [u8; 3],
    pub pending_count: usize,
    pub ramwr_bytes: u64,
    pub commands: u64,
    pub frame_marker: u64,
}
impl Default for Lcd {
    fn default() -> Self {
        Self {
            gram: vec![0; GRAM_W * GRAM_H],
            command: 0,
            argc: 0,
            args: [0; 16],
            xs: 0,
            xe: 0,
            ys: 0,
            ye: 0,
            x: 0,
            y: 0,
            madctl: 0,
            colmod: 0,
            sleep_out: false,
            display_on: false,
            inverted: false,
            gamma_positive: [0; 16],
            gamma_negative: [0; 16],
            gamma_counts: [0; 2],
            power_c0: [0; 2],
            power_c2: 0,
            vcom_c5: 0,
            vcom_c6: 0,
            power_counts: [0; 4],
            pending_pixel: [0; 3],
            pending_count: 0,
            ramwr_bytes: 0,
            commands: 0,
            frame_marker: 0,
        }
    }
}
impl Lcd {
    fn finish_command(&mut self) {
        match self.command {
            0x2a if self.argc >= 4 => {
                self.xs = u16::from_be_bytes([self.args[0], self.args[1]]) as u32;
                self.xe = u16::from_be_bytes([self.args[2], self.args[3]]) as u32;
                self.x = self.xs;
            }
            0x2b if self.argc >= 4 => {
                self.ys = u16::from_be_bytes([self.args[0], self.args[1]]) as u32;
                self.ye = u16::from_be_bytes([self.args[2], self.args[3]]) as u32;
                self.y = self.ys;
            }
            0xe0 if self.argc >= 16 => {
                self.gamma_positive = self.args;
                self.gamma_counts[0] = 16;
            }
            0xe1 if self.argc >= 16 => {
                self.gamma_negative = self.args;
                self.gamma_counts[1] = 16;
            }
            0xc0 if self.argc >= 1 => {
                self.power_c0 = [self.args[0], if self.argc >= 2 { self.args[1] } else { 0 }];
                self.power_counts[0] = self.argc.min(2) as u8;
            }
            0xc2 if self.argc >= 1 => {
                self.power_c2 = self.args[0];
                self.power_counts[1] = 1;
            }
            0xc5 if self.argc >= 1 => {
                self.vcom_c5 = self.args[0];
                self.power_counts[2] = 1;
            }
            0xc6 if self.argc >= 1 => {
                self.vcom_c6 = self.args[0];
                self.power_counts[3] = 1;
            }
            _ => {}
        }
    }
    pub fn write_command(&mut self, command: u8) {
        self.finish_command();
        self.command = command;
        self.argc = 0;
        self.pending_count = 0;
        self.commands = self.commands.wrapping_add(1);
        match command {
            0x01 => {
                self.madctl = 0;
                self.sleep_out = false;
                self.display_on = false;
                self.inverted = false;
                self.xs = 0;
                self.xe = GRAM_W as u32 - 1;
                self.ys = 0;
                self.ye = GRAM_H as u32 - 1;
            }
            0x10 => self.sleep_out = false,
            0x11 => self.sleep_out = true,
            0x20 => self.inverted = false,
            0x21 => self.inverted = true,
            0x28 => self.display_on = false,
            0x29 => {
                self.display_on = true;
                self.frame_marker = self.frame_marker.wrapping_add(1);
            }
            0x2c => {
                self.x = self.xs;
                self.y = self.ys;
            }
            _ => {}
        }
    }
    pub fn write_data(&mut self, data: u8) {
        if self.command == 0x2c {
            self.argc = self.argc.wrapping_add(1);
            self.ramwr_bytes = self.ramwr_bytes.wrapping_add(1);
            self.pending_pixel[self.pending_count] = data;
            self.pending_count += 1;
            if self.pending_count >= if self.colmod & 15 == 5 { 2 } else { 3 } {
                let p = self.pending_pixel;
                let pixel = if self.colmod & 15 == 5 {
                    u16::from_be_bytes([p[0], p[1]])
                } else {
                    ((p[0] as u16 >> 3) << 11) | ((p[1] as u16 >> 2) << 5) | (p[2] as u16 >> 3)
                };
                self.pending_count = 0;
                if self.x < GRAM_W as u32 && self.y < GRAM_H as u32 {
                    self.gram[self.y as usize * GRAM_W + self.x as usize] = pixel;
                }
                self.x = self.x.wrapping_add(1);
                if self.x > self.xe {
                    self.x = self.xs;
                    self.y = self.y.wrapping_add(1);
                    if self.y > self.ye {
                        self.y = self.ys;
                    }
                }
            }
            return;
        }
        if self.argc < 16 {
            self.args[self.argc as usize] = data;
        }
        self.argc = self.argc.wrapping_add(1);
        if self.argc == 1 {
            match self.command {
                0x36 => self.madctl = data,
                0x3a => self.colmod = data,
                _ => {}
            }
        }
    }
    /// Exactly the reference's visible panel, including RGB/BGR, inversion and standby.
    pub fn rgb(&self, stay_awake: bool) -> Vec<u8> {
        let mut out = vec![0; PANEL_W * PANEL_H * 3];
        if (!self.sleep_out || !self.display_on) && !stay_awake {
            return out;
        }
        for y in 0..PANEL_H {
            for x in 0..PANEL_W {
                let mut p = self.gram[y * GRAM_W + x];
                if self.inverted {
                    p = !p;
                }
                let r = (p >> 11) & 31;
                let g = (p >> 5) & 63;
                let b = p & 31;
                let mut r = ((r << 3) | (r >> 2)) as u8;
                let g = ((g << 2) | (g >> 4)) as u8;
                let mut b = ((b << 3) | (b >> 2)) as u8;
                if self.madctl & 8 != 0 {
                    std::mem::swap(&mut r, &mut b);
                }
                out[(y * PANEL_W + x) * 3..(y * PANEL_W + x) * 3 + 3].copy_from_slice(&[r, g, b]);
            }
        }
        out
    }
}
