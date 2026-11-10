<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros -->
# UI polish, display sleep and Angel audio

The existing five-theme layout and menu structure remain. A tinted menu-bar
background and fine lower border separate navigation from content. Settings
uses an 8-point volume rail, accent fill, outlined handle and percentage.
File retains Import ROM; Manage ROMs retains management/repair/index recovery
and no longer duplicates Import ROM.

## P's display sleep

The observed black display is firmware standby, not missing framebuffer data.
The local P's image issues SLPIN (sleep_out=false), keeps its GRAM and switches
MCLK down to 16384 Hz. The same CPU-cycle run of unchanged tamaemu produces
the same RAM/I/O/GRAM state and black rendered panel. Buttons wake it normally.

Settings → Keep screen awake is optional, persistent and P's-only, off by default.
It applies tamaemu's existing --stay-awake policy: clear the board's sleep flag
at A0RAM 0x146C and permit rendering retained GRAM while the LCD sleep latch is
off. It does not fake button presses, accelerate time or affect other devices.
When enabling it after the firmware has already entered standby, wake once with
A/B/C. Normal sleep behavior resumes when disabled; the next firmware timeout
determines the actual screen-off time. A 40-emulated-second run with the policy
enabled keeps sleep_out/display_on true and the original color framebuffer.
The normal reference run retains GRAM and enters display sleep. The diagnostic
example's --emulated-time flag accounts for dynamic MCLK; its default nominal
cycle checkpoints remain useful for exact C comparisons.

This is a host preference, excluded from machine-state schemas. The worker
reapplies it after load/restore; reset preserves the preference. Existing
checksummed settings with eleven fields decode with the new option disabled;
new settings have twelve fields. ROMs and existing save formats are unchanged.

## Angel sound measurements and correction

A local 15-second Angel firmware capture contains 661,506 PCM samples. Before:
58,158 nonzero samples, including 50,728 at the +/-655 limiter (87.2%). WinMM
reported zero empty-queue events and zero dropped blocks, maximum write gap
32.199 ms with the existing 80 ms priming. This establishes a source-distortion
problem in this capture, without evidence of transport underruns.

BrickEmuPy generates a sine at five times the final mixer's permitted peak,
then clips it. RetroHatch previously reproduced that behavior. Angel now uses
unity headroom before saturation, retaining peak range, frequency, continuous
phase, envelope levels and all sound-control/timer behavior. This is a targeted
presentation correction, not a new piezo/speaker hardware model. No filtering,
resampling, artificial clock acceleration or arbitrary crossfade is introduced.
Other classic models retain their existing reference waveform; ARM/S1C33
audio and Windows transport are unchanged.

After, the same firmware and duration produce 661,506 samples, 58,148 nonzero,
zero samples at the limiter and peak 654. The small nonzero-count change comes
from PCM16 quantization at the quieter envelope levels. WinMM again reports
zero underruns/dropped blocks, maximum gap 32.569 ms. A coherent 4096-Hz test
shows third-harmonic/fundamental amplitude above 0.2 before and below 0.001
after; both paths finish with identical oscillator and sound-control state.
Old saved PCM remains valid and may play its previously captured clipped tail;
new samples use the corrected Angel presentation after restore.

The captures and diagnostics live under ignored app/target; no firmware/audio
is included in source. `audio_diagnostics` is an opt-in local firmware probe.
An audible assessment on the user's output hardware remains a human check;
the measured saturation is removed, while legitimate buzzer on/off transients
are preserved. These results do not exclude every possible driver/device issue.

Focused tests cover source headroom/phase, legacy waveform parity, settings
migration, the worker's awake command, sleep/GRAM isolation, ROM import menu
placement and existing theme/input/library behavior. Full Rust regression and
the relevant Python MCU/Angel/buzzer comparisons are run for delivery.

## Changed files and delivery checks

- `app/src/ui.rs`, `app/src/ui/library_ui.rs`: navigation/slider styling,
  P's setting and import-menu cleanup, with a focused interaction regression.
- `app/src/host/settings.rs`, `runner.rs`, `emulator.rs` and
  `emulator/tests.rs`: persistent optional display policy and worker dispatch.
- `app/src/core/s1c33/machine.rs`, `tests.rs`: optional reference-derived awake
  behavior, reset preservation and isolation tests.
- `app/src/core/mcu/buzzer.rs`, `mod.rs`: Angel presentation headroom and
  phase/control/harmonic tests; unchanged legacy waveform path.
- `app/examples/audio_diagnostics.rs`, `s1c33_boot.rs`, `save_acceptance.rs`:
  local measurement, checkpoint capture before replay-test button presses,
  dynamic-clock diagnostic timing and the additional settings field.
- `developer/S1C33.md`, this document: behavior, scope and measured results.

The full Rust suite passed 172 tests (one existing browser test ignored);
the subsequently added worker-policy test also passed. Formatting and locked
all-target compilation pass. The rebuilt legacy PCM oracle matches all 8,776
samples exactly; the 391 MCU and 236 Angel reference checkpoints pass.
The optimized Windows release build and isolated native-window launch pass.
The menu-bar tint was visually inspected; interactive menu behavior is covered
by egui tests. Subjective listening on the user's hardware remains unverified.
Original firmware, reference sources and personal settings/library are untouched.
