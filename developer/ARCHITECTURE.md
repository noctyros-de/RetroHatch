<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros -->

# Architecture for contributors

The emulation core is independent of egui and host filesystem/wall time. A
shared E0C6200 instruction interpreter implements the non-reserved reference
encodings, with explicit atomic errors for unsupported operations. MCU I/O,
timer/interrupt scheduling, oscillator phases and audio generation live above
instruction execution. Emulated time uses integer master ticks; the worker
paces this deterministic state against a monotonic host clock.

`DeviceProfile` describes P1, P2, Genjintch, Angel, Morino and Umino. P2 shares
P1 hardware and LCD because the reference explicitly confirms it. Sensor models
keep their own electrical display mappings. Profiles have stable canonical IDs;
save codes are explicit (P1=0, Genjintch=1, Angel=2, Morino=3, Umino=4, P2=5).
TamaOtch is a pending research descriptor outside the executable profile list.

The single-owner worker processes commands, advances bounded emulated work,
and publishes a latest immutable snapshot. Native Windows PCM queues and file
dialogs are host services. File writes use same-directory atomic replacement
and backups. egui paints actual LCD state, handles input/focus release, and
exposes menus/settings without changing the deterministic clock.

Full save files have a versioned container/schema, explicit field encoding,
ROM SHA-256, profile identity and integrity checking. They include CPU, RAM/LCD,
I/O/timer/oscillator/interrupt state, audio generation and emulated time. Loading
prepares a compatible replacement before changing the active device. Manual
slots and separate autosave are per-ROM/profile, with host settings independent
of machine state. ROM contents are never embedded in a save.

The local ROM library stores content-addressed private copies and a versioned,
checksummed index under Windows user data. Exact catalog hashes and remembered
choices can identify a profile; byte length only narrows candidates. Unknown
content requires confirmation. Index changes do not erase saved games. ROM
bytes are verified again before starting the worker. SHA-256 is a content
identifier, not an assertion that every possible regional variant is known.

Rust tests contain synthetic programs and stored reference expectations. Optional
Python tools regenerate/check expectations using a separately obtained local
BrickEmuPy source tree. Cargo compilation/tests require no proprietary firmware
or private reference directory. Current generated LCD data can compile by itself,
but the original editable vector inputs are not in the proposed source snapshot;
resolving their rights and source-form availability is a release gate.

Privacy boundaries: `docs/` contains private research/audits; `developer/` is the
public documentation area. Keep ROMs, saves, library indexes, credentials and
diagnostic captures outside public source. Do not publish `.git` or tool refs.
