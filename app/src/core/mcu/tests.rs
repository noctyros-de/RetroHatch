use super::*;

fn machine(words: &[(usize, u16)]) -> Mcu {
    let mut program = vec![0xFFB; 0x1800];
    for &(address, word) in words {
        program[address] = word;
    }
    Mcu::new(&program).unwrap()
}

#[test]
fn cpu_io_lcd_halt_and_gpio_interrupt_execute_as_one_machine() {
    let mut m = machine(&[
        (0x100, 0xE0F),
        (0x101, 0xE80),
        (0x102, 0xB71),
        (0x103, 0xE20),
        (0x104, 0xB14),
        (0x105, 0xE24),
        (0x106, 0xF48),
        (0x107, 0xFF8),
        (0x106 + 0x100, 0xFFF),
    ]);
    // The first program configures I/O via actual instructions.
    for _ in 0..8 {
        m.clock().unwrap();
    }
    assert!(m.cpu.state().halted);
    assert_eq!(m.read(0xF71), Ok(0));
    assert_eq!(m.read(0xFE0), Err(CpuError::UnsupportedAddress(0xFE0)));
    // Enable K02 and verify that a physical edge wakes HALT and pushes the current PC.
    m.write(0xF14, 4).unwrap();
    m.pin(Port::K0, 4, Some(false));
    let result = m.clock().unwrap();
    assert!(result.instruction.is_none());
    assert_eq!(result.interrupt, Some(6));
    assert_eq!(result.cycles, 20);
    assert_eq!(m.cpu.state().pc, 0x106);
    assert_eq!(m.cpu.state().sp, 0xFD);
    assert_eq!(m.read(0xFD), Ok(8));
    assert!(!m.cpu.state().interrupt_enable);
    assert!(!m.cpu.state().halted);
    assert_eq!(m.read(0xF04), Ok(1));
    assert_eq!(m.read(0xF04), Ok(0));
}

#[test]
fn timer_interrupt_wakes_halt_and_handler_can_acknowledge_then_return() {
    let mut m = machine(&[
        (0x100, 0x020),
        (0x120, 0xF48),
        (0x121, 0xFF8),
        (0x122, 0xFF8),
        (0x102, 0xE0F),
        (0x103, 0xE80),
        (0x104, 0xB00),
        (0x105, 0xEC2),
        (0x106, 0xFDF),
    ]);
    m.write(0xF10, 1).unwrap();
    for _ in 0..3 {
        m.clock().unwrap();
    }
    assert!(m.cpu.state().halted);
    let mut woke = false;
    for _ in 0..200 {
        if m.clock().unwrap().interrupt == Some(2) {
            woke = true;
            break;
        }
    }
    assert!(woke);
    assert_eq!(m.cpu.state().pc, 0x102);
    assert_eq!(m.cpu.state().sp, 0xFD);
    for _ in 0..5 {
        m.clock().unwrap();
    }
    assert_eq!(m.cpu.state().a, 1);
    assert_eq!(m.read(0xF00), Ok(0));
    assert_eq!(m.cpu.state().pc, 0x122);
    assert_eq!(m.cpu.state().sp, 0);
}

#[test]
fn read_to_clear_effects_roll_back_if_a_later_cpu_operand_is_invalid() {
    let mut m = machine(&[
        (0x100, 0xE0E),
        (0x101, 0xE80),
        (0x102, 0xB50),
        (0x103, 0xE0F),
        (0x104, 0xE90),
        (0x105, 0x804),
        (0x106, 0xECB),
    ]);
    for _ in 0..6 {
        m.clock().unwrap();
    }
    m.write(0xF14, 4).unwrap();
    m.pin(Port::K0, 4, Some(false));
    // LD M(X),M(Y): valid interrupt-register source, invalid destination.
    let before_time = m.time();
    let before = m.cpu.clone();
    assert_eq!(m.clock().err(), Some(CpuError::UnsupportedAddress(0xE50)));
    assert_eq!(m.cpu, before);
    assert_eq!(m.read(0xF04), Ok(1));
    assert_eq!(m.time(), before_time);
}

