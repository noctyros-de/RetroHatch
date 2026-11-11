// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation)
// Derived from maragotchi/tamaemu/src/pn512.c and periph.c; retain upstream notices.
//! PN512 register/UART controller with disconnected RF. No synthetic peer or DLC.
use crate::core::state::{Decoder, Encoder, require};
#[derive(Clone, Debug, PartialEq)]
pub struct Pn512 {
    pub reg: [u8; 64],
    pub fifo: Vec<u8>,
    pub replies: Vec<u8>,
    pub buf: [u8; 25],
    pub write_address: Option<u8>,
    pub prng: u32,
    pub receiving: bool,
    pub pending: Vec<u8>,
    pub pending_at: u64,
    pub reply_at: u64,
}
impl Pn512 {
    pub fn new() -> Self {
        let mut n = Self {
            reg: [0; 64],
            fifo: Vec::new(),
            replies: Vec::new(),
            buf: [0; 25],
            write_address: None,
            prng: 0x4e464331,
            receiving: false,
            pending: Vec::new(),
            pending_at: 0,
            reply_at: 0,
        };
        n.reset();
        n
    }
    fn reset(&mut self) {
        self.reg = [0; 64];
        for (a, v) in [
            (1, 0x20),
            (0x11, 0x3f),
            (0x14, 0x80),
            (0x15, 0x40),
            (0x18, 0x84),
            (0x19, 0x4d),
            (0x1f, 0xeb),
            (0x26, 0x48),
            (0x27, 0x88),
            (0x37, 0x82),
        ] {
            self.reg[a] = v;
        }
        self.fifo.clear();
        self.write_address = None;
        self.receiving = false;
        self.pending.clear();
        // SoftReset preserves UART replies, memory buffer and PRNG, as upstream.
    }
    fn push(&mut self, v: u8) {
        if self.fifo.len() < 64 {
            self.fifo.push(v)
        } else {
            self.reg[6] |= 0x10;
        }
    }
    fn pop(&mut self) -> u8 {
        if self.fifo.is_empty() {
            0
        } else {
            self.fifo.remove(0)
        }
    }
    fn random(&mut self) -> u8 {
        self.prng ^= self.prng << 13;
        self.prng ^= self.prng >> 17;
        self.prng ^= self.prng << 5;
        self.prng as u8
    }
    fn transmit(&mut self) {
        self.fifo.clear();
        self.reg[4] |= 0x40;
    }
    fn command(&mut self, v: u8, cycles: u64) {
        let mut cmd = v & 15;
        if cmd == 7 {
            cmd = self.reg[1] & 15;
        }
        match cmd {
            0 => {
                self.receiving = false;
                self.pending.clear();
            }
            1 => {
                if self.fifo.is_empty() {
                    for v in self.buf {
                        self.push(v);
                    }
                } else {
                    for i in 0..25 {
                        if self.fifo.is_empty() {
                            break;
                        }
                        self.buf[i] = self.pop();
                    }
                }
                cmd = 0;
            }
            2 => {
                self.prng ^= cycles as u32 | 1;
                for i in 0..10 {
                    self.buf[i] = self.random();
                }
                cmd = 0;
            }
            3 => {
                let mut crc = [0u16, 0x6363, 0xa671, 0xffff][(self.reg[0x11] & 3) as usize];
                for &b in &self.fifo {
                    let mut ch = b ^ crc as u8;
                    ch ^= ch.wrapping_shl(4);
                    crc = (crc >> 8) ^ ((ch as u16) << 8) ^ ((ch as u16) << 3) ^ ((ch as u16) >> 4);
                }
                self.fifo.clear();
                self.reg[0x21] = (crc >> 8) as u8;
                self.reg[0x22] = crc as u8;
                self.reg[5] |= 4;
            }
            4 => {
                self.transmit();
                self.reg[4] |= 0x10;
                cmd = 0;
            }
            8 | 13 => self.receiving = true,
            12 => {
                if self.reg[0x14] & 3 == 0 {
                    self.receiving = true;
                }
            }
            15 => {
                self.reset();
                return;
            }
            _ => {} // Matches upstream reserved commands and authentication without a peer.
        }
        self.reg[1] = (v & 0x30) | cmd;
    }
    fn write(&mut self, a: usize, v: u8, cycles: u64, hz: f64) {
        match a {
            1 => self.command(v, cycles),
            4 | 5 => {
                if v & 0x80 != 0 {
                    self.reg[a] |= v & 0x7f
                } else {
                    self.reg[a] &= !(v & 0x7f)
                }
            }
            9 => self.push(v),
            10 => {
                if v & 0x80 != 0 {
                    self.fifo.clear();
                    self.reg[6] &= !0x10;
                    self.receiving = false;
                }
            }
            12 => {
                if v & 0x40 != 0 {
                    self.reg[4] |= 1;
                }
                if v & 0x20 != 0 {
                    for b in self.buf[..10].to_vec() {
                        self.push(b);
                    }
                }
                self.reg[a] = v & 0x1f;
            }
            13 => {
                self.reg[a] = v & 0x7f;
                if v & 0x80 != 0 && self.reg[1] & 15 == 12 {
                    if self.reg[0x14] & 3 != 0 {
                        self.transmit();
                        self.receiving = true;
                    } else {
                        self.pending = std::mem::take(&mut self.fifo);
                        self.pending_at = cycles
                            + (hz * 1e-6 * (1000.0 + 300.0 * (self.random() & 7) as f64)) as u64;
                        self.receiving = true;
                    }
                }
            }
            6..=8 => {}
            _ => self.reg[a] = v,
        }
    }
    fn read(&mut self, a: usize) -> u8 {
        match a {
            9 => self.pop(),
            10 => self.fifo.len() as u8,
            7 => {
                0x20 | if self.reg[0x21] == 0 && self.reg[0x22] == 0 {
                    0x40
                } else {
                    0
                }
            }
            8 => {
                if matches!(self.reg[1] & 15, 8 | 12) {
                    5
                } else {
                    0
                }
            }
            _ => self.reg[a],
        }
    }
    pub fn host_byte(&mut self, b: u8, cycles: u64, hz: f64) {
        if self.prng == 0 {
            self.reset();
            self.replies.clear();
            self.reply_at = 0;
            self.prng = 0x4e464331;
        }
        let response = if let Some(a) = self.write_address.take() {
            self.write(a as usize, b, cycles, hz);
            Some(a)
        } else if b & 0x80 != 0 {
            Some(self.read((b & 63) as usize))
        } else {
            self.write_address = Some(b & 63);
            None
        };
        if let Some(v) = response {
            if self.replies.len() == 64 {
                self.replies.remove(0);
            }
            self.replies.push(v);
        }
    }
    pub fn tick(&mut self, cycles: u64) {
        if !self.pending.is_empty() && cycles >= self.pending_at {
            self.pending.clear();
            self.reg[4] |= 0x40;
        }
    }
    pub fn encode(&self, e: &mut Encoder) {
        e.bytes(&self.reg);
        e.bytes(&self.buf);
        e.u8(self.write_address.unwrap_or(255));
        e.u32(self.prng);
        e.bool(self.receiving);
        e.u64(self.pending_at);
        e.u64(self.reply_at);
        for v in [&self.fifo, &self.replies, &self.pending] {
            e.u8(v.len() as u8);
            e.bytes(v);
        }
    }
    pub fn decode(r: &mut Decoder<'_>) -> Result<Self, String> {
        let mut n = Self::new();
        n.reg = r.array()?;
        n.buf = r.array()?;
        let a = r.u8()?;
        require(a == 255 || a < 64, "Invalid PN512 write address")?;
        n.write_address = if a == 255 { None } else { Some(a) };
        n.prng = r.u32()?;
        n.receiving = r.bool()?;
        n.pending_at = r.u64()?;
        n.reply_at = r.u64()?;
        for v in [&mut n.fifo, &mut n.replies, &mut n.pending] {
            let count = r.u8()?;
            require(count <= 64, "Invalid PN512 queue")?;
            for _ in 0..count {
                v.push(r.u8()?);
            }
        }
        Ok(n)
    }
}
