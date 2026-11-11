// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Synthetic ARM programs and host integration; these are not firmware boot claims.
use super::*;
use crate::host::{
    fingerprints::Assignment,
    library::{Library, inspect_bytes},
    runner::{Command, Controller, Snapshot},
    saves::{SavePolicy, SaveTarget},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tg18::cpu::Bus;
use tg18::sys::{GPIO_BASE, LCD_BASE};

fn image() -> Vec<u8> {
    let mut bytes = vec![0; tg18::FLASH_SIZE];
    bytes[..4].copy_from_slice(b"SPII");
    // ARM: add r0,r0,#1; branch back. Entirely synthetic and independently authored.
    bytes[0x48..0x4c].copy_from_slice(&0xe2800001u32.to_le_bytes());
    bytes[0x4c..0x50].copy_from_slice(&0xeafffffdu32.to_le_bytes());
    bytes
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "retrohatch-on-{}-{}",
            std::process::id(),
            tg18::unix_now().to_bits()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn wait(c: &Controller, p: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let end = Instant::now() + Duration::from_secs(20);
    loop {
        let s = c.snapshot();
        if p(&s) {
            return s;
        }
        assert!(
            Instant::now() < end,
            "worker timeout: {:?} {:?}",
            s.error,
            s.save_error
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn on(d: &mut Emulator) -> &mut OnDevice {
    match &mut d.backend {
        Backend::On(on) => on,
        _ => unreachable!(),
    }
}

#[test]
fn arm_program_matches_the_reused_reference_core_and_lcd_bus() {
    let bytes = image();
    let mut device = Emulator::from_bytes(&bytes, DeviceProfile::On).unwrap();
    let mut reference = Machine::new(Arc::new(bytes.clone()), &bytes, 0.0);
    let tick = MASTER_HZ / 10000;
    let limit = (u128::from(tick) * CPU_HZ as u128).div_ceil(u128::from(MASTER_HZ)) as u64;
    assert_eq!(reference.run(limit), Outcome::Limit);
    reference.sys.advance(reference.now());
    device.advance_to(tick).unwrap();
    let arm = on(&mut device);
    assert_eq!(arm.machine.cpu.state(), reference.cpu.state());
    assert_eq!(arm.machine.executed, reference.executed);
    assert!(arm.machine.cpu.r[0] > 0);
    // Send the original LCD command/data transactions through the MCU bus.
    for (data, control) in [(0x2c, 0x81), (0xf800, 0xa1), (0x07e0, 0xa1), (0x001f, 0xa1)] {
        arm.machine.sys.write32(LCD_BASE + 0x16c, data);
        arm.machine.sys.write32(LCD_BASE + 0x140, control);
    }
    let rgb = device.color_display().unwrap();
    assert_eq!(&rgb[..9], &[255, 0, 0, 0, 255, 0, 0, 0, 255]);
    assert!(device.classic_cpu_state().is_none());
}

#[test]
fn buttons_debounce_release_sleep_rtc_wakeup_and_reset_keep_real_hardware_state() {
    let mut device = Emulator::from_bytes(&image(), DeviceProfile::On).unwrap();
    for (button, mask) in [
        (Button::A, 1 << 5),
        (Button::B, 1 << 6),
        (Button::C, 1 << 7),
    ] {
        device.set_button(button, true);
        let arm = on(&mut device);
        arm.machine.sys.advance(arm.machine.now());
        assert_ne!(arm.machine.sys.read32(GPIO_BASE + 0x20) & mask, 0);
        device.set_button(button, false);
    }
    device.release_inputs();
    let arm = on(&mut device);
    arm.machine.sys.advance(arm.machine.now());
    assert_eq!(arm.machine.sys.keys_held(), [false; 3]);
    arm.machine.sys.powered_off = true;
    let start = arm.machine.sys.rtc.ticks(arm.machine.sys.game_time);
    device.advance_to(MASTER_HZ * 60).unwrap();
    let arm = on(&mut device);
    assert_eq!(
        arm.machine.sys.rtc.ticks(arm.machine.sys.game_time) - start,
        60 * 32768
    );
    assert_eq!(arm.machine.sys.turbo, 1.0);
    arm.machine.sys.ram[tg18::FLASH_SIZE - 1] = 0x5a;
    device.set_button(Button::B, true);
    device
        .advance_to(MASTER_HZ * 60 + MASTER_HZ / 1000)
        .unwrap();
    assert!(!on(&mut device).machine.sys.powered_off);
    device.reset();
    assert_eq!(on(&mut device).machine.sys.ram[tg18::FLASH_SIZE - 1], 0x5a);
}

#[test]
fn tg18_state_roundtrip_preserves_flash_ram_lcd_rtc_sound_and_continuation_atomically() {
    let bytes = image();
    let mut device = Emulator::from_bytes(&bytes, DeviceProfile::On).unwrap();
    device.advance_to(MASTER_HZ / 1000).unwrap();
    device.drain_audio();
    let arm = on(&mut device);
    arm.machine.sys.sram[512] = 99;
    arm.machine.sys.ram[tg18::FLASH_SIZE - 2] = 123;
    arm.machine.sys.lcd.fb[0] = 0x1f;
    arm.machine.sys.sound.state = (1000.0, 0.5);
    arm.synth.freq = 1000.0;
    arm.synth.restore_phase(0.25);
    let saved = device.save_state().unwrap();
    let mut restored = device.restored(&saved).unwrap();
    assert_eq!(
        on(&mut restored).machine.cpu.state(),
        on(&mut device).machine.cpu.state()
    );
    assert_eq!(
        on(&mut restored).machine.sys.ram,
        on(&mut device).machine.sys.ram
    );
    assert_eq!(on(&mut restored).machine.sys.sram[512], 99);
    assert_eq!(restored.color_display(), device.color_display());
    assert_eq!(on(&mut restored).synth.phase(), 0.25);
    let end = device.time() + MASTER_HZ / 500;
    device.advance_to(end).unwrap();
    restored.advance_to(end).unwrap();
    assert_eq!(
        on(&mut restored).machine.cpu.state(),
        on(&mut device).machine.cpu.state()
    );
    assert_eq!(restored.drain_audio(), device.drain_audio());
    let before = restored.time();
    let mut corrupt = saved.clone();
    corrupt[100] ^= 1;
    assert!(restored.restored(&corrupt).is_err());
    assert_eq!(restored.time(), before);
    let mut other = bytes.clone();
    other[100] ^= 1;
    assert!(
        Emulator::from_bytes(&other, DeviceProfile::On)
            .unwrap()
            .restored(&saved)
            .is_err()
    );
    let classic = vec![0; DeviceProfile::P1.rom_bytes()];
    assert!(
        Emulator::from_bytes(&classic, DeviceProfile::P1)
            .unwrap()
            .restored(&saved)
            .is_err()
    );
}

#[test]
fn private_library_remembers_unknown_on_model_and_worker_resumes_after_restart() {
    let tmp = Temp::new();
    let bytes = image();
    let prepared = inspect_bytes(bytes.clone(), "misleading-P1-name.bin".into()).unwrap();
    assert_eq!(prepared.candidates, [DeviceProfile::On]);
    let mut library = Library::open(&tmp.0).unwrap();
    assert!(library.automatic(&prepared).is_none());
    let entry = library
        .import(
            &prepared,
            Assignment {
                profile: DeviceProfile::On,
                variant: "Synthetic ARM program".into(),
                evidence: "Test-only model confirmation".into(),
            },
        )
        .unwrap();
    drop(library);
    let library = Library::open(&tmp.0).unwrap();
    assert_eq!(
        library.automatic(&prepared).unwrap().profile,
        DeviceProfile::On
    );
    let path = library.verified_path(&entry.key).unwrap();
    let policy = SavePolicy {
        save_on_exit: true,
        ..SavePolicy::default()
    };
    let c = Controller::with_storage(false, tmp.0.clone(), policy.clone());
    c.send(Command::LoadVerified(
        path.clone(),
        DeviceProfile::On,
        sha256(&bytes),
    ));
    wait(&c, |s| s.loaded && !s.loading && s.ticks > 0);
    c.send(Command::Pause);
    let paused = wait(&c, |s| !s.running);
    assert!(paused.arm_pc.is_some());
    assert!(paused.color_display.is_some());
    c.send(Command::Save(SaveTarget::Slot(1)));
    wait(&c, |s| s.slots[0]);
    c.send(Command::LoadSave(SaveTarget::Slot(1)));
    wait(&c, |s| s.restores > 0);
    let export = tmp.0.join("export.rhstate");
    let count = c.snapshot().save_count;
    c.send(Command::Export(export.clone()));
    wait(&c, |s| s.save_count > count);
    let restored = c.snapshot().restores;
    c.send(Command::Import(export));
    wait(&c, |s| s.restores > restored);
    c.send(Command::PrepareExit);
    wait(&c, |s| s.exit_ready);
    drop(c);
    let again = Controller::with_storage(false, tmp.0.clone(), policy);
    again.send(Command::LoadProfile(path, DeviceProfile::On));
    let resumed = wait(&again, |s| s.restores > 0 && !s.loading);
    assert!(resumed.ticks >= paused.ticks);
    assert!(resumed.error.is_none());
    again.send(Command::PrepareExit);
    wait(&again, |s| s.exit_ready);
    drop(again);
    assert_eq!(
        std::fs::read(library.verified_path(&entry.key).unwrap()).unwrap(),
        bytes
    );
}

#[test]
fn on_validation_never_uses_filenames_or_silently_accepts_wrong_formats() {
    let mut bytes = image();
    bytes[..4].copy_from_slice(b"NOPE");
    assert!(validate_rom(&bytes, DeviceProfile::On).is_err());
    assert!(validate_rom(&image()[..4096], DeviceProfile::On).is_err());
    assert!(RomImage::decode_for_profile(&image(), DeviceProfile::On).is_err());
    assert!(DeviceProfile::On.is_color());
    assert!(!DeviceProfile::On.has_knock());
    assert_eq!(DeviceProfile::parse("on"), Some(DeviceProfile::On));
    for classic in DeviceProfile::CLASSIC {
        assert!(DeviceProfile::ALL.contains(&classic));
    }
}

#[test]
fn ps_awake_policy_reaches_the_worker_before_and_after_loading() {
    let temp = Temp::new();
    let path = temp.0.join("synthetic-ps.bin");
    let mut bytes = vec![255; 0x800000];
    bytes[0x400000..0x400004].copy_from_slice(&0x02001000u32.to_le_bytes());
    bytes[0x1000..0x1002].copy_from_slice(&0x0080u16.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    let controller = Controller::new(false);
    controller.send(Command::KeepScreenAwake(true));
    controller.send(Command::LoadProfile(path, DeviceProfile::Ps));
    wait(&controller, |s| s.loaded && !s.loading && !s.sleeping);
    controller.send(Command::KeepScreenAwake(false));
    wait(&controller, |s| s.sleeping);
    controller.send(Command::KeepScreenAwake(true));
    wait(&controller, |s| !s.sleeping);
}

#[test]
fn id_library_profile_and_saves_are_distinct_from_ps_and_other_firmware() {
    let temp = Temp::new();
    let mut image = vec![255; DeviceProfile::Id.rom_bytes()];
    image[..4].copy_from_slice(&0xc01000u32.to_le_bytes());
    image[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    let prepared = inspect_bytes(image.clone(), "misleading-Ps-name.bin".into()).unwrap();
    assert_eq!(
        prepared.candidates,
        [
            DeviceProfile::Id,
            DeviceProfile::IdMelody,
            DeviceProfile::PlusColor,
            DeviceProfile::PlusColorHexa
        ]
    );
    let mut library = Library::open(&temp.0).unwrap();
    assert!(library.automatic(&prepared).is_none());
    let entry = library
        .import(
            &prepared,
            Assignment {
                profile: DeviceProfile::Id,
                variant: String::new(),
                evidence: "Synthetic test; user-confirmed model".into(),
            },
        )
        .unwrap();
    drop(library);
    let library = Library::open(&temp.0).unwrap();
    assert_eq!(
        library.automatic(&prepared).unwrap().profile,
        DeviceProfile::Id
    );
    assert_eq!(
        std::fs::read(library.verified_path(&entry.key).unwrap()).unwrap(),
        image
    );
    let mut device = Emulator::from_bytes(&image, DeviceProfile::Id).unwrap();
    device.advance_to(102_400).unwrap();
    let saved = device.save_state().unwrap();
    let restored = device.restored(&saved).unwrap();
    assert_eq!(restored.time(), device.time());
    assert_eq!(restored.color_display(), device.color_display());
    assert!(device.identity().starts_with("tamagotchi-id-"));
    let mut different = image.clone();
    different[0x2000] ^= 1;
    assert!(
        Emulator::from_bytes(&different, DeviceProfile::Id)
            .unwrap()
            .restored(&saved)
            .is_err()
    );
    assert!(Emulator::from_bytes(&image, DeviceProfile::Ps).is_err());
    device.set_button(Button::A, true);
    device.set_button(Button::B, true);
    device.release_inputs();
    let Backend::Ps(machine) = &device.backend else {
        panic!("Expected shared S1C33 backend")
    };
    assert_eq!(machine.memory.profile.key, "id");
    assert_eq!(machine.memory.peripherals.as_ref().unwrap().buttons, 0);
    assert_eq!(profile_code(DeviceProfile::Ps), 7);
    assert_eq!(profile_code(DeviceProfile::Id), 8);
}

#[test]
fn idl_models_share_execution_but_never_save_identity_or_awake_overrides() {
    let mut image = vec![255; DeviceProfile::Idl.rom_bytes()];
    image[0x400000..0x400004].copy_from_slice(&0x2401000u32.to_le_bytes());
    image[0x401000..0x401004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    let jp = Emulator::from_bytes(&image, DeviceProfile::Idl).unwrap();
    let saved = jp.save_state().unwrap();
    let mut en = Emulator::from_bytes(&image, DeviceProfile::IdlEnglish).unwrap();
    assert_ne!(jp.identity(), en.identity());
    let before = en.save_state().unwrap();
    assert!(en.restored(&saved).is_err());
    assert_eq!(en.save_state().unwrap(), before);
    en.set_keep_screen_awake(true);
    let Backend::Ps(m) = &en.backend else {
        unreachable!()
    };
    assert!(m.keep_screen_awake);
    let mut princess = Emulator::from_bytes(&image, DeviceProfile::IdlPrincess).unwrap();
    princess.set_keep_screen_awake(true);
    let Backend::Ps(m) = &princess.backend else {
        unreachable!()
    };
    assert!(!m.keep_screen_awake);
}

#[test]
fn plus_color_imports_require_evidence_and_isolate_profile_and_revision_saves() {
    let mut image = vec![255; DeviceProfile::PlusColor.rom_bytes()];
    image[..4].copy_from_slice(&0xc01000u32.to_le_bytes());
    image[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    let prepared = inspect_bytes(image.clone(), "P's-or-iD-is-not-evidence.bin".into()).unwrap();
    assert_eq!(
        prepared.candidates,
        vec![
            DeviceProfile::Id,
            DeviceProfile::IdMelody,
            DeviceProfile::PlusColor,
            DeviceProfile::PlusColorHexa
        ]
    );
    let temp = Temp::new();
    let mut lib = Library::open(&temp.0).unwrap();
    assert!(lib.automatic(&prepared).is_none());
    let entry = lib
        .import(
            &prepared,
            Assignment {
                profile: DeviceProfile::PlusColor,
                variant: "Synthetic".into(),
                evidence: "Explicit confirmation".into(),
            },
        )
        .unwrap();
    assert_eq!(
        Library::open(&temp.0)
            .unwrap()
            .automatic(&prepared)
            .unwrap()
            .profile,
        DeviceProfile::PlusColor
    );
    assert!(lib.verified_path(&entry.key).is_ok());
    let mut color = Emulator::from_bytes(&image, DeviceProfile::PlusColor).unwrap();
    let mut hexa = Emulator::from_bytes(&image, DeviceProfile::PlusColorHexa).unwrap();
    color.set_keep_screen_awake(true);
    hexa.set_keep_screen_awake(true);
    let Backend::Ps(ordinary) = &color.backend else {
        unreachable!()
    };
    let Backend::Ps(special) = &hexa.backend else {
        unreachable!()
    };
    assert!(
        !ordinary.keep_screen_awake,
        "Early Plus Color marker is unverified"
    );
    assert!(
        special.keep_screen_awake,
        "Verified Hexagontchi marker at zero is valid"
    );
    assert_ne!(color.identity(), hexa.identity());
    let state = color.save_state().unwrap();
    let before = hexa.save_state().unwrap();
    assert!(hexa.restored(&state).is_err());
    assert_eq!(hexa.save_state().unwrap(), before);
    image[0x2000] ^= 1;
    let revision = Emulator::from_bytes(&image, DeviceProfile::PlusColor).unwrap();
    assert_ne!(color.identity(), revision.identity());
    assert!(revision.restored(&state).is_err());
    assert!(Emulator::from_bytes(&image, DeviceProfile::Ps).is_err());
}

#[test]
fn four_u_plus_variants_have_independent_schema_four_saves_and_reject_old_device_mixing() {
    let mut bytes = vec![255; 0x800000];
    bytes[0x400000..0x400004].copy_from_slice(&0x2401000u32.to_le_bytes());
    bytes[0x401000..0x401004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    let jp = Emulator::from_bytes(&bytes, DeviceProfile::FourUPlus).unwrap();
    let en = Emulator::from_bytes(&bytes, DeviceProfile::FourUPlusEnglish).unwrap();
    assert_ne!(jp.identity(), en.identity());
    let saved = jp.save_state().unwrap();
    assert_eq!(&saved[10..13], &[4, 0, 16]);
    let before = en.save_state().unwrap();
    assert!(en.restored(&saved).is_err());
    assert_eq!(en.save_state().unwrap(), before);
    assert_eq!(jp.restored(&saved).unwrap().save_state().unwrap(), saved);
    let ps = Emulator::from_bytes(&bytes, DeviceProfile::Ps).unwrap();
    assert_eq!(&ps.save_state().unwrap()[10..12], &[3, 0]);
    assert!(ps.restored(&saved).is_err());
    bytes[0x2000] ^= 1;
    let other = Emulator::from_bytes(&bytes, DeviceProfile::FourUPlus).unwrap();
    assert_ne!(jp.identity(), other.identity());
    assert!(other.restored(&saved).is_err());
    assert_eq!(
        DeviceProfile::parse("tamagotchi-4u-plus-en"),
        Some(DeviceProfile::FourUPlusEnglish)
    );
}
