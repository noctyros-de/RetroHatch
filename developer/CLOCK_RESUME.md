<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Practical clock synchronization on resume

## Implemented contract

Version 0.11.0 adds persistent **Settings > Sync Clock on Resume**, enabled by
default for the verified **Tamagotchi On Magic EN v058** code fingerprint.
Eligibility also requires TG18's existing firmware clock-reader signature.
Other firmware keeps its previous behavior; the shared preference is retained
when switching devices and its checkbox is unavailable for incompatible images.

On automatic reopening of a library entry/autosave, add actual elapsed UTC
milliseconds to the saved external 32,768 Hz RTC and SoC time-of-day bases.
Preserve the toy's existing date/time offset. Do not substitute Windows local
time, apply an extra timezone/DST offset, execute missed CPU cycles, accelerate
emulation, replay missed interrupts or directly change care/age/health fields.
UTC elapsed time covers midnight/year rollover and 23/25-hour DST days; the toy
retains its own clock convention rather than automatically adopting Windows DST.

A one-shot refresh of the already-used TG18 firmware RTC reader updates its
month/day/hour/minute cache at the next normal main-context idle boundary.
This follows upstream's never-sleep clock refresh, avoids interrupt-stack and
routine re-entry, and needs no fake key, forced reset or guessed RAM patch.
A pending refresh survives exact snapshots. Sleeping firmware instead reads the
RTC on its ordinary cold wake. An expired alarm may cause a normal immediate
wake; no offline sequence of alarms is synthesized. A failed cache-reader call
stops execution with an explicit error rather than reporting a successful clock.

Firmware may naturally react to the new date/time during subsequent normal
execution (for example bedtime or calendar events). This is **not** a promise
that gameplay is independent of time. The emulator performs no offline care,
growth, hunger, happiness, health or age simulation.

## Saves and settings

- The existing `RTHSTATE` version, backend schemas, ROM/profile identities and
  save directories are unchanged. Persistent TG18 captures add optional
  `rh-resume-utc-v1` snapshot metadata: one little-endian UTC Unix-millisecond
  `u64`, captured coherently before asynchronous encoding/disk I/O. The existing
  full-state SHA-256 covers it. Earlier snapshot readers ignore unknown chunks.
- Native `save_state()`/exact restores remain exact. Manual slots and save import
  never apply elapsed time. All three slots, export, autosave and Save on Exit
  carry an anchor for future automatic reopening; exporting does not rewrite ROMs.
- Historical saves without this explicit timestamp load unchanged. Never infer
  elapsed time from file modification time or historical diagnostic timestamps.
  The first new persistent save starts clock synchronization for later reopenings.
- Each reopen restores the original snapshot before applying its own elapsed
  interval. It does not add the same interval to an already-corrected machine.
  Saving a corrected device anchors that state at the new UTC time.
- Disabled synchronization retains frozen-clock resume. Invalid/ambiguous,
  future-dated or more-than-366-day intervals are skipped with visible status.
  Bad state integrity/ROM identity still rejects the load atomically. A host
  forward-clock correction cannot be distinguished from actual absence; users
  can disable this optional behavior. No network time service is used.

## Compatibility

This table concerns the new clock feature, not existing device/playability support.

| Device | Backend | Automatic clock synchronization |
| --- | --- | --- |
| Tamagotchi P1 | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi P2 | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Genjintch | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Angel / Angelgotchi | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Morino | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Umino / Ocean | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Yasashii | E0C6200 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Connection V3 EU | SPLB32 | Disabled: no separate verified calendar mechanism |
| Tamagotchi Tama-Go (experimental) | GPLB52320A | Disabled: no separate verified calendar mechanism |
| Tamagotchi On | TG18 | Enabled only for Magic EN v058; other On firmware is unverified |
| Tamagotchi Meets Fairy | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi Meets Magical | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi Meets Fantasy | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi Meets Pastel | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi Meets Sanrio | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi Meets Sweets | TG18 | Disabled: time advances, but date rollover fails in tested snapshots |
| Tamagotchi P's | S1C33 | Disabled: visible software clock does not follow a calendar-only change |
| Tamagotchi iD | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi iD Lovely Melody | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi iD L | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi iD L English | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi iD L 15th Anniversary | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi iD L Princess Spacy | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi Plus Color | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi Plus Color Hexagontchi | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi 4U+ | S1C33 | Disabled: no verified firmware clock-setting mechanism |
| Tamagotchi 4U+ English | S1C33 | Disabled: no verified firmware clock-setting mechanism |

