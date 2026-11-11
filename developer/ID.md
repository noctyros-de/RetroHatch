<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Original Japanese Tamagotchi iD

## Hardware and integration

The board is the existing tamaemu `device.c` configuration: 4-MiB NOR at
0x00C00000, reset table at 0x00C00000, 18,432,000-Hz OSC3 and the shared CMU/RTC,
32-KiB A0RAM, 12-KiB IVRAM, 4-KiB DSTRAM, I/O at 0x00300000 and 128x128 color
LCD at 0x00600000/1. Buttons are active-low P0 bits 0/1/2 for A/B/C. There is no
tap sensor. The firmware sleep marker is 0xEFC; disconnected IR context is
0x842BC. No instruction, timer, GPIO, RTC, audio or LCD algorithm was changed.

The shared production worker provides pacing, audio, input, pause, reset,
release, settings, library and saves. The UI keeps Left=A, Down=B, Right=C;
Up has no gameplay effect. Exact SHA-256 identifies the locally verified image;
other structurally valid 4-MiB images require model confirmation. Filenames do
not determine a model. Firmware revisions have distinct content identities.

S1C33 schema 3 now dispatches save profile 8 to iD and retains profile 7 for
P's with the same existing payload. Full NOR, CPU, LCD, timers, RTC, IRQ, phase,
PCM and emulated time are restored; ROM/profile mismatch is rejected. Save
slots/autosave use the existing profile-plus-firmware-hash directories. Original
ROM files are never overwritten. Upstream .sav/.tamasave import remains unavailable.

## Local firmware validation (2026-11-11)

- October 2009 Japanese 4-MiB image: actual reset, LCD initialization and setup,
  A/B/C input, new-generation/hatch/name-entry and saved execution were exercised.
  The supplied dump initially resumes an existing end-of-life screen. A+C
  (Left+Right together) starts a new generation. The tested new game performs
  337 flash program operations and one erase; these changes are in emulated NOR,
  never the original dump. The resulting state resumes in the Windows EXE.
- Twenty one-second checkpoints match unchanged tamaemu C using its `id` board:
  PC/cycles, complete RAM/I/O/GRAM hashes, interrupt/LCD/tone counts match.
  All 882,000 PCM samples match exactly, without filtering or smoothing.
- Production worker: 60.016 wall seconds produced 60.044 emulated seconds,
  Windows PCM output active, zero reported audio drops. Library persistence,
  Slot 1, import/export, Save on Exit and fresh-worker resume passed.
- Actual full states round-trip and continue identically for 10,000 steps.
  Synthetic tests cover iD GPIO/RTC/LCD, model identity, manual association,
  wrong-ROM rejection and isolation from P's saves. Existing regressions apply.
- The Windows EXE launched with the real gameplay state, saved on exit and
  reopened its remembered library. Windows foreground restrictions blocked the
  automated native keyboard/screenshot harness; an egui integration test checks
  iD import, manual confirmation, color presentation and one-click library launch.
- Subjective loudspeaker quality, every minigame and a complete device lifetime
  are not certified by these measurements. IR, DLC and connectivity are unavailable.

## iD-family extension

See [ID_FAMILY.md](ID_FAMILY.md) for the five additional profiles, verified
Japanese archive/English short-image adapters and the still-unidentified
February-named 8-MiB image. The original iD board and profile code 8 remain
unchanged. That 8-MiB image is not an original-iD revision merely because its
filename says so; selecting the 4-MiB iD profile correctly rejects it.
