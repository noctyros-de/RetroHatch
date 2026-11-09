// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

#[test]
fn stack_transfers_match_original_for_all_sp_values_flags_nibbles_and_aliases() {
    assert_eq!(
        transfers::check_reference(include_str!(
            "../../../tests/data/e0c6200_stack_reference.txt"
        )),
        112640
    );
}

#[test]
fn every_stack_encoding_selects_the_original_operand_register_or_flags() {
    let values = [
        StackValue::Operand(Operand::A),
        StackValue::Operand(Operand::B),
        StackValue::Operand(Operand::Mx),
        StackValue::Operand(Operand::My),
        StackValue::Register(RegisterNibble::Xp),
        StackValue::Register(RegisterNibble::Xh),
        StackValue::Register(RegisterNibble::Xl),
        StackValue::Register(RegisterNibble::Yp),
        StackValue::Register(RegisterNibble::Yh),
        StackValue::Register(RegisterNibble::Yl),
        StackValue::Flags,
    ];
    for (index, value) in values.into_iter().enumerate() {
        assert_eq!(
            Instruction::decode(0xFC0 + index as u16),
            Some(Instruction::Push(value))
        );
        assert_eq!(
            Instruction::decode(0xFD0 + index as u16),
            Some(Instruction::Pop(value))
        );
    }
}

#[test]
fn push_and_pop_wrap_sp_and_preserve_old_pointer_aliases() {
    let mut cpu = program(&[0xFC2, 0xFD3]);
    cpu.state.ix = 0xFF;
    cpu.state.iy = 0xFF;
    cpu.write_memory(0xFF, 0xA).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state.sp, 0xFF);
    assert_eq!(cpu.read_memory(0xFF), Ok(0xA));
    cpu.step().unwrap();
    assert_eq!(cpu.state.sp, 0);
    assert_eq!((cpu.state.ix, cpu.state.iy), (0xFF, 0xFF));
    assert_eq!(cpu.read_memory(0xFF), Ok(0xA));
}

#[test]
fn pop_flags_replaces_every_flag_and_delays_only_a_new_interrupt_enable() {
    for (initial, popped, delay) in [
        (0, 8, true),
        (8, 8, false),
        (15, 0, false),
        (0, 15, true),
        (15, 7, false),
    ] {
        let mut cpu = program(&[0xFDA, 0xFCA, 0xFDA]);
        set_flags(&mut cpu.state, initial);
        cpu.write_memory(0, popped).unwrap();
        cpu.state.interrupt_delay = true;
        cpu.step().unwrap();
        assert_eq!(flags(&cpu.state), popped);
        assert_eq!(cpu.state.interrupt_delay, delay);
        cpu.step().unwrap();
        assert!(!cpu.state.interrupt_delay);
        assert_eq!(flags(&cpu.state), popped);
        cpu.step().unwrap();
        assert_eq!(flags(&cpu.state), popped);
        assert!(!cpu.state.interrupt_delay);
        assert_eq!(cpu.state.sp, 1);
    }
}

#[test]
fn invalid_push_sources_and_pop_destinations_are_atomic_even_after_page_or_flag_control() {
    for opcode in [0xFC2, 0xFC3, 0xFD2, 0xFD3] {
        for address in [0x300, 0xE50, 0xED0, 0xF00] {
            for sp in [0, 1, 0xFE, 0xFF] {
                let mut cpu = program(&[opcode]);
                cpu.state.ix = address;
                cpu.state.iy = address;
                cpu.state.sp = sp;
                cpu.state.npc = 0x1A55;
                cpu.state.interrupt_delay = true;
                set_flags(&mut cpu.state, 15);
                cpu.ram = [7; RAM_NIBBLES];
                cpu.lcd = [9; LCD_NIBBLES];
                let before = cpu.clone();
                assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(address)));
                assert_eq!(cpu, before);
            }
        }
    }
}
