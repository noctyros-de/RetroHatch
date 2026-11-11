<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Japanese Tamagotchi Meets editions

The six editions reuse RetroHatch's existing MIT TG18 ARM926 backend. No second
core, frontend, shell artwork or firmware assets are introduced. The unchanged
local upstream README lists all six; `core/src/roms.rs` corroborates the image
identities by FNV-1a of the first 0x1E0000 code bytes. Exact full-image SHA-256
is used for automatic RetroHatch edition assignment. Filenames are not evidence.

## Verified images

All six are 8,388,608-byte SPII images with the shared 96-MHz nominal TG18
execution clock, 128x128 RGB565 LCD, GPIO 0x15/0x16/0x17 A/B/C inputs, timer-driven
audio and 32,768-Hz RTC. Firmware signatures locate edition-specific routines
and idle/clock fields using the existing upstream detection. No edition required
a guessed GPIO map, patched firmware or new hardware algorithm.

| Edition | User-provided filename | SHA-256 |
| --- | --- | --- |
| Tamagotchi Meets Fairy | `fw_tg18_jp_v030_2018-10-19_fairy.bin` | `d4639d3ecf464762c7911dccae6404ee3737c9bea71bd9fc43b20a6ad1e53b8a` |
| Tamagotchi Meets Magical | `fw_tg18_jp_v031_2018-10-19_magic.bin` | `ab2cc5c488b3b0bce2b5d5598b148bf3105394535d9ce4f1b1d646dfd31c6953` |
| Tamagotchi Meets Fantasy | `fw_tg18_jp_v047_2019-05-28_fantasy.bin` | `315c473af925b47b3025db1ba4b3c66d923a4c3eadbce877065e22d255a1e587` |
| Tamagotchi Meets Pastel | `fw_tg18_jp_v055_2019-03-10_pastel.bin` | `b527e68dcdf10cf4b56228512f14b6e587be6e73ce8ef5abc1a7eb91b1ebccac` |
| Tamagotchi Meets Sanrio | `fw_tg18_jp_v056_2019-04-24_sanrio.bin` | `43479ce1253f4ae6c8f3c25fce9f0d091e3a6c2400a68e707208e90334be3ce7` |
| Tamagotchi Meets Sweets | `fw_tg18_jp_v062_2019-09-12_sweets.bin` | `1b0972205bc104183e2406510f264d7ed4193bb5dc08c4811447ab986bbd2305` |

## Runtime validation

Each image boots from its original contents, completes setup and hatching,
produces its own graphics/character/menu content, and receives 74 real A/B/C
transitions during a 202-emulated-second sequence. Standby is observed for every
edition and the final input sequence wakes it. Private LCD frames show the baby
characters and menus; an ordinary clock screen at the final frame is a firmware
UI choice, not a framebuffer or model-identification failure.

Each image also runs through the actual worker and Windows PCM output, using an
isolated library and private working saves. Three slots, export/import, Save on
Exit, new-worker display restoration, pause/reset/resume and unchanged original
firmware are checked separately. Zero sample drops and zero empty-queue events
occurred in every measured run. This verifies the digital/output pipeline, not
acoustic equivalence to a physical toy.

| Edition | Monotonic seconds | Emulated seconds | Speed |
| --- | ---: | ---: | ---: |
| Tamagotchi Meets Fairy | 10.015 | 10.050 | 100.35% |
| Tamagotchi Meets Magical | 10.018 | 10.061 | 100.43% |
| Tamagotchi Meets Fantasy | 10.011 | 10.046 | 100.35% |
| Tamagotchi Meets Pastel | 10.017 | 10.050 | 100.33% |
| Tamagotchi Meets Sanrio | 10.014 | 10.047 | 100.33% |
| Tamagotchi Meets Sweets | 10.015 | 10.056 | 100.41% |

The short samples include worker publication/endpoint quantization; no timer
values, clock scaling or nominal CPU frequency were changed to obtain them.
The restored RTC advances consistently with emulated time, within one 32768-Hz
tick, in all six actual-image worker tests. Standby/alarm/button handling uses
the existing TG18 power-cycle path. No advancement of hunger/age while closed
or new host-clock synchronization feature is claimed.

## Save compatibility and timer correction

Each edition has a canonical profile ID and new explicit save code 19..24.
Existing profile codes (including generic On/Meets code 6 and Yasashii 18),
schema-2 payloads and ROM content hashes remain unchanged. Different editions
and images receive different save directories. Matching generic code-6 saves
can be loaded by the new profiles; generic On can import the same-image edition
states. Other edition codes still fail direct cross-profile loads, and every
state must match the original ROM SHA-256 and integrity checksum.

When a new edition lacks a particular autosave/slot, the host may read the
matching legacy `tamagotchi-on-meets-SHA256` file. New writes always go to the
new edition directory. Corrupt existing new files do not silently fall back.
Tests verify the legacy bytes are unchanged. Existing generic library records
are retained; reimporting an exact verified image prefers its dedicated edition
and reuses the content-addressed ROM copy without deleting older associations.

Real firmware exposed an upstream snapshot issue: `timers_resync()` changed
already serialized timer counters on restore (for example, a stopped sound
timer count of 50 became zero). The adapted MIT snapshot reader now restores
the stored counters/IRQ flags exactly. A focused synthetic regression protects
this correction and actual-image comparisons verify all serialized emulated
fields and future PCM. Only the upstream wall-clock capture timestamp is excluded
from semantic equality; sparse zero pages are sorted without dropping data.
No timer fields, RAM, LCD, RTC or emulated clock fields are ignored.

## Final application and regression checks

All six editions were automatically imported and launched in the native Windows
release application, then closed and reopened without a new ROM dialog. Titles
matched their dedicated profiles; 460x660 captures showed the original color
LCD and unclipped A/B/C controls. Saved gameplay time advanced beyond the
202-second seed in each pair of application sessions. Private captures and
library indexes are not part of the source tree.

`cargo fmt --check`, `cargo check --locked`, `cargo test --locked` and the locked
Windows release build pass: 209 Rust tests passed, with one existing interactive
browser check intentionally ignored. All 25 Python tooling tests and ten
MCU/device/S1C33/geometry/audio checks pass. The 106-package license inventory is
synchronized. The original upstream source hashes are unchanged; reused CPU,
system/peripherals, scheduler, signatures and ROM metadata are byte-identical
to the local reference. Only the adapted snapshot restoration changed here.
A separate actual English On Magic v058 smoke run also passed boot, input,
PCM generation and complete state/future PCM restoration; this is not a fresh
complete On gameplay acceptance claim.

## Reproduction and limits

```text
cargo run --release --locked --example tg18_probe -- ROM NEW_PRIVATE_OUTPUT 202 meets-fairy PRIVATE_INPUT_SCRIPT
cargo run --release --locked --example meets_validation -- ROM 10 --profile meets-fairy --root NEW_PRIVATE_DATA --resume PRIVATE_STATE --check-reset
```

Substitute the corresponding edition ID. No firmware/input-derived images or
personal saves are supplied by the project. Ordinary Cargo tests use synthetic
programs and require neither private references nor firmware. Controls remain
Left=A, Down=B, Right=C; Up has no effect. UI scaling, screenshots, recording,
themes and menus use the existing color-device path.

Validation is focused offline play, not exhaustive lifetime/minigame coverage
or a new acceptance campaign for every existing On/color/monochrome revision.
Bluetooth, infrared and online services remain unsupported. Existing upstream
MIT copyright/license notices are retained; references are never modified.
