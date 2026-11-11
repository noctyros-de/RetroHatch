<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Yasashii implementation and TamaOtch research boundary

## Firmware evidence

Both user-provided images contain 8,192 unpacked, big-endian 12-bit words,
16,384 bytes. Original files remain unchanged and outside the repository.
These locally calculated fingerprints identify the tested images, not every
regional revision or a certified provenance/authenticity database.

| Research label | SHA-256 | Execution status |
| --- | --- | --- |
| Tamagotchi Yasashii | `6722497ff4ce7c12fb380da31a067485ad8e42e0de156d770e426c9059d5eb6c` | Original firmware boot, hatching and focused input sequences verified |
| Tamagotchi TamaOtch | `b4eecf69ef28de62c4b4270c5773f229e1ba9591cfd6669f2d14e0adbd8e7481` | Recognized but deliberately not executed with a substitute board |

## Yasashii: established board configuration

Ground truth is the unchanged local BrickEmuPy
`assets/TamagotchiYasashii.brick`, `assets/TamagotchiYasashii.svg` and
`cores/E0C6200.py`, declared CC0-1.0 by upstream. The profile selects E0C6200,
1,060,000 Hz fast clock, 32,768 Hz MCU timebase, 768 RAM nibbles, K0/K1
pull-ups 15, R0-R4 push-pull masks 15 and P3 dedicated mask 0. This establishes
the software configuration, not an independently inspected physical die revision.
No separate RTC chip or tap sensor is specified. Timekeeping uses the existing
MCU timers/interrupts and deterministic oscillator phases, including HALT.

| Input | Electrical connection | RetroHatch key |
| --- | --- | --- |
| Food | K1.3, active low | 1 / Q |
| Smile | K1.2, active low | 2 / W |
| Heart | K1.1, active low | 3 / E |
| Clean | K1.0, active low | 4 / A |
| Light | K0.3, active low | 5 / S |
| Status / scale | K0.2, active low | 6 / D |
| Select | K0.1, active low | Left Arrow / Z |
| Cancel | K0.0, active low | Down Arrow / X |

The reference has eight controls, not three buttons plus a guessed knock input.
Mouse controls expose all eight. Focus loss, reset and state restoration release
host inputs; each electrical release allows the real pull-up to act. Arrow Up
and Arrow Right have no Yasashii gameplay effect. Other models retain their
existing A/B/C and sensor shortcuts. The scale/status key confirms clock setup;
several Select presses alone leave the firmware in clock-setting mode.

The SVG defines exactly **512 matrix segments and no fixed LCD icons**. Its six
other display groups are GPIO indicators: P0.0/P1.0/P2.0 and R0.0/R1.0/R2.0,
in Food/Smile/Heart/Clean/Light/Status order. The LCD geometry extractor preserves
every original bit assignment, orientation and coordinate under a uniform scale
(normalized pixel side 10 units; bounds 351 by 175). Original shell, gradients,
button art and LED paths are not copied. Plain host circles indicate the GPIO
states adjacent to their care controls. LCD-only screenshots contain only the
matrix. Existing exact icon assets for other devices are untouched; their
previously documented provenance issues are not resolved by this addition.

`core/mcu/io.rs` reconstructs the R push-pull mask from the device profile.
Existing boards retain their open-drain masks. Reset reapplies the mask; save
restore builds a correctly configured board before decoding mutable state.
The full classic schema-1 payload stays unchanged, with new explicit profile
code 18. Identity remains profile plus original ROM SHA-256, so Yasashii cannot
load an Angel or other device's state even for identical program bytes. All
slots, autosave, state import/export, audio and real-time pacing use existing
host infrastructure. No firmware bytes or LCD character sprites are embedded.

## Validation

On Windows, the actual local Yasashii image:

- Boots and displays its egg in the native RetroHatch window; the original LCD
  mapping then shows clock setup, hatching and the baby character during a
  420-emulated-second sequence. Normal input is used; no firmware RAM is patched.
