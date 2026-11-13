<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Original notice text: Copyright (C) 2026 Finn L?lsdorf / Noctyros -->

# Third-party notices

RetroHatch's original Rust ports, integration and additions are licensed under
**GPL-3.0-only**. Upstream authors retain their rights and applicable notices.
The complete project license is in [LICENSE](LICENSE); authorship scope is in
[COPYRIGHT.md](COPYRIGHT.md). This document credits incorporated material, not
firmware supplied privately by users.

## BrickEmuPy

- Developer: **azya52**; [repository](https://github.com/azya52/BrickEmuPy).
- Published license: **CC0-1.0**, retained in
  [the complete upstream text](app/LICENSES/BrickEmuPy-CC0-1.0.txt).
- Use: Rust translations/adaptations of E0C6200 CPU/MCU, SPLB32 CPU/MCU,
  HT24LC08 EEPROM, timing/audio behavior, hardware profiles and technical LCD
  mappings. Classic LCD vectors have derived geometry and alpha masks.
- The independent ownership of some classic icon silhouettes is not established
  by the repository-wide CC0 declaration. Detailed findings are retained in the
  [provenance record](developer/LICENSING_PROVENANCE.md). Connection V3's public
  replacement SVG symbols are original RetroHatch drawings under GPL-3.0-only.

## TG18-Emu

- Developer: **oreo-fox**; [repository](https://github.com/oreo-fox/tg18-emu).
- Published license: **MIT**, Copyright (c) 2026 oreo-fox. Complete unchanged
  notice: [LICENSES/TG18-Emu-MIT.txt](LICENSES/TG18-Emu-MIT.txt) and
  `app/third_party/tg18-core/LICENSE`.
- Use: the ARM CPU, bus, LCD, sound, RTC/peripherals and snapshots for On/Meets.
  MIT integration changes are marked within that library. No frontend artwork
  or firmware is incorporated.

## tamaemu

- Developer: **maragotchi**; [repository](https://github.com/maragotchi/tamaemu),
  source revision `79e19b65a1c970b9e1cd0b58ee6e929507257234`.
- Published license designation: **GNU GPL version 3**; complete retained text:
  [app/LICENSES/TamaEmu-GPL-3.0.txt](app/LICENSES/TamaEmu-GPL-3.0.txt).
- Use: native Rust translation/adaptation of S1C33 CPU, memory/flash, display,
  audio and peripherals. Source hashes and credits, including GMMan's
  [S1C33 SLEIGH instruction reference](https://github.com/GMMan/s1c33_sleigh),
  are preserved in `app/src/core/s1c33/UPSTREAM.json` and source headers.
- The earlier upstream README wording and instruction-reference provenance
  questions remain recorded in the historical provenance record. No resolution
  or additional license restriction is asserted here.

## Tama-Go: tamago, T7Emu and Paramix Studio

- **Bryon Vandiver (asterick)**: original JavaScript emulator,
  [asterick/tamago](https://github.com/asterick/tamago), revision
  `905de5cff0226ce5ae88ac7b8164c3df0cd3d7cb`. Its published `package.json`
  designation is **BSD; exact variant not specified**. A complete original BSD
  grant/copyright notice has not been located. No specific BSD SPDX identifier
  or substitute license text is invented.
- **mameise**: T7Emu C port and modifications, published in
  [Paramix Studio](https://github.com/mameise/Paramix-Studio), revision
  `675b904ab1c8a9086b977a82cbc47cd0c319187f`. Paramix's published notice declares
  **GNU GPL version 3 or later**, retaining predecessor notices. The separate
  [T7Emu repository](https://github.com/mameise/T7Emu) supplied no incorporated
  source. The complete GPLv3 text is in
  [LICENSES/Paramix-Studio-GPL-3.0.txt](LICENSES/Paramix-Studio-GPL-3.0.txt);
  the relevant original notice is retained in
  [LICENSES/Paramix-Studio-T7Emu-NOTICE.txt](LICENSES/Paramix-Studio-T7Emu-NOTICE.txt).
- **RetroHatch / Noctyros**: Rust port, host integration and original additions,
  Copyright (C) 2026 Finn L?lsdorf / Noctyros, **GPL-3.0-only** for those
  contributions. CPU, banked memory, GPIO, EEPROM, timers, DAC and SPI figure
  behavior derive from the above implementations. Exact source hashes and
  scope are in `app/src/core/tamago/UPSTREAM.json`.
- No T7Emu icon artwork is incorporated: electrical fixed-symbol states use
  ordinary host text labels; matrix/figure graphics come from user firmware.
  Paramix credits its own 20?20 icons to mameise; that is distinct from original
  firmware imagery and does not grant rights to Bandai artwork.
- The owner has elected to proceed with the documented BSD-variant uncertainty.
  This is a publication decision, **not** confirmation of the variant or a
  resolution of missing permissions. Historical evidence and intermediate-port
  questions remain in [the Tama-Go audit](developer/TAMAGO_LICENSE_AUDIT.md).

## Cargo dependencies, fonts and runtime

The exact locked Windows graph, license expressions, original copyright notices,
required acknowledgements and complete located license texts are retained in:

- [app/THIRD_PARTY_NOTICES.md](app/THIRD_PARTY_NOTICES.md): generated package list.
- [LICENSES/CARGO_PACKAGES.json](LICENSES/CARGO_PACKAGES.json): revision evidence.
- [LICENSES/CARGO_NOTICES.txt](LICENSES/CARGO_NOTICES.txt): complete collected notices.
- [LICENSES/rust-stdlib](LICENSES/rust-stdlib/): Rust runtime notices.

Embedded Ubuntu, Hack and egui icon fonts retain their separate font notices;
font conditions are not replaced by a library's MIT/Apache alternatives.
Development-only Python/Qt reference tools are not linked into RetroHatch.
No Windows system DLL or separate MSVC redistributable is bundled.

## Research, graphics and redistribution

TamaLIB/TamaTool, MAME and Epson documentation were technical research sources;
no implementations or artwork from those sources are included. The owner-supplied
README logo has its existing provenance record; no new artwork grant is invented.
Reference checkouts, ROMs, figure dumps and user saves are not distributed.

Keep applicable copyright notices, complete license texts and this document with
redistributed binaries. GPL distributions must provide the exact corresponding
source and build materials under the applicable GPL obligations; RetroHatch's
release workflow supplies a matching source archive with vendored dependencies.
See [the release guide](developer/RELEASE.md). Neither GPL nor CC0 licenses
third-party trademarks or user-provided firmware.
