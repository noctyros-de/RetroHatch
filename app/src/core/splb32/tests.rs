// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
use super::*;
use crate::core::state::{hex, sha256};
pub fn synthetic() -> Vec<u8> {
    let mut r = vec![0xea; 524288];
    r[0x4800..0x4803].copy_from_slice(&[0x4c, 0, 0xc8]);
    r[0x7ffc..0x7ffe].copy_from_slice(&[0, 0xc8]);
    r
}
#[test]
fn all_reference_opcodes_match_cpu_cycles_flags_and_ram() {
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/splb32.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let f: Vec<_> = line.split_whitespace().collect();
        let v: Vec<i64> = f[..9].iter().map(|s| s.parse().unwrap()).collect();
        let mut r = synthetic();
        r[0x4800..0x4803].copy_from_slice(&[v[0] as u8, 0x40, 0x80]);
        let mut m = Machine::new(&r).unwrap();
        let seed = v[1] as usize;
        m.a = [0, 127, 128, 255][seed];
        m.x = seed as i64;
        m.y = 3 - seed as i64;
        m.sp = [0, 1, 254, 255][seed];
        m.set_ps([0, 255, 9, 0x42][seed]);
        for (i, b) in m.ram.iter_mut().enumerate() {
            *b = ((i * 17 + seed * 63) & 255) as i64;
        }
        let cycles = m.step_cpu().unwrap();
        assert_eq!(
            [cycles, m.pc, m.a, m.x, m.y, m.sp, m.get_ps()],
            v[2..],
            "opcode {:02X} seed {seed}",
            v[0]
        );
        assert_eq!(
            hex(&sha256(&m.ram.map(|v| v as u8))),
            f[9],
            "RAM opcode {:02X}",
            v[0]
        );
        count += 1;
    }
    assert_eq!(count, 604);
}
#[test]
fn reserved_opcode_is_rejected_atomically() {
    let mut r = synthetic();
    r[0x4800] = 2;
    let mut m = Machine::new(&r).unwrap();
    let before = m.save_state().unwrap();
    assert!(m.clock().unwrap_err().contains("Unsupported SPLB32 opcode"));
    assert_eq!(before, m.save_state().unwrap());
}
#[test]
fn ram_lcd_banks_and_eu_pullups_follow_reference() {
    let mut r = synthetic();
    r[0x24000] = 0x35;
    r[0x4000] = 0x42;
    let mut m = Machine::new(&r).unwrap();
    m.write_mem(0, 4);
    assert_eq!(m.read_mem(0x4000), 0x35);
    assert_eq!(m.read_mem(0xc000), 0x42);
    assert_eq!(m.read_mem(9), 6);
    for a in [0x40, 0x4ff, 0x3e00, 0x3fff] {
        m.write_mem(a, 0xa5);
        assert_eq!(m.read_mem(a), 0xa5);
    }
    assert_eq!(m.read_mem(0x500), 0);
    m.write_mem(0x24, 1);
    assert_eq!(m.display()[511], 0xa5);
    m.write_mem(0x0e, 0);
    assert_eq!(m.display(), [0; 512]);
}
#[test]
fn stopped_cpu_keeps_subclock_and_wakes_on_active_high_a() {
    let mut m = Machine::new(&synthetic()).unwrap();
    m.write_mem(0x18, 1);
    m.write_mem(4, 7);
    let t = m.clock().unwrap();
    assert_eq!(t, SUB_DIV);
    assert_eq!(m.instructions, 0);
    assert_ne!(m.read_mem(12), 0);
    m.buttons(1);
    assert_eq!(m.read_mem(8) & 7, 1);
    m.clock().unwrap();
    assert_eq!(m.cpu_enbl, 1);
    assert_eq!(m.cpu_div, 2);
    m.buttons(0);
    assert_eq!(m.read_mem(8) & 7, 0);
    m.clock().unwrap();
    assert_eq!(m.instructions, 1);
}
#[test]
fn timer_overflow_interrupts_and_audio_survive_full_state() {
    let mut a = Machine::new(&synthetic()).unwrap();
    a.write_mem(16, 250);
    a.write_mem(17, 255);
    a.write_mem(15, 0x40);
    a.write_mem(18, 0x48);
    a.write_mem(19, 255);
    a.write_mem(1, 16);
    a.iflag = 0;
    a.advance_cycles(1000.).unwrap();
    assert!(a.interrupts > 0);
    assert!(a.drain_audio().iter().any(|v| *v != 0));
    let mut b = a.restored(&a.save_state().unwrap()).unwrap();
    for t in [2000., 4000., 8000.] {
        a.advance_cycles(t).unwrap();
        b.advance_cycles(t).unwrap();
        assert_eq!(a.save_state().unwrap(), b.save_state().unwrap());
    }
}
#[test]
fn corrupt_foreign_and_malformed_states_do_not_change_the_device() {
    let a = Machine::new(&synthetic()).unwrap();
    let bytes = a.save_state().unwrap();
    let mut b = bytes.clone();
    b[60] ^= 1;
    assert!(a.restored(&b).is_err());
    let mut r = synthetic();
    r[100] ^= 1;
    assert!(Machine::new(&r).unwrap().restored(&bytes).is_err());
    assert_eq!(a.save_state().unwrap(), bytes);
    for n in [0, 1, 31, 80, 1000] {
        assert!(a.restored(&bytes[..n]).is_err());
    }
}
fn start(e: &mut eeprom::Eeprom) {
    e.update(3);
    e.update(1);
}
fn stop(e: &mut eeprom::Eeprom) {
    e.update(0);
    e.update(1);
    e.update(3);
}
fn byte(e: &mut eeprom::Eeprom, v: u8) {
    for b in (0..8).rev() {
        let s = ((v >> b) & 1) * 2;
        e.update(s);
        e.update(s | 1);
    }
    e.update(2);
    assert_eq!(e.update(3), Some(0));
}
#[test]
fn eeprom_block_address_page_wrap_and_sequential_read() {
    let mut e = eeprom::Eeprom::default();
    start(&mut e);
    byte(&mut e, 0xa6);
    byte(&mut e, 0xff);
    byte(&mut e, 0x12);
    byte(&mut e, 0x34);
    stop(&mut e);
    assert_eq!(e.data[1023], 0x12);
    assert_eq!(e.data[1008], 0x34);
    start(&mut e);
    byte(&mut e, 0xa6);
    byte(&mut e, 0xff);
    start(&mut e);
    byte(&mut e, 0xa7);
    let mut v = 0;
    for _ in 0..8 {
        e.update(2);
        v = (v << 1) | u8::from(e.update(3) == Some(-1));
    }
    assert_eq!(v, 0x12);
    e.update(2);
    e.update(3);
    assert_eq!(e.address, 0);
}
#[test]
fn reset_preserves_eeprom_but_initializes_cpu() {
    let mut m = Machine::new(&synthetic()).unwrap();
    m.ee.data[1000] = 43;
    m.advance_cycles(800.).unwrap();
    let time = m.cycles;
    m.a = 77;
    m.reset();
    assert_eq!(m.a, 0);
    assert_eq!(m.pc, 0xc800);
    assert_eq!(m.ee.data[1000], 43);
    assert_eq!(m.cycles, time);
}
#[test]
fn in_flight_eeprom_write_continues_after_state_restore() {
    let mut m = Machine::new(&synthetic()).unwrap();
    start(&mut m.ee);
    byte(&mut m.ee, 0xa0);
    byte(&mut m.ee, 0x4e);
    for b in (4..8).rev() {
        let s = ((0xa5 >> b) & 1) * 2;
        m.ee.update(s);
        m.ee.update(s | 1);
    }
    let mut n = m.restored(&m.save_state().unwrap()).unwrap();
    for e in [&mut m.ee, &mut n.ee] {
        for b in (0..4).rev() {
            let s = ((0xa5 >> b) & 1) * 2;
            e.update(s);
            e.update(s | 1);
        }
        e.update(2);
        e.update(3);
        stop(e);
    }
    assert_eq!(m.ee.data[0x4e], 0xa5);
    assert_eq!(m.save_state().unwrap(), n.save_state().unwrap());
}
#[test]
fn lcd_mapping_is_bijective_and_within_electrical_ram() {
    let mut bits = std::collections::HashSet::new();
    for row in lcd::MATRIX {
        for bit in row {
            assert!(bit < 4096 && bits.insert(bit));
        }
    }
    assert_eq!(bits.len(), 960);
    assert_eq!(lcd::RECTS.len(), 960);
    for &(bit, x, y, w, h) in lcd::RECTS {
        assert!(bits.contains(&bit) && x + w <= 352 && y + h <= 363);
    }
}

#[test]
fn timebase_reads_are_bytes_while_interrupt_counter_keeps_full_phase() {
    let mut m = Machine::new(&synthetic()).unwrap();
    m.t1khz = 0x123;
    m.cycles = 1_200_000.;
    assert_eq!(m.read_mem(12), 0x23);
    assert_eq!(m.t1khz, 0x123);
    let mut r = synthetic();
    r[0x4800..0x4802].copy_from_slice(&[0xa5, 0x0c]);
    let mut cpu = Machine::new(&r).unwrap();
    cpu.t1khz = 0x1ff;
    cpu.step_cpu().unwrap();
    assert_eq!(cpu.a, 255);
    assert_eq!(cpu.nf, 1);
    assert_eq!(cpu.zf, 0);
}