#[test]
fn determinism_is_independent_of_requested_display_refresh_chunks() {
    let words = [(0x100, 0xFF8)];
    let mut whole = machine(&words);
    let mut chunks = machine(&words);
    for m in [&mut whole, &mut chunks] {
        m.write(0xF74, 3).unwrap();
        m.write(0xF54, 0).unwrap();
        m.write(0xF77, 1).unwrap();
    }
    whole.advance_to(MASTER_HZ / 10).unwrap();
    for i in 1..=100 {
        chunks.advance_to(MASTER_HZ * i / 1000).unwrap();
    }
    assert_eq!(whole.cpu, chunks.cpu);
    assert_eq!(whole.now, chunks.now);
    let audio = whole.drain_audio();
    assert!(audio.iter().any(|&sample| sample > 0));
    assert!(audio.iter().any(|&sample| sample < 0));
    assert_eq!(audio, chunks.drain_audio());
}

#[test]
fn fast_and_slow_oscillators_use_exact_integer_ratios() {
    let mut slow = machine(&[]);
    let mut fast = machine(&[]);
    fast.write(0xF70, 8).unwrap();
    slow.clock().unwrap();
    fast.clock().unwrap();
    assert_eq!(slow.time(), 5 * OSC1_TICKS);
    assert_eq!(fast.time(), 5 * OSC2_TICKS);
    assert_eq!(slow.cpu.state().pc, fast.cpu.state().pc);
}

#[test]
fn buzzer_frequency_envelope_one_shot_and_pcm_continue_during_halt() {
    let mut m = machine(&[(0x100, 0xFF8)]);
    m.write(0xF74, 3).unwrap();
    m.write(0xF54, 0).unwrap();
    assert_eq!(m.cpu.io().buzzer.frequency_hz(), 32768.0 / 14.0);
    m.advance_to(MASTER_HZ / 20).unwrap();
    let audio = m.drain_audio();
    assert!(audio.iter().any(|&sample| sample > 0));
    assert!(audio.iter().any(|&sample| sample < 0));
    m.write(0xF54, 8).unwrap();
    m.advance_to(MASTER_HZ / 10).unwrap();
    assert!(m.drain_audio().iter().all(|&sample| sample == 0));
    m.write(0xF75, 9).unwrap();
    assert_eq!(m.read(0xF75).unwrap() & 8, 8);
    m.advance_to(MASTER_HZ / 5).unwrap();
    assert_eq!(m.read(0xF75).unwrap() & 8, 0);
}

#[test]
fn invalid_io_and_reserved_opcodes_fail_without_advancing_time() {
    let mut m = machine(&[(0x100, 0xFF9)]);
    let before = m.cpu.clone();
    assert!(matches!(
        m.clock(),
        Err(CpuError::UnsupportedOpcode { opcode: 0xFF9, .. })
    ));
    assert_eq!(m.cpu, before);
    assert_eq!(m.time(), 0);
    assert_eq!(m.write(0xF7C, 1), Err(CpuError::UnsupportedAddress(0xF7C)));
}

#[test]
fn integrated_mcu_matches_the_original_clock_across_buttons_lcd_timers_audio_and_gpio() {
    compare_mcu_fixtures(
        include_str!("../../../tests/data/e0c6200_mcu_programs.txt"),
        include_str!("../../../tests/data/e0c6200_mcu_reference.txt"),
        crate::core::profile::DeviceProfile::P1,
        391,
    );
}

#[test]
fn genjintch_matches_original_clock_across_knock_lcd_gaps_and_oscillator_switching() {
    compare_mcu_fixtures(
        include_str!("../../../tests/data/genjintch_programs.txt"),
        include_str!("../../../tests/data/genjintch_reference.txt"),
        crate::core::profile::DeviceProfile::Genjintch,
        236,
    );
}