Meets v030/v031/v047/v055/v056/v062 passed the new storage/UTC plumbing tests,
but a 25-hour jump left their LCD date at 1/1 while time advanced from about
00:03 to 01:03. Direct use of the reference clock reader reproduced this, including
with a later native checkpoint. They are deliberately **not** enabled. Do not
interpret hardware RTC counters or successful save tests as proof of correct LCD
time. On Fairy EN v057 and Wonder Garden EN v063 were not firmware-validated.

For P's, changing the seven BCD calendar registers by 17 hours left its newly
opened LCD clock at the previous date/hour, even after normal minute progression
and sleep/wake. An additional test held the documented RTC calendar stop bit:
the hardware calendar stayed fixed while the visible software clock continued
counting seconds. This establishes that calendar registers alone are insufficient.
The exact firmware clock-setting/cache path remains unresolved. Replaying shared
periodic interrupts or patching guessed software-clock fields is not implemented.
Other S1C33 firmware is not assumed compatible on the basis of a shared CPU.

E0C6200, SPLB32 and the current GPLB52320A model expose shared periodic timer/timebase
interrupts rather than an independently verified firmware calendar. Their clock
behavior and saves are unchanged. Smart research is not part of this feature.

## Evidence and regression tools

`app/examples/clock_resume_validation.rs` exercises the real production save
coordinator with an injected UTC clock and private firmware: fresh save/close,
restart, a 25-hour jump, unchanged immediate CPU/RAM/time/IRQ state, visible clock
captures, repeated reopen, Save on Exit/restart, three exact manual slots,
import/export, opt-out and legacy loading. On Magic v058 passed from awake and
sleeping snapshots, and a 211-day jump displayed the correct January 1 rollover.
A separate 17-hour real-firmware probe also displayed the correct time.

`app/examples/clock_resume_probe.rs` retains the isolated RTC/calendar research,
including the P's stop-bit experiment and optional existing firmware reader.
Both tools require a new external output and matching `RETROHATCH_DATA_DIR`,
refuse personal RetroHatch AppData, and rehash source ROMs/checkpoints unchanged.
They emit private PNG/state diagnostics; no firmware or captures belong in Git.

```text
cargo build --release --locked --example clock_resume_validation
clock_resume_validation ROM DIAGNOSTIC_CHECKPOINT NEW_PRIVATE_OUTPUT [ELAPSED_MS]
cargo test --locked --example clock_resume_probe
```

Synthetic tests cover exact UTC/DST/year boundaries, atomic RTC range checks,
clock offset and phase preservation, legacy/default/opt-out settings, optional
metadata integrity, duplicate resume, unchanged manual restores and deferred
clock-refresh snapshot continuity. Firmware graphics are visually inspected;
synthetic tests alone do not establish model compatibility. Private evidence is
under external `local/test-data/clock-practical-20261119/`; no personal AppData,
original ROM, personal save or old diagnostic checkpoint was modified.

Relevant code: `app/src/host/resume_clock.rs`, `emulator.rs`, `saves.rs`,
`settings.rs`, `runner.rs`, `app/src/ui.rs`, and the attributed MIT TG18 core's
`machine.rs` and `snapshot.rs`. Upstream behavior was inspected in local
`tg18-emu-main/core/src/save.rs`, `machine.rs`, the prototype's clock-reader
comments and desktop `change_clock`. RetroHatch does not adopt upstream's forced
sleep/button clock-setting path or its replacement of toy time with Windows time.

## Release verification

- Formatting and locked Cargo checks, including all targets: passed.
- Complete sequential Rust suite: 205 library and 40 UI/binary tests passed;
  one pre-existing interactive browser test remains ignored. The attributed
  TG18 core passed 12 tests; the diagnostic tool passed two tests.
- The existing time-sensitive P1 worker test timed out once; its isolated and
  complete reruns passed unchanged. No tests or deadlines were weakened.
- Existing Python MCU, S1C33 C-reference and buzzer comparisons passed (391
  MCU checkpoints, 1,889 CPU/memory/NOR/LCD cases, 8,776 PCM samples).
- License inventory verified 106 locked Windows dependencies. License,
  packaging, automatic-release and workspace-path regression tests passed.
- The distribution wrapper built the Windows EXE with a static C runtime and
  private build-path remapping; packaging verified Windows system-DLL imports.

Private firmware/save/LCD evidence is not included. Version changes from
0.10.1 to 0.11.0; the existing automatic normal-release pipeline publishes only
after its own independent build, test, license and source-distribution gates.