- Matches unchanged BrickEmuPy CPU registers, PC/NPC/flags, all RAM and LCD RAM,
  emulated time, oscillator/display control and six GPIO indicators. A short
  run covers 3,841 checkpoints and 20 input transitions; a longer run covers
  1,681 checkpoints, 40 input transitions, 13,478,485 original MCU clocks and
  41 distinct LCD states, including post-hatching care/select/cancel inputs.
- Runs 60.017 monotonic wall-clock seconds versus 60.048 emulated seconds
  (approximately 100.05%). Windows PCM output is active with zero dropped
  samples and zero empty-queue events. Shared buzzer waveform regressions cover
  the upstream signal; physical-device acoustic fidelity is not measured.
- Preserves exact serialized CPU/MCU state and future PCM/interrupt/timer/LCD
  continuation across a separate process, including three slots and autosave.
  Worker tests cover import/export, Save on Exit, new-worker resume, pause,
  reset and resume. Native application restart is also exercised with isolated
  data directories, preserving the original image.
- Has focused synthetic tests for every input, K1 IRQ generation, output masks,
  reset/restore, save identity, 512 unique segments, and visible controls at
  350x500, 460x660 and 640x800 across 100/125/150/200% scaling.

Final checks pass: `cargo fmt --check`, `cargo check --locked`,
`cargo test --locked` (207 passed, one existing interactive browser test ignored),
and the optimized locked Windows release build. The 25 Python tooling tests,
existing MCU/device differential checks, 1,889 S1C33 C-oracle cases, all five
existing icon-mask checks and 8,776 buzzer PCM samples also pass. Actual-image
save continuation is checked for all seven classic profiles; color-device
regressions run without claiming a fresh gameplay campaign for every color ROM.
The release build script also remaps generated sources in the external Cargo
output directory, removing private workspace paths from the executable.

These checks do not certify every minigame, full character lifetime, every
possible firmware revision or physical oscillator accuracy. Private traces,
captures and gameplay saves are outside the public repository. Reproduce with
user-provided firmware only:

```text
cargo run --release --locked --example device_trace -- yasashii ROM PRIVATE_TRACE --gameplay --state PRIVATE_STATE
python -B app/tools/compare_yasashii.py --rom ROM --trace PRIVATE_TRACE
cargo run --release --locked --example yasashii_validation -- ROM 60 --check-reset
python -B app/tools/extract_yasashii_lcd.py --check
```

The Python comparison/extraction needs the optional private BrickEmuPy checkout;
ordinary Cargo builds and synthetic tests require neither references nor ROMs.

## TamaOtch: unresolved evidence, not playable support

The current BrickEmuPy checkout contains no TamaOtch device profile or SVG.
[TamaLIB](https://github.com/jcrona/tamalib) and
[TamaTool hardware documentation](https://github.com/jcrona/tamatool/blob/master/README.md)
attribute TamaOtch to Epson E0C6S48, which contains the E0C6200 CPU. This supports
reusing the instruction interpreter; it does not establish the board's exact
MCU mask settings, fast oscillator, GPIO/sound-sensor wiring or pulse behavior.
The image's valid 12-bit word layout likewise does not reveal electrical LCD
wiring or the original icon contours.

The exact LCD matrix/icon mapping and symbol geometry, sensor pins/polarity,
board masks and measured/documented fast clock remain unavailable. No Angel,
Genjintch, Yasashii or Umino display, clock or pin layout is substituted. No
new GPLv2 TamaLIB/TamaTool code or assets are incorporated. Existing GPL-3.0-only
and third-party attribution documents remain in force.

The existing real-ROM acceptance exposed a pre-existing host-dispatch regression:
direct worker loads no longer applied the pending-device guard. The constructor
now enforces the same exact-hash refusal before preparing any replacement, so a
failed load preserves the running device state. The real-image acceptance checks
that boundary as well as the unchanged library index and original firmware.

The known TamaOtch fingerprint is refused before import/execution, including
attempts to force the new Yasashii profile; existing library entries and saves
are preserved. It is not advertised in supported-device selection or the README
support table. Boot, LCD, inputs, sound, timing and save continuation for this
model remain **unverified and blocked**, not hidden behind placeholder controls.
The missing hardware/display evidence must be supplied or established before
an accurate executable profile and real-ROM tests can be implemented.
