<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# iD and iD L family integration

## Hardware and identity

All profiles reuse the existing native S1C33 CPU, peripherals, LCD, buzzer,
production worker and host UI. No CPU or peripheral timing algorithm was changed.
The reference is maragotchi/tamaemu `src/device.c` and its existing shared board.
All use 18.432-MHz OSC3, 32.768-kHz RTC/OSC1, 128x128 LCD and active-low P0 bits
0/1/2. Left/Down/Right and on-screen A/B/C are unchanged; there is no tap sensor.

| Profile key | Name | NOR / reset table | Save code | Optional keep-awake marker |
| --- | --- | --- | --- | --- |
| id | Tamagotchi iD | 4 MiB at 0x00C00000 / 0x00C00000 | 8 | 0xEFC |
| id-melody | Tamagotchi iD Lovely Melody | Same 4-MiB map | 9 | 0x11E0 |
| idl | Tamagotchi iD L | 8 MiB at 0x02000000 / 0x02400000 | 10 | 0xFD8 |
| idl-en | Tamagotchi iD L English | Same 8-MiB map | 11 | 0xFD8, confirmed during local sleep |
| idl-princess | Tamagotchi iD L Princess Spacy | Same 8-MiB map | 12 | Unknown; override disabled |
| idl-15 | Tamagotchi iD L 15th Anniversary | Same 8-MiB map | 13 | Unknown; override disabled |

The special-edition bus, LCD and normal peripherals match the reference iD-L
board in actual execution. No guessed optional sleep RAM address is used.
These are distinct device/save identities, not duplicate cores. Existing P's
code 7, iD code 8, classic/On schemas and state payloads are retained. Full mutable
NOR, CPU/RAM, LCD, RTC/timers/IRQ, oscillator, PCM/phase and emulated time resume
through schema 3. Original imported bytes determine SHA-256, including for the
normalization cases below. Wrong profile/hash loads fail without state changes.
No original firmware or existing player save is overwritten during validation.

## Verified import forms and unresolved identity

Exact local hashes in `host/fingerprints.rs` identify October 2009 iD, Lovely
Melody, Japanese/English iD L, Princess Spacy and 15th Anniversary. The catalog
contains identity metadata only, no firmware. Arbitrary structurally valid dumps
require explicit confirmation. New verified evidence conflicting with an older
remembered guess asks for selection and keeps the old association/saves intact.

The February-named image has 8 MiB, an erased first sector, and reset vector
0x024CA942 at offset 0x400000. Original iD instead uses a 4-MiB NOR and reset at
its beginning. The February image executes with the iD-L map and displays a
clover-themed room/birthday scene; that is insufficient to establish its exact
model or revision. Its provenance is unconfirmed. It deliberately has no catalog
model assignment and is not advertised as an original-iD revision. Compatible
8-MiB P's/iD-L choices are available instead of forcing P's or a 4-MiB iD board.

Two exact-hash import exceptions are necessary for the available test sources:

- Japanese source: a 6,039,638-byte ZIP disguised as `.bin`, containing one
  `japanese/IDL-JAP-ROM.bin` of 8,387,584 bytes. Only this bounded NOR member is
  decoded in memory; names are never extracted as filesystem paths. Deflate,
  length, CRC and the exact member SHA-256 are checked. Other artwork/files are
  not extracted. The same verified raw member may be imported directly.
- English source: an 8,386,560-byte NOR dump, 2 KiB short of the 8-MiB bus.
- Upstream `main.c` allocates NOR with `calloc()` and reads up to its configured
  size. Matching that behavior appends 1 KiB / 2 KiB of zero bytes only for the
  exact tested Japanese / English hashes, in memory. This is compatibility with
  the reference loader, not recovery or proof of omitted physical flash bytes.
- Unknown archives/short dumps are rejected. Original library copies remain
  byte-identical to the source, index sizes remain source sizes, and saves carry
  the entire normalized NOR. Importing differently normalized source bytes has
  a different hash and intentionally does not merge saves.

## Local validation (2026-11-11)

All seven available files, including the unknown February image on a manually
selected iD-L board, passed actual execution with A/B/C press/release schedules:

- Twenty one-second C/native checkpoints agree in PC, cycles, complete RAM/I/O/
  GRAM hashes, IRQ/LCD/T0 counts. All 882,000 PCM samples per image match exact
  C tone events; no filtering or smoothing was introduced. The C diagnostic now
  exports binary32 frequencies with enough decimal precision for exact replay.
- Real 128x128 frames were inspected locally. Lovely Melody hearts, Japanese
  and English standard rooms, Princess decorations and anniversary icons/setup
  are firmware-generated. No artwork/mapping was added. The supplied saves begin
  at different progress stages; this is not an identical new-game test matrix.
- Continued runs reach 150 emulated seconds, with name/setup/menu interactions
  and visible characters. The anniversary image additionally reaches name entry
  and a new character through 400 seconds; English HUNGRY / FRIEND LEVEL menus
  render correctly. Original iD hatching was verified previously; not every
  special edition's complete new-generation/hatch sequence has been verified.
- Normal idle sleep and button wake were observed for every image, including
  reduced 16,384-Hz MCLK followed by 18.432-MHz operation. RTC continues across
  those transitions. Optional keep-awake is separate from natural pet sleep.
- Each production worker ran for about ten real seconds at approximately 100%
  speed (10.012-10.019 wall seconds, 10.037-10.082 emulated seconds; snapshot
  granularity applies). Windows PCM output was active with zero reported drops.
  Existing original-iD 60-second timing evidence remains in [ID.md](ID.md).
- Each private library survived reopening, Slot 1/export/import/Save on Exit and
  a fresh-worker autosave resume. Full states also round-trip and continue
  identically for 10,000 CPU steps, including PCM and flash. Anniversary boot
  actually performs 6,132 program operations and 48 erases in emulated NOR.

The optimized 0.4.0 Windows EXE was launched and closed for every image, then
reopened without arguments from its remembered library. Advancing autosaves and
application-only window captures confirm native color rendering and persistence.
Final checks pass: 197 Rust tests (one existing browser test ignored), 25 Python
tooling tests, every existing Python emulator comparison, the 1,889-case C oracle,
license evidence for 106 packages, formatting/locked check and Windows release build.

These are focused local engineering checks, not a full lifetime playthrough or
subjective loudspeaker certification. IR/connectivity/DLC, upstream .sav/
.tamasave conversion and original 4U remain unsupported. 4U+ was subsequently
validated in [4UPLUS.md](4UPLUS.md). Plus Color/Hexagontchi were subsequently validated in
[PLUS_COLOR.md](PLUS_COLOR.md). Asset and upstream
licensing questions in THIRD_PARTY_LICENSES.md remain separate. No firmware,
private saves, screenshots or audio captures are included in the source tree.
