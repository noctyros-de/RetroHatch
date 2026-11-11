<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Tamagotchi 4U+ integration

Both tested images are complete 8,388,608-byte NOR dumps with reset vector
0x024F6256 at offset 0x400000. Their SHA-256 values are recorded as exact verified
identities in `host/fingerprints.rs`; filenames alone never select a model.
They differ in 376,878 bytes across 43 64-KiB blocks, including data and code areas.
The EN image visibly replaces Japanese instructions with NAME, PROFILE, MOOD and
SAD; Japanese kana input and some input labels remain. It is treated as a translated
image, not a verified official English release. Patch author, patch history and
translation completeness are unknown. No patch was obtained or applied.

## Shared hardware and host integration

The existing tamaemu `device.c` key `4u` denotes 4U+, not the original `4u-plain`.
It supplies 30-MHz OSC3, 32,768-Hz RTC, 8-MiB NOR at 0x02000000, reset table at
0x02400000, 32-KiB A0RAM, 12-KiB IVRAM and 4-KiB DSTRAM. The 128x128 LCD uses
0x00600000/1. A/B/C are active-low P00/P01/P02. Left/Down/Right and existing mouse
buttons are unchanged; Up has no gameplay effect and no Tap control is shown.
The firmware may consume the first standby button press solely to wake.

The English profile derives the same board, with a separate key/name/save code.
Both exact hashes select their profile automatically; unknown compatible images
still require confirmation. No P's/iD fallback is used for verified images.
Existing library, screenshots/video, scaling, volume, pause/reset and save menus
are shared. The firmware itself generates all LCD graphics; no artwork is added.

## PN512 and save architecture

The translated disconnected PN512 register/UART model implements power-up and
SoftReset values, register reads/writes and acknowledgements, bounded FIFO/reply
queues, CRC, memory buffer, deterministic PRNG, command/IRQ behavior and quiet-air
transmit deadlines. UART uses MCLK/16, correct TX vector 62 and paced RX delivery.
It provides no incoming RF frames, virtual phone/tag, device peer or DLC. Basic
controller behavior needed by firmware must not be confused with NFC connectivity.
Source/provenance remains maragotchi/tamaemu, recorded in UPSTREAM.json and the
existing GPL notices; the upstream licensing clarification remains separate.

Save codes 16/17 use schema 4, adding PN512 registers, buffers, partial writes,
reply queue, PRNG and deadlines after the existing S1C33 payload. Old S1C33 devices
retain schema 3; existing classic/ARM formats and identifiers are unchanged.
Hash/profile/schema/integrity mismatches fail before replacing the live device.
Original firmware is never overwritten; each source hash/profile has independent
slots/autosave. Tests include full snapshots and 10,000-step state/PCM continuation.

## Actual local validation (2026-11-11)

Both originals execute, render distinct firmware state, accept A/B/C, complete
setup/character creation, hatch and enter actual menus. The translated image
shows English profile/mood screens alongside the original Japanese equivalent.
Private runs reach 330 emulated seconds, idle standby and input wake with RTC
continuation. Both show the verified 0x1440 sleep marker (0 awake / 2 asleep).
Twenty one-second native/C checkpoints per image match PC/cycles, RAM/I/O/GRAM,
IRQ/LCD/tone counts and all 882,000 PCM samples, without filtering or tolerance.

Initial startup was slower than real time (about 1.31 wall seconds for the first
emulated second), causing audio overflow during catch-up. A firmware-independent
Windows queue probe passed. Exact zero-time/zero-sample audio fast paths and
inlining at CPU/bus boundaries reduced that startup measurement to about 0.92 s.
Clock rates, sample
values, queue limits and error handling were not changed; no samples were dropped.
Final production-worker checks pass: JP 60.010 real seconds / 60.045 emulated,
EN 10.012 / 10.040, active Windows playback, zero dropped samples and zero queue
underruns. Both pass Slot 1, import/export, Save on Exit, fresh-worker resume and
explicit pause/reset/run. A fresh P's firmware/reference/PCM regression and its
strict Windows-audio/save/reset checks also pass.

The optimized Windows 0.6.0 EXE automatically selected each image without a model
argument, rendered the real Japanese/translated LCD, saved on normal close and
reopened its remembered library with advancing progress. Native captures were
inspected: no clipped controls and no sensor button. Final formatting, locked
check, 205 Rust tests (one existing browser test ignored), 25 Python tool tests,
the 1,889-case C oracle, original-iD diagnostic regression, 106-package license
inventory and static-runtime Windows release build pass. No commit/push/release
is performed as part of this development task.

These are focused local tests, not exhaustive lifetime/minigame coverage or
subjective loudspeaker certification. Original 4U, NFC connectivity and DLC are
not supported. Firmware, captures, PCM and private saves are outside the repository.
