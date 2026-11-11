<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Architecture for contributors

The classic emulation core is independent of egui and host filesystem/wall time. A
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
of machine state. Classic states contain no original ROM. TG18 snapshots store modified flash
as an XOR delta against the required user ROM, plus RAM/SRAM, color framebuffer,
timers, RTC, peripheral state and synthesizer phase/cursor. They are private
runtime data and may contain firmware-derived graphics/code; never publish saves.

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
but the original editable vector inputs are not in the public source snapshot;
resolving their rights and source-form availability is a release gate.

Privacy boundaries: most `docs/` content is local research/audits. The public
exceptions are docs/assets/retrohatchlogo.png and docs/screenshots/; developer/
contains public technical guides. Keep ROMs, saves, library indexes, credentials and
diagnostic captures outside public source. Do not publish `.git` or tool refs.


The host `Emulator` dispatches to the unchanged classic device or the separately
licensed TG18 ARM library. TG18 uses its upstream 96-MHz nominal instruction-time
model and 32,768-Hz RTC, mapped onto the worker's common master-tick timebase.
The host initializes RTC from local time once, then advances it using emulated
time. It does not speed up the RTC to disguise execution lag. No offline pet
growth or closed-application clock advance is implemented. Work and input remain
on the same single-owner worker, independent of UI focus/repaint frequency.
ROM content and device profile isolate state directories. Classic profile codes
0..5 and their schema remain unchanged; On/Meets appends code 6, backend schema 2.
TG18 capture runs on the owner, while compression and ordered atomic writes run
on the existing I/O worker. See [TG18.md](TG18.md) for verification limits.


## Native S1C33 backend

`core/s1c33/` owns a separate CPU, RAM/NOR bus, LCD controller, CMU, timers,
GPIO/ITC/RTC and cycle-stamped audio. P's is dispatched by `host/emulator.rs`;
the existing worker owns all execution and input. It uses the shared color
framebuffer, PCM output, library, menus and clock-independent pacing.

P's appends profile code 7 and uses backend save schema 3. Classic schema 1 and
ARM schema 2 retain their formats and identities. Coherent machine capture occurs
on the emulation worker; explicit encoding, hashing and atomic writes occur on
the ordered save worker. See [S1C33.md](S1C33.md) for verification and limits.

The iD family uses this same S1C33 owner and hardware implementation: iD code 8,
Lovely Melody 9, iD L 10, English 11, Princess Spacy 12 and 15th Anniversary 13.
Only verified import adapters normalize source data in memory; identity hashes
always cover original imported bytes. Normalized mutable NOR is fully stored in
schema 3. Existing schema and profile numbers are stable. See [ID_FAMILY.md](ID_FAMILY.md).

Plus Color and Hexagontchi reuse their existing S1C33 board data, including a
20-MHz oscillator, reversed retail/debug-strap polarity, idle GPIO IR and top-boot
NOR. Codes 14/15 append model identities without altering existing saves. Model
capabilities gate the optional keep-awake policy; it is disabled for Plus Color
because early firmware revisions do not use the reference's 0x368 RAM marker.
Normal power management is always firmware-controlled. See [PLUS_COLOR.md](PLUS_COLOR.md).
