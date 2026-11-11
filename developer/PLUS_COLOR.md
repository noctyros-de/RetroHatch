<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Plus Color and Hexagontchi

Both devices reuse the native S1C33 CPU, bus, peripherals, LCD, flash, audio and
production worker. No new core or CPU/peripheral algorithm is introduced.
Profiles are the existing maragotchi/tamaemu `src/device.c` configurations:

| Setting | Plus Color | Hexagontchi |
| --- | --- | --- |
| Application key / save code | plus-color / 14 | plus-color-hexa / 15 |
| NOR / reset table | 4 MiB at 0x00C00000 | Same |
| OSC3 / RTC base | 20 MHz / 32,768 Hz | Same |
| RAM | 32-KiB A0, 12-KiB IV, 4-KiB DST | Same |
| Color LCD | 128x128 at 0x00600000/1 | Same |
| Retail P06 level | Low (debug high) | High (debug low) |
| Flash sectors | Top boot, final 64 KiB in 8-KiB sectors | Same |
| Disconnected GPIO IR | P03 TX / P05 RX idle high | Same |
| Reference keep-awake RAM marker | 0x368; valid only for tested later revision | 0x000, verified valid |

A/B/C are active-low P00/P01/P02. Left/Down/Right and the existing on-screen
buttons are unchanged; there is no tap sensor. Firmware controls CMU frequency,
RTC, standby and wake; observed MCLK transitions include 20 MHz, 10 MHz and
16,384 Hz. Nominal oscillator cycles are not treated as elapsed wall time.
Infrared device connections remain unavailable; correct disconnected GPIO levels
are distinct from implementing connectivity.

## Identification, library and saves

Four complete 4,194,304-byte local images were individually tested. The three
v040-named sources differ in contents, including code-area blocks, despite sharing
reset vector 0x00C2F0EC. The Hexagontchi source resets at 0x00C4215E and displays
its own number-themed room, icons and menus. Filename dates are test labels,
not independently authenticated manufacturing or build dates.

Only these exact SHA-256 values are automatically mapped in `host/fingerprints.rs`.
Unknown compatible 4-MiB images offer iD, Lovely Melody, Plus Color and Hexagontchi
for explicit confirmation; size/filename never proves the model. P's is not a
fallback for 4-MiB images. The older generic Plus Color file is content-identical
to the tested November source and therefore shares its hash identity.

The existing library preserves originals, remembers imports, deduplicates by hash
and keeps each revision/profile's saves separate. Schema 3 appends codes 14/15;
all existing profile codes and formats remain stable. Full NOR, CPU, RAM, LCD,
RTC/timers/IRQ/clock and PCM state round-trip and continue deterministically.
Slots, autosave, export/import, pause/resume and reset use the shared paths.
Reset restarts the CPU and retains emulated NOR progress; originals are untouched.

## Validation and limits (2026-11-11)

All four real images boot, accept A/B/C, pass setup, hatch, display live characters
and enter their actual menus. Firmware-generated pixels/icons use the existing
128x128 RGB path, not extracted artwork. Longer runs reach 350/356 emulated
seconds, normal display standby and button wake with RTC continuity. Full saves
round-trip and continue identically for 10,000 further CPU steps including PCM.

Each image matches unchanged C CPU/peripheral code at 20 cycle checkpoints:
PC/cycles, full RAM/I/O/GRAM hashes, interrupts, LCD commands and T0 counts.
All 5,762,211 captured PCM samples match exactly across the four runs. Diagnostics
now index checkpoints independently of elapsed time and replay actual reference
CMU clock transitions; they do not change emulator clocks or relax comparisons.

The production worker measured 60.016 real seconds to 60.035 emulated seconds for
the first image. The others passed ten-second checks near 100% speed. Windows PCM
output was active with zero reported drops. Each private library passed Slot 1,
export/import, Save on Exit and fresh-worker resume. Source hashes stayed unchanged.
These are objective output/timing checks, not subjective loudspeaker certification
or exhaustive minigame/lifetime verification.

The optional keep-awake override is disabled at the host capability boundary for
Plus Color: the October images keep unrelated 0x90/0x68 values at reference RAM
address 0x368 during awake and sleep phases. Clearing that byte would be unsafe.
The November image uses 0/2 there, but the model-wide policy stays conservative
until revision-specific markers are established. Normal sleep/wake is unaffected.
Hexagontchi uses the explicitly valid zero-address marker (0 awake / 2 asleep).
No speculative replacement address is used. Existing models retain their policies.

Firmware, saves, screenshots and PCM diagnostics remain in the private workspace.
No firmware or new artwork is included. Existing upstream licensing/provenance
questions in THIRD_PARTY_LICENSES.md remain separate from local engineering tests.

The optimized 0.5.0 Windows EXE was opened with each original file and no profile
argument: exact fingerprints selected the correct model automatically. Normal
close and argument-free reopen restored each isolated library and advancing
save. Application-only window captures were inspected for all four sources;
LCD, controls and the fixed footer fit the existing 460x660 layout.

Final verification: cargo fmt --check, cargo check --locked, cargo test --locked
(200 passed, one pre-existing interactive-browser test ignored), the 1,889-case
synthetic C oracle, original-iD clock-diagnostic compatibility, 25 Python tooling
tests, license evidence for 106 packages and static-runtime Windows release build
passed. No new dependency was added. The working version increased once from
0.4.0 to 0.5.0; no commit, push, tag or release was performed.