fn compare_mcu_fixtures(
    programs: &str,
    expected: &str,
    profile: crate::core::profile::DeviceProfile,
    expected_count: usize,
) {
    fn hex(s: &str) -> u16 {
        u16::from_str_radix(s, 16).unwrap()
    }
    fn memory<const N: usize>(text: &str) -> [u8; N] {
        let mut memory = [0; N];
        if !text.is_empty() {
            for pair in text.split(',') {
                let (address, value) = pair.split_once(':').unwrap();
                memory[hex(address) as usize] = hex(value) as u8;
            }
        }
        memory
    }
    let mut checkpoints = 0;
    for line in programs
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let input: Vec<_> = line.split('|').collect();
        let name = input[0];
        let clocks: usize = input[1].parse().unwrap();
        let program: Vec<_> = input[2]
            .split(',')
            .map(|pair| {
                let (address, word) = pair.split_once(':').unwrap();
                (hex(address) as usize, hex(word))
            })
            .collect();
        let parse_events = |text: &str| -> Vec<Vec<String>> {
            if text.is_empty() {
                Vec::new()
            } else {
                text.split(',')
                    .map(|event| event.split('@').map(str::to_owned).collect())
                    .collect()
            }
        };
        let pins = parse_events(input[3]);
        let writes = parse_events(input[4]);
        let rows: Vec<_> = expected
            .lines()
            .filter(|row| row.starts_with(&format!("{name}|")))
            .collect();
        let mut row_index = 0;
        let mut words = vec![0xFFB; profile.rom_bytes() / 2];
        for &(address, opcode) in &program {
            words[address] = opcode;
        }
        let mut m = Mcu::for_profile(&words, profile).unwrap();
        for clock in 0..clocks {
            for event in &pins {
                if event[0].parse::<usize>().unwrap() == clock {
                    let port = if hex(&event[1]) == 0 {
                        Port::K0
                    } else {
                        Port::K1
                    };
                    let level = match event[3].as_str() {
                        "-1" => None,
                        "0" => Some(false),
                        "1" => Some(true),
                        _ => panic!("bad pin event"),
                    };
                    m.pin(port, hex(&event[2]) as u8, level);
                }
            }
            for event in &writes {
                if event[0].parse::<usize>().unwrap() == clock {
                    m.write(hex(&event[1]), hex(&event[2]) as u8).unwrap();
                }
            }
            m.clock().unwrap();
            if row_index == rows.len() {
                continue;
            }
            let fields: Vec<_> = rows[row_index].split('|').collect();
            if fields[1].parse::<usize>().unwrap() != clock + 1 {
                continue;
            }
            let context = format!("{name}, clock {}", clock + 1);
            assert_eq!(
                m.time(),
                u64::from_str_radix(fields[2], 16).unwrap(),
                "{context}"
            );
            let s = m.cpu.state();
            let flags = u16::from(s.carry)
                | (u16::from(s.zero) << 1)
                | (u16::from(s.decimal) << 2)
                | (u16::from(s.interrupt_enable) << 3);
            let state = [
                s.pc,
                s.npc,
                u16::from(s.a),
                u16::from(s.b),
                s.ix,
                s.iy,
                u16::from(s.sp),
                flags,
                u16::from(s.interrupt_delay),
                u16::from(s.halted),
            ];
            assert_eq!(
                state.as_slice(),
                fields[3].split(',').map(hex).collect::<Vec<_>>(),
                "{context}"
            );
            assert_eq!(m.cpu.ram(), &memory::<0x300>(fields[4]), "{context}");
            assert_eq!(m.cpu.lcd_memory(), &memory::<0xA0>(fields[5]), "{context}");
            let mut io = m.cpu.io().clone();
            let actual: String = (0..0x7F)
                .map(|index| {
                    let address = 0xF00 + index;
                    let value = if io::Io::supports(address) {
                        io.read(address)
                    } else {
                        0
                    };
                    format!("{value:X}")
                })
                .collect();
            assert_eq!(actual, fields[6], "{context}");
            assert_eq!(
                m.cpu.io().buzzer.inspect().as_slice(),
                fields[7].split(',').map(hex).collect::<Vec<_>>(),
                "{context}"
            );
            m.drain_audio();
            row_index += 1;
            checkpoints += 1;
        }
        assert_eq!(row_index, rows.len());
        if name == "buttons_lcd" {
            assert_eq!(m.cpu.lcd_memory()[0], 15);
            assert_eq!(m.cpu.lcd_memory()[16], 10);
            assert_eq!(m.interrupts, 4);
        }
    }
    assert_eq!(checkpoints, expected_count);
}

#[test]
fn interrupt_priority_and_enable_delay_match_the_original_mcu_order() {
    let mut m = machine(&[(0x100, 0xF48), (0x101, 0xFFF)]);
    m.write(0xF14, 4).unwrap();
    m.pin(Port::K0, 4, Some(false));
    m.write(0xF26, 1).unwrap();
    m.write(0xF78, 2).unwrap();
    m.write(0xF79, 7).unwrap();
    m.write(0xF12, 1).unwrap();
    let first = m.clock().unwrap();
    assert_eq!(first.interrupt, None);
    assert!(m.cpu.state().interrupt_delay);
    let second = m.clock().unwrap();
    assert_eq!(second.interrupt, Some(0xC));
    assert_eq!(second.cycles, 20);
    assert_eq!((m.cpu.state().pc, m.cpu.state().sp), (0x10C, 0xFD));
    assert!(!m.cpu.state().interrupt_enable);
    assert_eq!(m.read(0xF04), Ok(1)); // Lower-priority GPIO remains pending.
}

