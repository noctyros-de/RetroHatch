// SPDX-License-Identifier: GPL-3.0-only
// Rust translation of azya52/BrickEmuPy SPLB32.py (CC0-1.0).
// Reproduce with tools/port_splb32_cpu.py. Original reference behavior is retained.
#![allow(unused_parens, unused_mut)]
use super::Machine;
impl Machine {
    pub(super) fn get_ps(&self) -> i64 {
        return (((((((self.nf << 7) | (self.vf << 6)) | (self.bf << 4)) | (self.df << 3))
            | (self.iflag << 2))
            | (self.zf << 1))
            | self.cf);
    }
    pub(super) fn set_ps(&mut self, ps: i64) {
        self.nf = (ps >> 7);
        self.vf = i64::from((ps & 64) > 0);
        self.bf = i64::from((ps & 16) > 0);
        self.df = i64::from((ps & 8) > 0);
        self.iflag = i64::from((ps & 4) > 0);
        self.zf = i64::from((ps & 2) > 0);
        self.cf = (ps & 1);
    }
    pub(super) fn adc(&mut self, operand: i64) {
        let mut a = self.a;
        let mut new_value = ((a + operand) + self.cf);
        if (i64::from(
            (self.df) != 0 && (i64::from((((a & 15) + (operand & 15)) + self.cf) > 9)) != 0,
        )) != 0
        {
            new_value += 6;
        }

        self.vf = ((((!(a ^ operand)) & (a ^ new_value)) >> 7) & 1);
        self.nf = ((new_value >> 7) & 1);
        if (i64::from((self.df) != 0 && (i64::from(new_value > 153)) != 0)) != 0 {
            new_value += 96;
        }

        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        self.a = (new_value & 255);
    }
    pub(super) fn sbc(&mut self, operand: i64) {
        let mut a = self.a;
        let mut new_value = ((a - operand) - i64::from(self.cf == 0));
        if (self.df) != 0 {
            if (i64::from((((a & 15) - (operand & 15)) - i64::from(self.cf == 0)) < 0)) != 0 {
                new_value -= 6;
            }

            if (i64::from(new_value < 0)) != 0 {
                new_value -= 96;
            }
        }

        self.vf = ((((a ^ operand) & (a ^ new_value)) >> 7) & 1);
        self.nf = ((new_value >> 7) & 1);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value >= 0);
        self.a = (new_value & 255);
    }
    pub(super) fn brk(&mut self) -> i64 {
        self.bf = 1;
        self.irq();
        return 7;
    }
    pub(super) fn ora_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 6;
    }
    pub(super) fn ora_zp(&mut self) -> i64 {
        self.a |= self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 3;
    }
    pub(super) fn asl_zp(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut new_value = (self.read_mem(zp) << 1);
        self.write_mem(zp, (new_value & 255));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 5;
    }
    pub(super) fn php(&mut self) -> i64 {
        self.write_mem((self.sp | 256), self.get_ps());
        self.sp = ((self.sp - 1) & 255);
        return 3;
    }
    pub(super) fn ora_imm(&mut self) -> i64 {
        self.a |= self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn asl_a(&mut self) -> i64 {
        let mut new_value = (self.a << 1);
        self.a = (new_value & 255);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = (new_value >> 8);
        return 2;
    }
    pub(super) fn ora_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn asl_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut new_value = (self.read_mem(addr) << 1);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 6;
    }
    pub(super) fn bpl(&mut self) -> i64 {
        if (i64::from(self.nf == 0)) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn ora_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn ora_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 3;
    }
    pub(super) fn asl_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut new_value = (self.read_mem(addr) << 1);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 6;
    }
    pub(super) fn ora_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn and_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 6;
    }
    pub(super) fn ora_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.a |= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn asl_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut new_value = (self.read_mem(addr) << 1);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return (6 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn clc(&mut self) -> i64 {
        self.cf = 0;
        return 2;
    }
    pub(super) fn jsr_abs(&mut self) -> i64 {
        let mut pc = ((self.pc + 1) & 65535);
        self.write_mem((self.sp | 256), (pc >> 8));
        self.sp = ((self.sp - 1) & 255);
        self.write_mem((self.sp | 256), (pc & 255));
        self.sp = ((self.sp - 1) & 255);
        self.pc = (self.read_mem(self.pc) | (self.read_mem(pc) << 8));
        return 6;
    }
    pub(super) fn and_abs_y(&mut self) -> i64 {
        let mut addr = (((self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8))
            + self.y)
            & 65535);
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn and_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn rol_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut new_value = ((self.read_mem(addr) << 1) | self.cf);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return (6 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn bit_zp(&mut self) -> i64 {
        let mut m = self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (m >> 7);
        self.vf = i64::from((m & 64) > 0);
        self.zf = i64::from((self.a & m) == 0);
        return 3;
    }
    pub(super) fn and_zp(&mut self) -> i64 {
        self.a &= self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 3;
    }
    pub(super) fn rol_zp(&mut self) -> i64 {
        let mut opcode = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        let mut new_value = ((self.read_mem(opcode) << 1) | self.cf);
        self.write_mem(opcode, (new_value & 255));
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 5;
    }
    pub(super) fn plp(&mut self) -> i64 {
        self.sp = ((self.sp + 1) & 255);
        self.set_ps(self.read_mem((self.sp | 256)));
        return 4;
    }
    pub(super) fn and_imm(&mut self) -> i64 {
        self.a &= self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn rol_a(&mut self) -> i64 {
        let mut new_value = ((self.a << 1) | self.cf);
        self.a = (new_value & 255);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 2;
    }
    pub(super) fn bit_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut m = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (m >> 7);
        self.vf = i64::from((m & 64) > 0);
        self.zf = i64::from((self.a & m) == 0);
        return 4;
    }
    pub(super) fn and_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn rol_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut new_value = ((self.read_mem(addr) << 1) | self.cf);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 5;
    }
    pub(super) fn bmi(&mut self) -> i64 {
        if (self.nf) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn and_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn and_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.a &= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 6;
    }
    pub(super) fn rol_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut new_value = ((self.read_mem(addr) << 1) | self.cf);
        self.write_mem(addr, (new_value & 255));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = i64::from((new_value & 128) > 0);
        self.zf = i64::from((new_value & 255) == 0);
        self.cf = i64::from(new_value > 255);
        return 6;
    }
    pub(super) fn sec(&mut self) -> i64 {
        self.cf = 1;
        return 2;
    }
    pub(super) fn rti(&mut self) -> i64 {
        self.sp = ((self.sp + 1) & 255);
        self.set_ps(self.read_mem((self.sp | 256)));
        self.sp = ((self.sp + 1) & 255);
        self.pc = self.read_mem((self.sp | 256));
        self.sp = ((self.sp + 1) & 255);
        self.pc |= (self.read_mem((self.sp | 256)) << 8);
        return 6;
    }
    pub(super) fn eor_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn eor_zp(&mut self) -> i64 {
        self.a ^= self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 3;
    }
    pub(super) fn lsr_zp(&mut self) -> i64 {
        let mut addr = self.read_mem(self.pc);
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, (prev_value >> 1));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = 0;
        self.zf = i64::from((prev_value & 254) == 0);
        self.cf = (prev_value & 1);
        return 5;
    }
    pub(super) fn pha(&mut self) -> i64 {
        self.write_mem((self.sp | 256), self.a);
        self.sp = ((self.sp - 1) & 255);
        return 3;
    }
    pub(super) fn eor_imm(&mut self) -> i64 {
        self.a ^= (self.read_mem(self.pc) & 255);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn lsr_a(&mut self) -> i64 {
        let mut prev_value = self.a;
        self.a = (prev_value >> 1);
        self.nf = 0;
        self.zf = i64::from((prev_value & 254) == 0);
        self.cf = (prev_value & 1);
        return 2;
    }
    pub(super) fn jmp_abs(&mut self) -> i64 {
        self.pc = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        return 3;
    }
    pub(super) fn eor_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn lsr_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, (prev_value >> 1));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = 0;
        self.zf = i64::from((prev_value & 254) == 0);
        self.cf = (prev_value & 1);
        return 6;
    }
    pub(super) fn bvc(&mut self) -> i64 {
        if (i64::from(self.vf == 0)) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn eor_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn eor_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn lsr_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, (prev_value >> 1));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = 0;
        self.zf = i64::from((prev_value & 254) == 0);
        self.cf = (prev_value & 1);
        return 6;
    }
    pub(super) fn cli(&mut self) -> i64 {
        self.iflag = 0;
        return 2;
    }
    pub(super) fn eor_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn eor_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.a ^= self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn lsr_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, (prev_value >> 1));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = 0;
        self.zf = i64::from((prev_value & 254) == 0);
        self.cf = (prev_value & 1);
        return (6 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn rts(&mut self) -> i64 {
        self.sp = ((self.sp + 1) & 255);
        self.pc = self.read_mem((self.sp | 256));
        self.sp = ((self.sp + 1) & 255);
        self.pc |= (self.read_mem((self.sp | 256)) << 8);
        self.pc += 1;
        return 6;
    }
    pub(super) fn adc_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return 6;
    }
    pub(super) fn adc_zp(&mut self) -> i64 {
        self.adc(self.read_mem(self.read_mem(self.pc)));
        self.pc = ((self.pc + 1) & 65535);
        return 3;
    }
    pub(super) fn ror_zp(&mut self) -> i64 {
        let mut opcode = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        let mut prev_value = self.read_mem(opcode);
        self.write_mem(opcode, ((prev_value >> 1) | (self.cf << 7)));
        self.nf = self.cf;
        self.zf = i64::from(((prev_value & 254) | self.cf) == 0);
        self.cf = (prev_value & 1);
        return 5;
    }
    pub(super) fn pla(&mut self) -> i64 {
        self.sp = ((self.sp + 1) & 255);
        self.a = self.read_mem((self.sp | 256));
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn adc_imm(&mut self) -> i64 {
        self.adc(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn ror_a(&mut self) -> i64 {
        let mut prev_value = self.a;
        self.a = ((prev_value >> 1) | (self.cf << 7));
        self.nf = self.cf;
        self.zf = i64::from(((prev_value & 254) | self.cf) == 0);
        self.cf = (prev_value & 1);
        return 2;
    }
    pub(super) fn jmp_ind(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.pc = self.read_mem(addr);
        self.pc |= (self.read_mem(((addr + 1) & 65535)) << 8);
        return 6;
    }
    pub(super) fn adc_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return 4;
    }
    pub(super) fn ror_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, ((prev_value >> 1) | (self.cf << 7)));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = self.cf;
        self.zf = i64::from(((prev_value & 254) | self.cf) == 0);
        self.cf = (prev_value & 1);
        return 6;
    }
    pub(super) fn bvs(&mut self) -> i64 {
        if (self.vf) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn adc_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn adc_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return 5;
    }
    pub(super) fn ror_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, ((prev_value >> 1) | (self.cf << 7)));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = self.cf;
        self.zf = i64::from(((prev_value & 254) | self.cf) == 0);
        self.cf = (prev_value & 1);
        return 6;
    }
    pub(super) fn sei(&mut self) -> i64 {
        self.iflag = 1;
        return 2;
    }
    pub(super) fn adc_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn adc_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.adc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn ror_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut prev_value = self.read_mem(addr);
        self.write_mem(addr, ((prev_value >> 1) | (self.cf << 7)));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = self.cf;
        self.zf = i64::from(((prev_value & 254) | self.cf) == 0);
        self.cf = (prev_value & 1);
        return (6 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn sta_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 1) & 65535);
        return 6;
    }
    pub(super) fn sty_zp(&mut self) -> i64 {
        self.write_mem(self.read_mem(self.pc), self.y);
        self.pc = ((self.pc + 1) & 65535);
        return 3;
    }
    pub(super) fn sta_zp(&mut self) -> i64 {
        self.write_mem(self.read_mem(self.pc), self.a);
        self.pc = ((self.pc + 1) & 65535);
        return 3;
    }
    pub(super) fn stx_zp(&mut self) -> i64 {
        self.write_mem(self.read_mem(self.pc), self.x);
        self.pc = ((self.pc + 1) & 65535);
        return 3;
    }
    pub(super) fn dey(&mut self) -> i64 {
        self.y = ((self.y - 1) & 255);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 2;
    }
    pub(super) fn txa(&mut self) -> i64 {
        self.a = self.x;
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn sty_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.write_mem(addr, self.y);
        self.pc = ((self.pc + 2) & 65535);
        return 4;
    }
    pub(super) fn sta_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 2) & 65535);
        return 4;
    }
    pub(super) fn stx_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.write_mem(addr, self.x);
        self.pc = ((self.pc + 2) & 65535);
        return 4;
    }
    pub(super) fn bcc(&mut self) -> i64 {
        if (i64::from(self.cf == 0)) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn sta_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 1) & 65535);
        return 6;
    }
    pub(super) fn sty_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.write_mem(addr, self.y);
        self.pc = ((self.pc + 1) & 65535);
        return 4;
    }
    pub(super) fn sta_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 1) & 65535);
        return 4;
    }
    pub(super) fn stx_zp_y(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.y) & 255);
        self.write_mem(addr, self.x);
        self.pc = ((self.pc + 1) & 65535);
        return 4;
    }
    pub(super) fn tya(&mut self) -> i64 {
        self.a = self.y;
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn sta_abs_y(&mut self) -> i64 {
        let mut addr = (((self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8))
            + self.y)
            & 65535);
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 2) & 65535);
        return 5;
    }
    pub(super) fn txs(&mut self) -> i64 {
        self.sp = self.x;
        return 2;
    }
    pub(super) fn sta_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.write_mem(addr, self.a);
        self.pc = ((self.pc + 2) & 65535);
        return 5;
    }
    pub(super) fn ldy_imm(&mut self) -> i64 {
        self.y = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 2;
    }
    pub(super) fn lda_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 6;
    }
    pub(super) fn ldx_imm(&mut self) -> i64 {
        self.x = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 2;
    }
    pub(super) fn ldy_zp(&mut self) -> i64 {
        self.y = self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 3;
    }
    pub(super) fn lda_zp(&mut self) -> i64 {
        self.a = self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 3;
    }
    pub(super) fn ldx_zp(&mut self) -> i64 {
        self.x = self.read_mem(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 3;
    }
    pub(super) fn tay(&mut self) -> i64 {
        self.y = self.a;
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 2;
    }
    pub(super) fn lda_imm(&mut self) -> i64 {
        self.a = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 2;
    }
    pub(super) fn tax(&mut self) -> i64 {
        self.x = self.a;
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 2;
    }
    pub(super) fn ldy_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.y = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 4;
    }
    pub(super) fn lda_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn ldx_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.x = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 4;
    }
    pub(super) fn bcs(&mut self) -> i64 {
        if (self.cf) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn lda_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn ldy_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.y = self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 4;
    }
    pub(super) fn lda_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return 4;
    }
    pub(super) fn ldx_zp_y(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.y) & 255);
        self.x = self.read_mem(addr);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 4;
    }
    pub(super) fn clv(&mut self) -> i64 {
        self.vf = 0;
        return 2;
    }
    pub(super) fn lda_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn tsx(&mut self) -> i64 {
        self.x = self.sp;
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 2;
    }
    pub(super) fn ldy_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.y = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn lda_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.a = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.a >> 7);
        self.zf = i64::from(self.a == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn ldx_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.x = self.read_mem(addr);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn cpy_imm(&mut self) -> i64 {
        let mut test_value = (self.y - self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 2;
    }
    pub(super) fn cmp_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 6;
    }
    pub(super) fn cpy_zp(&mut self) -> i64 {
        let mut test_value = (self.y - self.read_mem(self.read_mem(self.pc)));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 3;
    }
    pub(super) fn cmp_zp(&mut self) -> i64 {
        let mut test_value = (self.a - self.read_mem(self.read_mem(self.pc)));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 3;
    }
    pub(super) fn dec_zp(&mut self) -> i64 {
        let mut opcode = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        let mut new_value = ((self.read_mem(opcode) - 1) & 255);
        self.write_mem(opcode, new_value);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 5;
    }
    pub(super) fn iny(&mut self) -> i64 {
        self.y = ((self.y + 1) & 255);
        self.nf = (self.y >> 7);
        self.zf = i64::from(self.y == 0);
        return 2;
    }
    pub(super) fn cmp_imm(&mut self) -> i64 {
        let mut test_value = (self.a - self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 2;
    }
    pub(super) fn cpy_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut test_value = (self.y - self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 3;
    }
    pub(super) fn cmp_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 4;
    }
    pub(super) fn dec_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut new_value = ((self.read_mem(addr) - 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 5;
    }
    pub(super) fn dex(&mut self) -> i64 {
        self.x = ((self.x - 1) & 255);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 2;
    }
    pub(super) fn bne(&mut self) -> i64 {
        if (i64::from(self.zf == 0)) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn cmp_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return (5 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn cmp_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 4;
    }
    pub(super) fn dec_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut new_value = ((self.read_mem(addr) - 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 6;
    }
    pub(super) fn cld(&mut self) -> i64 {
        self.df = 0;
        return 2;
    }
    pub(super) fn cmp_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn cmp_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut test_value = (self.a - self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn dec_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut new_value = ((self.read_mem(addr) - 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 7;
    }
    pub(super) fn cpx_imm(&mut self) -> i64 {
        let mut test_value = (self.x - self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 2;
    }
    pub(super) fn sbc_ind_x(&mut self) -> i64 {
        let mut zp = (self.read_mem(self.pc) + self.x);
        let mut addr = (self.read_mem((zp & 255)) | (self.read_mem(((zp + 1) & 255)) << 8));
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return 6;
    }
    pub(super) fn cpx_zp(&mut self) -> i64 {
        let mut test_value = (self.x - self.read_mem(self.read_mem(self.pc)));
        self.pc = ((self.pc + 1) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 3;
    }
    pub(super) fn sbc_zp(&mut self) -> i64 {
        self.sbc(self.read_mem(self.read_mem(self.pc)));
        self.pc = ((self.pc + 1) & 65535);
        return 3;
    }
    pub(super) fn inc_zp(&mut self) -> i64 {
        let mut opcode = self.read_mem(self.pc);
        self.pc = ((self.pc + 1) & 65535);
        let mut new_value = ((self.read_mem(opcode) + 1) & 255);
        self.write_mem(opcode, new_value);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 5;
    }
    pub(super) fn inx(&mut self) -> i64 {
        self.x = ((self.x + 1) & 255);
        self.nf = (self.x >> 7);
        self.zf = i64::from(self.x == 0);
        return 2;
    }
    pub(super) fn sbc_imm(&mut self) -> i64 {
        self.sbc(self.read_mem(self.pc));
        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn cpx_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut test_value = (self.x - self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        self.nf = ((test_value >> 7) & 1);
        self.zf = i64::from(test_value == 0);
        self.cf = i64::from(test_value >= 0);
        return 4;
    }
    pub(super) fn sbc_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return 4;
    }
    pub(super) fn inc_abs(&mut self) -> i64 {
        let mut addr = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut new_value = ((self.read_mem(addr) + 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 6;
    }
    pub(super) fn nop(&mut self) -> i64 {
        return 2;
    }
    pub(super) fn beq(&mut self) -> i64 {
        if (self.zf) != 0 {
            let mut rel = self.read_mem(self.pc);
            let mut prev_pc = ((self.pc + 1) & 65535);
            self.pc = (((prev_pc + rel) - ((rel & 128) << 1)) & 65535);
            return (3 + i64::from((self.pc ^ prev_pc) > 255));
        }

        self.pc = ((self.pc + 1) & 65535);
        return 2;
    }
    pub(super) fn sbc_ind_y(&mut self) -> i64 {
        let mut zp = self.read_mem(self.pc);
        let mut base = (self.read_mem(zp) | (self.read_mem(((zp + 1) & 255)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn sbc_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 1) & 65535);
        return 4;
    }
    pub(super) fn inc_zp_x(&mut self) -> i64 {
        let mut addr = ((self.read_mem(self.pc) + self.x) & 255);
        let mut new_value = ((self.read_mem(addr) + 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 1) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 6;
    }
    pub(super) fn sed(&mut self) -> i64 {
        self.df = 1;
        return 2;
    }
    pub(super) fn sbc_abs_y(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.y) & 65535);
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn sbc_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        self.sbc(self.read_mem(addr));
        self.pc = ((self.pc + 2) & 65535);
        return (4 + i64::from((base ^ addr) > 255));
    }
    pub(super) fn inc_abs_x(&mut self) -> i64 {
        let mut base = (self.read_mem(self.pc) | (self.read_mem(((self.pc + 1) & 65535)) << 8));
        let mut addr = ((base + self.x) & 65535);
        let mut new_value = ((self.read_mem(addr) + 1) & 255);
        self.write_mem(addr, new_value);
        self.pc = ((self.pc + 2) & 65535);
        self.nf = (new_value >> 7);
        self.zf = i64::from(new_value == 0);
        return 7;
    }
    pub(super) fn irq(&mut self) {
        self.write_mem((self.sp | 256), (self.pc >> 8));
        self.sp = ((self.sp - 1) & 255);
        self.write_mem((self.sp | 256), (self.pc & 255));
        self.sp = ((self.sp - 1) & 255);
        self.write_mem((self.sp | 256), self.get_ps());
        self.sp = ((self.sp - 1) & 255);
        self.iflag = 1;
        self.pc = self.addr_irq;
    }
    pub(super) fn nmi(&mut self) {
        if (self.nmi_ctrl & 128) != 0 {
            if (self.cpu_enbl) != 0 {
                self.write_mem((self.sp | 256), (self.pc >> 8));
                self.sp = ((self.sp - 1) & 255);
                self.write_mem((self.sp | 256), (self.pc & 255));
                self.sp = ((self.sp - 1) & 255);
                self.write_mem((self.sp | 256), self.get_ps());
                self.sp = ((self.sp - 1) & 255);
                self.pc = self.addr_nmi;
            }
        }
    }
    pub fn step_cpu(&mut self) -> Result<i64, String> {
        let opcode = self.read_mem(self.pc);
        match opcode {
            0 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.brk())
            }
            1 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_ind_x())
            }
            5 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_zp())
            }
            6 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.asl_zp())
            }
            8 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.php())
            }
            9 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_imm())
            }
            10 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.asl_a())
            }
            13 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_abs())
            }
            14 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.asl_abs())
            }
            16 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bpl())
            }
            17 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_ind_y())
            }
            21 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_zp_x())
            }
            22 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.asl_zp_x())
            }
            24 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.clc())
            }
            25 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_abs_y())
            }
            29 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ora_abs_x())
            }
            30 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.asl_abs_x())
            }
            32 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.jsr_abs())
            }
            33 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_ind_x())
            }
            36 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bit_zp())
            }
            37 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_zp())
            }
            38 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rol_zp())
            }
            40 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.plp())
            }
            41 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_imm())
            }
            42 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rol_a())
            }
            44 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bit_abs())
            }
            45 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_abs())
            }
            46 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rol_abs())
            }
            48 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bmi())
            }
            49 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_ind_y())
            }
            53 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_zp_x())
            }
            54 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rol_zp_x())
            }
            56 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sec())
            }
            57 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_abs_y())
            }
            61 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.and_abs_x())
            }
            62 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rol_abs_x())
            }
            64 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rti())
            }
            65 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_ind_x())
            }
            69 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_zp())
            }
            70 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lsr_zp())
            }
            72 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.pha())
            }
            73 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_imm())
            }
            74 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lsr_a())
            }
            76 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.jmp_abs())
            }
            77 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_abs())
            }
            78 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lsr_abs())
            }
            80 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bvc())
            }
            81 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_ind_y())
            }
            85 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_zp_x())
            }
            86 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lsr_zp_x())
            }
            88 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cli())
            }
            89 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_abs_y())
            }
            93 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.eor_abs_x())
            }
            94 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lsr_abs_x())
            }
            96 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.rts())
            }
            97 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_ind_x())
            }
            101 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_zp())
            }
            102 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ror_zp())
            }
            104 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.pla())
            }
            105 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_imm())
            }
            106 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ror_a())
            }
            108 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.jmp_ind())
            }
            109 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_abs())
            }
            110 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ror_abs())
            }
            112 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bvs())
            }
            113 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_ind_y())
            }
            117 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_zp_x())
            }
            118 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ror_zp_x())
            }
            120 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sei())
            }
            121 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_abs_y())
            }
            125 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.adc_abs_x())
            }
            126 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ror_abs_x())
            }
            129 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_ind_x())
            }
            132 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sty_zp())
            }
            133 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_zp())
            }
            134 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.stx_zp())
            }
            136 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dey())
            }
            138 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.txa())
            }
            140 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sty_abs())
            }
            141 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_abs())
            }
            142 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.stx_abs())
            }
            144 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bcc())
            }
            145 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_ind_y())
            }
            148 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sty_zp_x())
            }
            149 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_zp_x())
            }
            150 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.stx_zp_y())
            }
            152 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.tya())
            }
            153 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_abs_y())
            }
            154 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.txs())
            }
            157 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sta_abs_x())
            }
            160 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldy_imm())
            }
            161 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_ind_x())
            }
            162 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldx_imm())
            }
            164 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldy_zp())
            }
            165 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_zp())
            }
            166 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldx_zp())
            }
            168 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.tay())
            }
            169 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_imm())
            }
            170 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.tax())
            }
            172 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldy_abs())
            }
            173 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_abs())
            }
            174 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldx_abs())
            }
            176 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bcs())
            }
            177 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_ind_y())
            }
            180 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldy_zp_x())
            }
            181 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_zp_x())
            }
            182 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldx_zp_y())
            }
            184 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.clv())
            }
            185 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_abs_y())
            }
            186 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.tsx())
            }
            188 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldy_abs_x())
            }
            189 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.lda_abs_x())
            }
            190 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.ldx_abs_y())
            }
            192 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpy_imm())
            }
            193 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_ind_x())
            }
            196 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpy_zp())
            }
            197 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_zp())
            }
            198 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dec_zp())
            }
            200 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.iny())
            }
            201 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_imm())
            }
            202 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dex())
            }
            204 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpy_abs())
            }
            205 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_abs())
            }
            206 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dec_abs())
            }
            208 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.bne())
            }
            209 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_ind_y())
            }
            213 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_zp_x())
            }
            214 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dec_zp_x())
            }
            216 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cld())
            }
            217 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_abs_y())
            }
            221 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cmp_abs_x())
            }
            222 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.dec_abs_x())
            }
            224 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpx_imm())
            }
            225 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_ind_x())
            }
            228 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpx_zp())
            }
            229 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_zp())
            }
            230 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.inc_zp())
            }
            232 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.inx())
            }
            233 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_imm())
            }
            234 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.nop())
            }
            236 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.cpx_abs())
            }
            237 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_abs())
            }
            238 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.inc_abs())
            }
            240 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.beq())
            }
            241 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_ind_y())
            }
            245 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_zp_x())
            }
            246 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.inc_zp_x())
            }
            248 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sed())
            }
            249 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_abs_y())
            }
            253 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.sbc_abs_x())
            }
            254 => {
                self.pc = (self.pc + 1) & 65535;
                Ok(self.inc_abs_x())
            }
            _ => Err(format!(
                "Unsupported SPLB32 opcode {opcode:02X} at {:05X}",
                self.physical_pc()
            )),
        }
    }
}
