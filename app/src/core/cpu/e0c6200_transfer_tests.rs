// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

fn parse_state(value: &str) -> CpuState {
    let data: Vec<_> = value.split(',').map(hex).collect();
    assert_eq!(data.len(), 10);
    let mut state = CpuState {
        pc: data[0],
        npc: data[1],
        a: data[2] as u8,
        b: data[3] as u8,
        ix: data[4],
        iy: data[5],
        sp: data[6] as u8,
        interrupt_delay: data[8] != 0,
        halted: data[9] != 0,
        ..CpuState::default()
    };
    set_flags(&mut state, data[7]);
    state
}

fn apply_changes<const N: usize>(memory: &mut [u8; N], changes: &str) {
    if !changes.is_empty() {
        for item in changes.split(',') {
            let (address, value) = item.split_once(':').unwrap();
            memory[hex(address) as usize] = hex(value) as u8;
        }
    }
}

/// Reconstruct both complete memory arrays; zero-valued writes must also be checked.
pub(super) fn check_reference(data: &str) -> usize {
    let mut cpu = Cpu::new(&vec![0xFFB; PROGRAM_WORDS]).unwrap();
    let mut cases = 0;
    for row in data
        .lines()
        .filter(|row| !row.starts_with('#') && !row.is_empty())
    {
        let fields: Vec<_> = row.split('|').collect();
        assert_eq!(fields.len(), 9);
        let opcode = hex(fields[1]);
        let seed = hex(fields[2]) as usize;
        cpu.state = parse_state(fields[3]);
        cpu.program[cpu.state.pc as usize] = opcode;
        cpu.ram = std::array::from_fn(|i| ((i * 3 + seed) & 15) as u8);
        cpu.lcd = std::array::from_fn(|i| ((i + seed + 4) & 15) as u8);
        apply_changes(&mut cpu.ram, fields[4]);
        let mut ram = cpu.ram;
        let mut lcd = cpu.lcd;
        apply_changes(&mut ram, fields[7]);
        apply_changes(&mut lcd, fields[8]);
        let pc = cpu.state.pc;
        assert_eq!(
            cpu.step(),
            Ok(Step {
                pc,
                opcode,
                cycles: hex(fields[5]) as u8,
            }),
            "{row}"
        );
        assert_eq!(cpu.state, parse_state(fields[6]), "{row}");
        assert_eq!(cpu.ram, ram, "{row}");
        assert_eq!(cpu.lcd, lcd, "{row}");
        cases += 1;
    }
    cases
}

#[test]
fn all_remaining_loads_match_original_registers_memory_flags_and_cycles() {
    assert_eq!(
        check_reference(include_str!(
            "../../../tests/data/e0c6200_loads_reference.txt"
        )),
        76928
    );
}

#[test]
fn byte_load_preflights_both_destinations_and_wraps_within_the_index_page() {
    for (ix, expected_ix, second) in [(0xFF, 0x01, 0), (0x2FF, 0x201, 0x200)] {
        let mut cpu = program(&[0x9AB]);
        cpu.state.ix = ix;
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory(ix), Ok(0xB));
        assert_eq!(cpu.read_memory(second), Ok(0xA));
        assert_eq!(cpu.state.ix, expected_ix);
    }
    for (ix, invalid) in [
        (0xE4F, 0xE50),
        (0xECF, 0xED0),
        (0x300, 0x300),
        (0xF00, 0xF00),
    ] {
        let mut cpu = program(&[0x9AB]);
        cpu.state.ix = ix;
        cpu.state.interrupt_delay = true;
        set_flags(&mut cpu.state, 15);
        let before = cpu.clone();
        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(invalid)));
        assert_eq!(cpu, before);
    }
}

#[test]
fn index_load_reads_through_the_old_pointer_and_preserves_adjacent_nibbles() {
    for (opcode, ix, expected) in [
        (0xE82, 0x2FF, 0xAFF),
        (0xE86, 0x2FF, 0x2AF),
        (0xE8A, 0x2FF, 0x2FA),
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.ix = ix;
        cpu.write_memory(ix, 0xA).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.state.ix, expected);
        assert_eq!(cpu.read_memory(ix), Ok(0xA));
    }
    let mut cpu = program(&[0xEA2]);
    cpu.state.ix = 0x2FF;
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0x2FF), Ok(2));
    assert_eq!(cpu.state.ix, 0x2FF);
}

#[test]
fn new_indirect_load_errors_preserve_registers_memory_flags_and_pending_context() {
    for opcode in [
        0xE82, 0xE93, 0xEA2, 0xEB3, 0xFE2, 0xFF3, 0xFE6, 0xFF7, 0xEE2, 0xEE8, 0xEF3, 0xEFC,
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.ix = 0xF00;
        cpu.state.iy = 0xF00;
        cpu.state.npc = 0x1A00;
        cpu.state.interrupt_delay = true;
        set_flags(&mut cpu.state, 15);
        let before = cpu.clone();
        assert_eq!(
            cpu.step(),
            Err(CpuError::UnsupportedAddress(0xF00)),
            "{opcode:03X}"
        );
        assert_eq!(cpu, before);
    }
}

#[test]
fn post_increment_transfer_uses_old_addresses_and_only_increments_the_selected_index() {
    for (opcode, x, y) in [(0xEEB, 0x200, 0x2FF), (0xEFE, 0x2FF, 0x200)] {
        let mut cpu = program(&[opcode]);
        cpu.state.ix = 0x2FF;
        cpu.state.iy = 0x2FF;
        cpu.write_memory(0x2FF, 9).unwrap();
        cpu.step().unwrap();
        assert_eq!((cpu.state.ix, cpu.state.iy), (x, y));
        assert_eq!(cpu.read_memory(0x2FF), Ok(9));
        assert_eq!(cpu.read_memory(0x200), Ok(0));
    }
}