#[test]
fn reset_hold_stops_the_oscillator_and_release_restarts_actual_cpu_execution() {
    let mut m = machine(&[(0x100, 0xE01)]);
    m.pin(Port::K0, 4, Some(false));
    m.write(0xE00, 15).unwrap();
    m.hold_reset(true);
    let held = m.clock().unwrap();
    assert!(held.instruction.is_none());
    assert_eq!(m.cpu.state().pc, 0x100);
    assert_eq!(m.read(0xF20), Ok(0));
    assert_eq!(m.read(0xF40), Ok(15));
    assert_eq!(m.read(0xE00), Ok(0));
    assert!(m.drain_audio().iter().all(|&v| v == 0));
    m.hold_reset(false);
    m.clock().unwrap();
    assert_eq!(m.cpu.state().a, 1);
    assert_eq!(m.cpu.state().pc, 0x101);
    assert_eq!(m.read(0xF20), Ok(1));
}

#[test]
fn cycle_counters_preserve_exact_fast_slow_and_halt_clock_scaling() {
    let mut m = machine(&[]);
    for _ in 0..10 {
        m.clock().unwrap();
    }
    assert_eq!(m.counters.slow, 50);
    assert_eq!(m.time(), 50 * OSC1_TICKS);
    m.write(0xF70, 8).unwrap();
    for _ in 0..10 {
        m.clock().unwrap();
    }
    assert_eq!(m.counters.fast, 50);
    assert_eq!(m.counters.instructions, 100);
    assert_eq!(m.counters.total(), m.counters.fast + m.counters.slow);
    assert_eq!(
        m.time(),
        m.counters.fast * OSC2_TICKS + m.counters.slow * OSC1_TICKS
    );
    assert_eq!(m.counters.osc1_edges, m.time() / OSC1_TICKS + 1);

    let mut halted = machine(&[(0x100, 0xFF8)]);
    halted.clock().unwrap(); // HALT executes for five slow-clock cycles.
    halted.clock().unwrap(); // Idle preserves the selected slow oscillator.
    halted.write(0xF70, 8).unwrap();
    halted.clock().unwrap(); // Idle now consumes seven fast-clock cycles.
    assert_eq!(halted.counters.instructions, 5);
    assert_eq!(halted.counters.halt, 14);
    assert_eq!(halted.counters.slow, 12);
    assert_eq!(halted.counters.fast, 7);
    assert_eq!(halted.counters.total(), 19);
    assert_eq!(halted.time(), 12 * OSC1_TICKS + 7 * OSC2_TICKS);
}

#[test]
fn detailed_profiling_does_not_change_execution_time_state_or_nonzero_pcm() {
    let mut plain = machine(&[]);
    let mut profiled = machine(&[]);
    profiled.set_profiling(true);
    for m in [&mut plain, &mut profiled] {
        m.write(0xF74, 3).unwrap();
        m.write(0xF54, 0).unwrap();
    }
    for clock in 0..600 {
        if clock == 100 {
            plain.write(0xF70, 8).unwrap();
            profiled.write(0xF70, 8).unwrap();
        }
        plain.clock().unwrap();
        profiled.clock().unwrap();
    }
    assert_eq!(plain.time(), profiled.time());
    assert_eq!(plain.cpu, profiled.cpu);
    assert_eq!(plain.counters, profiled.counters);
    let samples = plain.drain_audio();
    assert!(samples.iter().any(|&sample| sample != 0));
    assert_eq!(samples, profiled.drain_audio());
    assert_eq!(plain.profile.cpu_ns, 0);
    assert!(profiled.profile.cpu_ns > 0);
    assert!(profiled.profile.irq_ns > 0);
    assert!(profiled.profile.timers_ns > 0);
    assert!(profiled.profile.pcm_ns > 0);
}

