// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

#[test]
fn calls_and_returns_match_original_opcodes_stack_layout_banks_flags_and_store_aliases() {
    assert_eq!(
        transfers::check_reference(include_str!(
            "../../../tests/data/e0c6200_subroutines_reference.txt"
        )),
        92160
    );
}

#[test]
fn all_subroutine_opcode_fields_decode_without_truncating_the_immediate() {
    for value in 0..256 {
        assert_eq!(
            Instruction::decode(0x100 | value),
            Some(Instruction::ReturnData(value as u8))
        );
        assert_eq!(
            Instruction::decode(0x400 | value),
            Some(Instruction::Call(value as u8))
        );
        assert_eq!(
            Instruction::decode(0x500 | value),
            Some(Instruction::CallZero(value as u8))
        );
    }
    assert_eq!(Instruction::decode(0xFDE), Some(Instruction::ReturnSkip));
    assert_eq!(Instruction::decode(0xFDF), Some(Instruction::Return));
}

#[test]
fn calls_store_page_high_low_nibbles_with_individual_eight_bit_wrapping() {
    for sp in [0u8, 1, 2, 3, 0xFE, 0xFF] {
        let mut cpu = Cpu::new(&vec![0xFFB; PROGRAM_WORDS]).unwrap();
        cpu.state.pc = 0x12FE;
        cpu.state.npc = 0x1A55;
        cpu.state.sp = sp;
        cpu.program[0x12FE] = 0x4AB;
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory(u16::from(sp.wrapping_sub(1))), Ok(2));
        assert_eq!(cpu.read_memory(u16::from(sp.wrapping_sub(2))), Ok(0xF));
        assert_eq!(cpu.read_memory(u16::from(sp.wrapping_sub(3))), Ok(0xF));
        assert_eq!(cpu.state.sp, sp.wrapping_sub(3));
        assert_eq!((cpu.state.pc, cpu.state.npc), (0x1AAB, 0x1A55));
    }
}

#[test]
fn returns_read_beyond_ff_without_restoring_the_program_bank_or_clearing_stack_memory() {
    for sp in [0xFE, 0xFF] {
        let mut cpu = program(&[0xFDF]);
        cpu.state.sp = sp;
        cpu.state.pc = 0x100;
        cpu.program[0x100] = 0xFDF;
        cpu.write_memory(u16::from(sp), 3).unwrap();
        cpu.write_memory(u16::from(sp) + 1, 2).unwrap();
        cpu.write_memory(u16::from(sp) + 2, 1).unwrap();
        cpu.write_memory(0, 0xF).unwrap();
        cpu.write_memory(1, 0xF).unwrap();
        let memory = cpu.ram;
        cpu.step().unwrap();
        assert_eq!(cpu.state.pc, 0x123);
        assert_eq!(cpu.state.npc, 0x123);
        assert_eq!(cpu.state.sp, sp.wrapping_add(3));
        assert_eq!(cpu.ram, memory);
    }
    let mut cpu = Cpu::new(&vec![0xFFB; PROGRAM_WORDS]).unwrap();
    cpu.state.pc = 0x1100;
    cpu.program[0x1100] = 0xFDF;
    cpu.write_memory(0, 3).unwrap();
    cpu.write_memory(1, 2).unwrap();
    cpu.write_memory(2, 1).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state.pc, 0x1123);
}

#[test]
fn retd_captures_stack_address_before_overlapping_stores_and_preflights_every_write() {
    let mut cpu = program(&[0x1AB]);
    cpu.state.ix = 0;
    cpu.write_memory(0, 3).unwrap();
    cpu.write_memory(1, 2).unwrap();
    cpu.write_memory(2, 1).unwrap();
    assert_eq!(cpu.step().unwrap().cycles, 12);
    assert_eq!(
        (cpu.state.pc, cpu.state.npc, cpu.state.sp, cpu.state.ix),
        (0x123, 0x123, 3, 2)
    );
    assert_eq!(cpu.read_memory(0), Ok(0xB));
    assert_eq!(cpu.read_memory(1), Ok(0xA));
    for (ix, invalid) in [
        (0xE4F, 0xE50),
        (0xECF, 0xED0),
        (0x300, 0x300),
        (0xF00, 0xF00),
    ] {
        let mut cpu = program(&[0x1AB]);
        cpu.state.ix = ix;
        cpu.state.sp = 0xFF;
        cpu.state.npc = 0x1A55;
        cpu.state.interrupt_delay = true;
        set_flags(&mut cpu.state, 15);
        cpu.ram = [7; RAM_NIBBLES];
        cpu.lcd = [9; LCD_NIBBLES];
        let before = cpu.clone();
        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(invalid)));
        assert_eq!(cpu, before);
    }
}

#[test]
fn return_skip_wraps_inside_the_current_bank_and_calz_sets_both_pc_and_npc() {
    let mut cpu = Cpu::new(&vec![0xFFB; PROGRAM_WORDS]).unwrap();
    cpu.state.pc = 0x1100;
    cpu.program[0x1100] = 0xFDE;
    for address in 0..3 {
        cpu.write_memory(address, 15).unwrap();
    }
    cpu.step().unwrap();
    assert_eq!(
        (cpu.state.pc, cpu.state.npc, cpu.state.sp),
        (0x1000, 0x1000, 3)
    );
    let mut cpu = program(&[0x5AB]);
    cpu.state.npc = 0x1A55;
    cpu.step().unwrap();
    assert_eq!((cpu.state.pc, cpu.state.npc), (0x10AB, 0x10AB));
}
