# Historical licensing and provenance record

This is the retained pre-0.10.0 assessment. Statements about release gates and
Paramix being research-only describe that historical scope, not current incorporation.
For current component credits see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).
The owner authorized proceeding on 13 November 2026 with the recorded BSD-variant
uncertainty; this decision does not identify a variant or resolve permissions.

<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros; original report text only -->

# Third-party licensing and provenance

Reviewed on 11 November 2026 against the current source, local upstream copies,
public upstream declarations and locked default Windows dependency graph.
RetroHatch is public and its original contributions use **GPL-3.0-only**.
Third-party notices remain applicable. Public availability does not by itself
resolve the specific evidence gaps below; this review is not legal clearance.
See [current audit findings](OPEN_SOURCE_AUDIT.md).

## Incorporated reference material

### BrickEmuPy

- Upstream: [azya52/BrickEmuPy](https://github.com/azya52/BrickEmuPy), maintained
  under that upstream handle; no personal name or copyright year is invented.
- Declared license: **CC0-1.0**. The complete unchanged local upstream text is
  [app/LICENSES/BrickEmuPy-CC0-1.0.txt](../app/LICENSES/BrickEmuPy-CC0-1.0.txt).
- Incorporated/adapted material: E0C6200 instruction and MCU/I/O/timer/buzzer
  behavior; technical device-profile values; display-only segment mappings and
  generated geometry/alpha masks; synthetic reference expectations generated
  with the original handlers. The Rust implementation's original contributions
  are distinguished from that upstream material in COPYRIGHT.md/source headers.
- Connection V3 additionally translates SPLB32 CPU/MCU/timer/audio behavior and
  HT24LC08 I2C EEPROM directly from the same CC0 source. Exact source hashes and
  scope are in `app/src/core/splb32/UPSTREAM.json`. Public V3 LCD data includes technical matrix wiring and ten original GPL-3.0-only
  RetroHatch replacement SVGs/coverage. Original upstream fixed-symbol coverage
  remains optional private user data, never incorporated artwork. Paramix Studio was researched,
  but no Paramix C/custom icon or Tama-Go/T7 implementation was copied into this
  integration. See [Connection V3 scope](CONNECTION_V3.md).
- CC0 does not impose an attribution condition like MIT, but this project
  preserves the source declaration and acknowledgement. CC0 does not grant
  trademark rights, and its upstream application cannot resolve third-party
  ownership that the upstream author may not hold.
- The five LCD resource sets cover P1/P2 (shared), Genjintch, Angel, Morino and
  Umino. Yasashii additionally uses the original 512 matrix coordinates and
  electrical segment mapping from `TamagotchiYasashii.svg`; its six GPIO LEDs
  are represented by ordinary host UI circles, without copied LED/shell art.
  No shell, decorative background or complete SVG is embedded in the
  executable. The original icon silhouettes have **unresolved independent
  provenance**. A repository-level CC0 file does not verify all third-party rights.

Affected material includes `app/assets/lcd/*.alpha`,
`app/src/lcd_geometry.rs`, `*_lcd_geometry.rs` and `lcd_masks.rs`.
Do not describe the symbols as newly authored by Noctyros. Their editable source
vectors remain in the excluded reference checkout, rather than the public
source tree. Record the original icon rights and provide the preferred editable
inputs where required; derived spans/masks are not equivalent provenance proof.
This audit does not remove, redraw or relicense these assets.

## TG18-Emu implementation incorporated under MIT

The On/Meets backend (including six dedicated Japanese Meets editions) incorporates the Rust library from
[oreo-fox/tg18-emu](https://github.com/oreo-fox/tg18-emu), Copyright (c) 2026
oreo-fox, under MIT. Its ARM926 CPU, bus/peripherals, LCD, timers/RTC,
signature-based firmware hooks, sound and snapshot/flash logic are preserved
in `app/third_party/tg18-core/`. The complete unchanged MIT text is in that
directory's `LICENSE` and [LICENSES/TG18-Emu-MIT.txt](../LICENSES/TG18-Emu-MIT.txt).
Both must accompany redistributed substantial copies; the binary packager
includes the notice. MIT is compatible with the enclosing GPL-3.0-only program.

`UPSTREAM.json` records SHA-256 values of the local original source files before
changes. Integration changes add bounded snapshot decompression, peripheral
and synthesizer state continuity, and flash-delta snapshots. These additions to
the copied library are identified as MIT; the RetroHatch host adapter is
GPL-3.0-only. No TG18 frontend, logo, shell graphics, screenshots or firmware
are copied. Display artwork for this backend comes from the user's firmware
while it runs. The separate classic-LCD provenance blockers remain unchanged.

## tamaemu S1C33 translation

The native Rust implementation under `app/src/core/s1c33/` translates CPU, memory/
NOR, LCD-controller, shared peripheral and disconnected PN512 register/UART behavior from [maragotchi/tamaemu](https://github.com/maragotchi/tamaemu),
revision `79e19b65a1c970b9e1cd0b58ee6e929507257234`. Credit maragotchi as the
original author. The upstream README declares GNU GPL-3.0; no separate original
code copyright year or later-version grant is invented. The unchanged upstream
license text is retained in [app/LICENSES/TamaEmu-GPL-3.0.txt](../app/LICENSES/TamaEmu-GPL-3.0.txt).
Rust translation/integration contributions are Copyright (C) 2026 Noctyros,
GPL-3.0-only. `UPSTREAM.json` records local source hashes and scope. No tamaemu
frontend, icons, firmware, user saves, network service or DLC assets are included.

**Unresolved upstream wording:** the [README Licence section](https://github.com/maragotchi/tamaemu#licence)
also says not to charge for the software and attributes that instruction to GPL.
The actual [GPLv3 text, section 4](https://www.gnu.org/licenses/gpl-3.0.html#section4)
permits conveying copies for a price. No no-charge clause exists in the retained
GPL text, and RetroHatch does not adopt such an extra restriction. The observed
contradiction is between upstream prose and its GPL declaration, not a finding
that GPLv3 itself is incompatible with RetroHatch. Upstream intent/any claimed
additional condition still needs clarification; no permission or resolution is
invented, and this is not a categorical claim that all publication is forbidden.

The original CPU header credits [GMMan/s1c33_sleigh](https://github.com/GMMan/s1c33_sleigh)
as its instruction-set reference. That credit is retained. No SLEIGH source is
copied directly into RetroHatch and no permissive license for that separate
repository was established by this inspection. Review upstream derivation and
any required permission if it goes beyond technical instruction-set facts.

The optional C oracle compiles unchanged locally supplied reference sources in
`app/target/`; it is a development test, never a production emulator executable.
The product core is native Rust and has no C runtime bridge or new dependencies.

## Inspiration and research; no other incorporated implementation identified

| Project/source | Verified declaration and attribution | RetroHatch use / limits |
| --- | --- | --- |
| [TamaLIB](https://github.com/jcrona/tamalib), [TamaTool](https://github.com/jcrona/tamatool) | Upstream GPLv2 texts and Jean-Christophe Rona attribution inspected | Hardware/ROM-format research only; no code, icon set or firmware included. GPL-2.0-only code must not be assumed compatible with GPL-3.0-only; review the exact grant before any future incorporation. |
| [Paramix Studio](https://github.com/mameise/Paramix-Studio) | Project notices declare GPLv3-or-later; include TamaLIB components with their own notices | Factual research labels/checksum corroboration only. No emulator implementation, approximate icon bitmap or firmware copied. No exclusive ownership of that project's contributions is claimed. |
| [MAME](https://github.com/mamedev/mame) | Inspected E0C6200 handheld driver header: BSD-3-Clause; the whole MAME tree has mixed licenses | Hardware/dump research only; no MAME core or display assets incorporated. Do not apply that single driver's license to all of MAME. |
| [Epson E0C6S48 manual](https://download.epson-europe.com/pub/electronics-de/asmic/4bit/62family/technicalmanual/tm_6s48.pdf) | Epson copyright/rights-reserved notice | Technical facts consulted; no manual pages/figures reproduced or license to artwork inferred. |

These rows distinguish inspected material and actual engineering use from
blanket claims about every upstream file or version. Future copied code/assets
need a fresh, version-specific provenance/licensing review and retained notices.
No license from a research-only project is silently imported into this program.

## Branding and promotional media

The owner supplied the RetroHatch branding. The public logo is
`docs/assets/retrohatchlogo.png`; the optional local ICO is excluded from Git
but can be embedded in local Windows builds. No separate artist or rights-chain
document was supplied in this audit; record its intended asset license and any
third-party elements without inventing an author or blanket artwork clearance.
`docs/screenshots/` is reserved for owner-selected promotional images. Generated
Pictures captures remain local. A screenshot can contain firmware artwork and
is not automatically licensed by GPL or CC0 merely because RetroHatch made it.

## Rust/Cargo dependencies actually resolved for Windows

There are **106 third-party packages** in the locked default Windows normal/
build graph, plus RetroHatch itself. There are no Git-source dependencies in
that graph. See the complete per-name/version/upstream/license inventory:

- [LICENSES/CARGO_INVENTORY.md](../LICENSES/CARGO_INVENTORY.md)
- [LICENSES/CARGO_PACKAGES.json](../LICENSES/CARGO_PACKAGES.json): features, declared
  authors (not copyright assignments), notice-section hashes and lockfile hash.
- [LICENSES/CARGO_NOTICES.txt](../LICENSES/CARGO_NOTICES.txt): full located upstream
  license texts, nested notices, source copyright/license headers, OpenGL XML
  legal comments and embedded font copyright/license metadata.

The inventory conservatively retains build-time and unused feature-resource
notices rather than claiming every listed item is linked into the executable.
It does not audit every other platform/feature in Cargo.lock. Published package
authors are not substituted for copyright holders where notices say otherwise.

Direct dependencies are eframe 0.37.0 (MIT OR Apache-2.0), windows-sys 0.61.2
(MIT OR Apache-2.0), and the local MIT tg18-core library described above. Their complete resolved GUI/windowing/font/image/Windows
dependency set is enumerated, including multiple versions where applicable.
Apache-2.0 contributions can be included in GPLv3 works while retaining their
own conditions; see the [Apache compatibility explanation](https://apache.org/licenses/GPL-compatibility.html).
No mandatory GPL-2.0-only component was identified in this Windows Cargo graph.
This does not eliminate font/data or upstream-provenance review.

### License choices and nontrivial obligations

- `self_cell` 1.3.0 offers Apache-2.0 OR GPL-2.0-only. Use **Apache-2.0** for this
  distribution; the incompatible GPL-2.0-only alternative is not selected.
- MIT/BSD/ISC/Zlib components retain their copyright, permission and disclaimer
  notices. Apache components retain their license and applicable NOTICE/source
  attributions. Keeping notices does not relicense those components exclusively
  as RetroHatch code. Legacy `MIT/Apache-2.0` expressions are retained verbatim;
  their actual MIT and Apache texts are included.
- `dpi` 0.1.2 declares Apache-2.0 AND MIT. Its source distinguishes Apache code
  from the MIT libm-derived no_std path. Both full notices/copyright headers are
  retained even though the current graph's std path does not need that module.
- `cfg_aliases` includes a nested tectonic notice. `tracing-core` includes a
  separate spin notice. Unicode-data conditions in `unicode-ident` and
  `unicode-properties` are **AND** requirements, not alternatives to MIT/Apache.
  Their complete located notices are included.
- `gl_generator`/`khronos_api` omit standalone license files but declare Apache-2.0
  and contain actual source/registry copyright notices. Those notices and the
  standard full Apache text are preserved. Additional registry legal comments
  are retained rather than assuming the root metadata covers every subresource.
- Several egui workspace packages omit their standalone license texts from the
  published crate. The complete workspace MIT/Apache texts were obtained from
  upstream **release tag 0.37.0**; [EGUI_SOURCE.json](../LICENSES/EGUI_SOURCE.json)
  records that evidence. The cached VCS-revision URL was unavailable; the
  versioned release-tag texts are not fabricated author attributions.
- `profiling` omits standalone texts in the cached package. Its full MIT/Apache
  notices were retrieved at its recorded upstream source revision
  `8271551172eb6fa4cba47369aedd93790c623df9` and are preserved unchanged.

### Embedded fonts: keep their separate licenses

`epaint_default_fonts` 0.37.0 declares `(MIT OR Apache-2.0) AND OFL-1.1 AND
Ubuntu-font-1.0`. Actual font resources/notices were inspected, including their
embedded copyright metadata. The current feature graph does not enable the
optional full monochrome emoji fonts, but retains all supplied notices.

| Resource | Actual attribution / terms |
| --- | --- |
| Ubuntu-Light.ttf | Embedded notice: Copyright 2011 Canonical Ltd.; Ubuntu Font Licence 1.0 |
| Hack-Regular.ttf | Source Foundry Authors (2018), Bitstream Inc. (2003), and public-domain DejaVu work as stated by the supplied full font notice; MIT plus Bitstream Vera conditions/reserved names |
| egui-icons / emoji-icon-font | Supplied MIT notice: Copyright (c) 2014 John Slegers; platform/logo glyphs do not grant trademark rights |
| NotoEmoji-Regular.ttf (optional) | Embedded notice: Copyright 2013 Google Inc.; supplied OFL-1.1 text |

Do not label these fonts GPL-3.0-only, remove notices or assume the package's
MIT option replaces the **AND** font licenses. OFL/UFL embedding, reserved-name,
modified-font and redistribution requirements require explicit review for the
actual release package. The supplied Hack notice additionally contains Bitstream
Vera terms not fully described by the crate's short SPDX expression. Relevant
primary guidance: [Ubuntu Font Licence](https://ubuntu.com/legal/font-licence)
and [embedding FAQ](https://ubuntu.com/legal/font-licence/faq). This preparation
preserves notices; it does not make a blanket legal compatibility determination
for every possible font distribution/modification scenario.

### Rust runtime and native system components

[LICENSES/rust-stdlib](../LICENSES/rust-stdlib/) retains the installed Rust 1.99.1
standard-library attribution HTML and its complete supplied license set,
including applicable runtime exceptions. These compiler-produced runtime
components are not owned or relicensed by Noctyros. Re-audit when changing the
toolchain. Rust compiler tools themselves are not bundled with RetroHatch.
No Windows system DLL or separately licensed MSVC redistributable is copied
from this workstation into a release package. Review the actual runtime/linking
and System Library treatment before conveying a future package.

## Development-only dependencies and distribution gates

Python/PyQt6/Qt are used by optional local comparison/SVG-generation tools, not
linked into the Rust executable. PyQt6 has GPL/commercial terms; consult the
version's own license before redistributing those development dependencies.
Using a generator does not cure the underlying icon ownership gap. The local
reference repositories and firmware are excluded from the public source.

Keep private user data and copyrighted firmware out of contributions. For
redistribution, retain applicable notices and provide exact matching source and
build information under GPLv3; track the concrete icon/vector, tamaemu wording,
SLEIGH and font obligations above. See [developer/RELEASE.md](RELEASE.md).

## Experimental Tama-Go backend: Paramix-Studio T7Emu and asterick/tamago

The native Rust code in `app/src/core/tamago/` ports/adapts the 6502 interpreter,
banked memory, GPIO, EEPROM, timer/DAC and serial figure behavior of
[mameise/Paramix-Studio](https://github.com/mameise/Paramix-Studio), `t7/`, revision
`675b904ab1c8a9086b977a82cbc47cd0c319187f`. Credit **mameise** for that emulator.
The complete root GPL version 3 text is preserved in
[LICENSES/Paramix-Studio-GPL-3.0.txt](../LICENSES/Paramix-Studio-GPL-3.0.txt).
`app/src/core/tamago/UPSTREAM.json` records the actual source hashes and scope.
The opt-in `tamago_reference_host.c` test adapter is adapted from its GPL host glue.
Original RetroHatch Rust/integration contributions are Copyright (C) 2026
Noctyros, GPL-3.0-only; this does not claim authorship of the upstream algorithms.

Paramix explicitly credits [asterick/tamago](https://github.com/asterick/tamago),
revision `905de5cff0226ce5ae88ac7b8164c3df0cd3d7cb`, as the original JavaScript
implementation. Its `package.json` says `BSD`, but the inspected tree supplies
no complete BSD grant, clause count or reliable copyright-holder notice.
**Public-distribution blocker:** obtain/verify that original notice and the
license chain for the translated portions. Do not invent a BSD text or treat
Paramix's attribution as proof of its predecessor's precise terms.
[mameise/T7Emu](https://github.com/mameise/T7Emu) currently supplies only a short
README pointing elsewhere; no implementation was copied from that repository.

No downloaded firmware, figure contents, fixed LCD artwork, Font Awesome assets,
Paradise UI, program binaries, connection bridge or dynamic compiler are bundled.
Tama-Go's fixed icon states are currently presented as text labels because exact
original icon geometry was not established. Game graphics are read from private,
user-supplied main/figure firmware. See [Tama-Go status](TAMAGO.md).