#[test]
fn genjintch_fractional_fast_clock_preserves_remainders_across_halt_and_switches() {
    use crate::core::profile::DeviceProfile;
    let words = vec![0xFFB; 0x2000];
    let mut m = Mcu::for_profile(&words, DeviceProfile::Genjintch).unwrap();
    for i in 0..10000 {
        m.write(0xF70, if i % 53 < 31 { 8 } else { 0 }).unwrap();
        m.clock().unwrap();
        let expected = m.counters.slow * OSC1_TICKS + m.counters.fast * MASTER_HZ / 1_060_000;
        assert_eq!(m.time(), expected);
    }
    let mut words = words;
    words[0x100] = 0xFF8;
    let mut m = Mcu::for_profile(&words, DeviceProfile::Genjintch).unwrap();
    m.write(0xF70, 8).unwrap();
    for _ in 0..1000 {
        m.clock().unwrap();
    }
    assert_eq!(m.counters.halt, 999 * 7);
    assert_eq!(m.time(), m.counters.fast * MASTER_HZ / 1_060_000);
    assert_eq!(m.counters.osc1_edges, m.time() / OSC1_TICKS + 1);
    assert_eq!(m.counters.audio_samples, m.time() * 44100 / MASTER_HZ);
}

#[test]
fn angel_matches_reference_timers_knock_halt_audio_clock_and_disconnected_bus() {
    // compare_angel.py verifies identical clock/masks/GPIO in the original profiles.
    compare_mcu_fixtures(
        include_str!("../../../tests/data/genjintch_programs.txt"),
        include_str!("../../../tests/data/genjintch_reference.txt"),
        crate::core::profile::DeviceProfile::Angel,
        236,
    );
}

#[test]
fn morino_matches_reference_timers_knock_halt_audio_clock_and_disconnected_bus() {
    // compare_morino.py checks the original profile before sharing its hardware oracle.
    compare_mcu_fixtures(
        include_str!("../../../tests/data/genjintch_programs.txt"),
        include_str!("../../../tests/data/genjintch_reference.txt"),
        crate::core::profile::DeviceProfile::Morino,
        236,
    );
}

#[test]
fn umino_matches_reference_timers_knock_halt_audio_clock_and_disconnected_bus() {
    // compare_umino.py checks the original profile before sharing its hardware oracle.
    compare_mcu_fixtures(
        include_str!("../../../tests/data/genjintch_programs.txt"),
        include_str!("../../../tests/data/genjintch_reference.txt"),
        crate::core::profile::DeviceProfile::Umino,
        236,
    );
}

#[test]
fn umino_unmapped_data_bus_matches_reference_without_changing_strict_profiles() {
    use crate::core::profile::DeviceProfile;
    let words = vec![0xFF8; 0x2000];
    let mut umino = Mcu::for_profile(&words, DeviceProfile::Umino).unwrap();
    for address in [
        0x300, 0xDFF, 0xE50, 0xE7F, 0xED0, 0xEFF, 0xF06, 0xF7C, 0xF7F, 0xFD0, 0xFFF,
    ] {
        let before = umino.cpu.clone();
        assert_eq!(umino.read(address), Ok(0));
        umino.write(address, 15).unwrap();
        assert_eq!(
            umino.cpu, before,
            "Disconnected write changed real memory at {address:03X}"
        );
    }
    let before = umino.cpu.clone();
    assert!(umino.write(0x1000, 15).is_err());
    assert!(umino.read(0x1000).is_err());
    assert_eq!(umino.cpu, before);
    for profile in [
        DeviceProfile::P1,
        DeviceProfile::Genjintch,
        DeviceProfile::Angel,
        DeviceProfile::Morino,
    ] {
        let mut other = Mcu::for_profile(&words, profile).unwrap();
        let before = other.cpu.clone();
        assert_eq!(other.read(0xFD0), Err(CpuError::UnsupportedAddress(0xFD0)));
        assert!(other.write(0xFD0, 15).is_err());
        assert_eq!(other.cpu, before);
    }
    let mut reserved = words.clone();
    reserved[0x100] = 0xFF9;
    let mut invalid = Mcu::for_profile(&reserved, DeviceProfile::Umino).unwrap();
    let before = invalid.cpu.clone();
    assert!(matches!(
        invalid.clock(),
        Err(CpuError::UnsupportedOpcode { .. })
    ));
    assert_eq!(invalid.cpu, before);
    assert_eq!(invalid.time(), 0);
    compare_mcu_fixtures(
        include_str!("../../../tests/data/umino_programs.txt"),
        include_str!("../../../tests/data/umino_reference.txt"),
        DeviceProfile::Umino,
        65,
    );
}
