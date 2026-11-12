// SPDX-License-Identifier: GPL-3.0-only
// Rust port of BrickEmuPy peripherals/HT24LC08.py (CC0-1.0), by azya52.
#[derive(Clone)]
pub struct Eeprom {
    pub data: [u8; 1024],
    pub state: u8,
    pub bits: u8,
    pub address: usize,
    pub scl: u8,
    pub sda: u8,
    pub ack: bool,
    pub rx: u8,
    pub tx: u8,
}
impl Default for Eeprom {
    fn default() -> Self {
        Self {
            data: [0; 1024],
            state: 0,
            bits: 0,
            address: 0,
            scl: 1,
            sda: 1,
            ack: false,
            rx: 0,
            tx: 0,
        }
    }
}
impl Eeprom {
    pub fn update(&mut self, mask: u8) -> Option<i64> {
        let prev_scl = self.scl;
        let prev_sda = self.sda;
        self.scl = mask & 1;
        self.sda = mask & 2;
        if self.scl != 0 && (prev_scl != self.scl || prev_sda != self.sda) {
            if prev_sda > self.sda {
                self.state = 1;
                self.bits = 0;
                self.ack = false;
                return Some(-1);
            }
            if prev_sda < self.sda {
                self.state = 0;
                self.ack = false;
                return None;
            }
            if self.ack {
                self.ack = false;
                return Some(0);
            }
            if self.state == 4 {
                if self.bits < 8 {
                    let level = if self.tx >> (7 - self.bits) & 1 != 0 {
                        -1
                    } else {
                        0
                    };
                    self.bits += 1;
                    return Some(level);
                }
                self.address = (self.address + 1) % 1024;
                self.tx = self.data[self.address];
                self.bits = 0;
                return None;
            }
            self.rx = (self.rx << 1) | u8::from(self.sda != 0);
            self.bits += 1;
            if self.bits == 8 {
                let rx = self.rx;
                self.bits = 0;
                match self.state {
                    1 => {
                        if rx & 0xf8 == 0xa0 {
                            self.ack = true;
                            self.address = ((rx as usize & 7) >> 1) * 256 + self.address % 256;
                            if rx & 1 != 0 {
                                self.state = 4;
                                self.tx = self.data[self.address];
                            } else {
                                self.state = 2;
                            }
                        } else {
                            self.state = 0;
                        }
                    }
                    2 => {
                        self.ack = true;
                        self.address = self.address / 256 * 256 + rx as usize;
                        self.state = 3;
                    }
                    3 => {
                        self.ack = true;
                        self.data[self.address] = rx;
                        self.address = self.address / 16 * 16 + (self.address + 1) % 16;
                    }
                    _ => {}
                }
            }
            return Some(-1);
        }
        None
    }
}
