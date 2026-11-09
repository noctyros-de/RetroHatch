<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros; original report text only -->

# Third-party licensing and provenance

Audit scope: the current RetroHatch working tree, Cargo.lock and default
`x86_64-pc-windows-msvc` dependency graph, inspected on 9 November 2026.
This is distribution preparation, **not legal clearance**. The repository
remains private and the unresolved LCD artwork/source-form issues block public
distribution. License declarations do not establish rights to every asset.

## Incorporated reference material

### BrickEmuPy

- Upstream: [azya52/BrickEmuPy](https://github.com/azya52/BrickEmuPy), maintained
  under that upstream handle; no personal name or copyright year is invented.
- Declared license: **CC0-1.0**. The complete unchanged local upstream text is
  [app/LICENSES/BrickEmuPy-CC0-1.0.txt](app/LICENSES/BrickEmuPy-CC0-1.0.txt).
- Incorporated/adapted material: E0C6200 instruction and MCU/I/O/timer/buzzer
  behavior; technical device-profile values; display-only segment mappings and
  generated geometry/alpha masks; synthetic reference expectations generated
  with the original handlers. The Rust implementation's original contributions
  are distinguished from that upstream material in COPYRIGHT.md/source headers.
- CC0 does not impose an attribution condition like MIT, but this project
  preserves the source declaration and acknowledgement. CC0 does not grant
  trademark rights, and its upstream application cannot resolve third-party
  ownership that the upstream author may not hold.
- The five LCD resource sets cover P1/P2 (shared), Genjintch, Angel, Morino and
  Umino. No shell, decorative background or complete SVG is embedded in the
  executable. The independently owned icon silhouettes have **unresolved
  provenance**. A repository-level CC0 file is insufficient clearance.

Affected material includes `app/assets/lcd/*.alpha`,
`app/src/lcd_geometry.rs`, `*_lcd_geometry.rs` and `lcd_masks.rs`.
Do not describe the symbols as newly authored by Noktyros. Their editable source
vectors remain in the private reference checkout; rights and suitable
Corresponding Source/preferred-source availability must be resolved before
distributing the affected assets or executable. This preparation does not remove,
redraw, relicense or publicly package them.

## Inspiration and research; no incorporated implementation identified

| Project/source | Verified declaration and attribution | RetroHatch use / limits |
| --- | --- | --- |
| [TG18-Emu](https://github.com/oreo-fox/tg18-emu) | Local LICENSE: MIT; Copyright (c) 2026 oreo-fox | Desktop design, menus/settings/save-architecture inspiration. No incorporated TG18 implementation identified in the inspected E0C6200 application. Full MIT notice retained in [LICENSES/TG18-Emu-MIT.txt](LICENSES/TG18-Emu-MIT.txt) as acknowledgement; this does not claim a copied TG18 core. |
| [TamaEmu](https://github.com/maragotchi/tamaemu) | Upstream LICENSE contains GPL version 3; README declares GNU GPL-3.0. Maintainer handle maragotchi; no separate code copyright owner established by the inspected README/LICENSE/header. | S1C33/color-device research context, not a linked or ported RetroHatch implementation. Exact-only versus later-version grant was not established from the inspected code header. Its README's instruction against charging conflicts with the ordinary GPL freedom to charge; clarify terms before any future reuse. No such restriction is imposed on RetroHatch. |
| [TamaLIB](https://github.com/jcrona/tamalib), [TamaTool](https://github.com/jcrona/tamatool) | Upstream GPLv2 texts and Jean-Christophe Rona attribution inspected | Hardware/ROM-format research only; no code, icon set or firmware included. GPL-2.0-only code must not be assumed compatible with GPL-3.0-only; review the exact grant before any future incorporation. |
| [Paramix Studio](https://github.com/mameise/Paramix-Studio) | Project notices declare GPLv3-or-later; include TamaLIB components with their own notices | Factual research labels/checksum corroboration only. No emulator implementation, approximate icon bitmap or firmware copied. No exclusive ownership of that project's contributions is claimed. |
| [MAME](https://github.com/mamedev/mame) | Inspected E0C6200 handheld driver header: BSD-3-Clause; the whole MAME tree has mixed licenses | Hardware/dump research only; no MAME core or display assets incorporated. Do not apply that single driver's license to all of MAME. |
| [Epson E0C6S48 manual](https://download.epson-europe.com/pub/electronics-de/asmic/4bit/62family/technicalmanual/tm_6s48.pdf) | Epson copyright/rights-reserved notice | Technical facts consulted; no manual pages/figures reproduced or license to artwork inferred. |

These rows distinguish inspected material and actual engineering use from
blanket claims about every upstream file or version. Future copied code/assets
need a fresh, version-specific provenance/licensing review and retained notices.
No license from a research-only project is silently imported into this program.

## Rust/Cargo dependencies actually resolved for Windows

There are **100 third-party packages** in the locked default Windows normal/
build graph, plus RetroHatch itself. There are no Git-source dependencies in
that graph. See the complete per-name/version/upstream/license inventory:

- [LICENSES/CARGO_INVENTORY.md](LICENSES/CARGO_INVENTORY.md)
- [LICENSES/CARGO_PACKAGES.json](LICENSES/CARGO_PACKAGES.json): features, declared
  authors (not copyright assignments), notice-section hashes and lockfile hash.
- [LICENSES/CARGO_NOTICES.txt](LICENSES/CARGO_NOTICES.txt): full located upstream
  license texts, nested notices, source copyright/license headers, OpenGL XML
  legal comments and embedded font copyright/license metadata.

The inventory conservatively retains build-time and unused feature-resource
notices rather than claiming every listed item is linked into the executable.
It does not audit every other platform/feature in Cargo.lock. Published package
authors are not substituted for copyright holders where notices say otherwise.

Direct dependencies are eframe 0.37.0 (MIT OR Apache-2.0) and windows-sys 0.61.2
(MIT OR Apache-2.0). Their complete resolved GUI/windowing/font/image/Windows
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
  upstream **release tag 0.37.0**; [EGUI_SOURCE.json](LICENSES/EGUI_SOURCE.json)
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

[LICENSES/rust-stdlib](LICENSES/rust-stdlib/) retains the installed Rust 1.99.1
standard-library attribution HTML and its complete supplied license set,
including applicable runtime exceptions. These compiler-produced runtime
components are not owned or relicensed by Noktyros. Re-audit when changing the
toolchain. Rust compiler tools themselves are not bundled with RetroHatch.
No Windows system DLL or separately licensed MSVC redistributable is copied
from this workstation into a release package. Review the actual runtime/linking
and System Library treatment before conveying a future package.

## Development-only dependencies and distribution gates

Python/PyQt6/Qt are used by optional local comparison/SVG-generation tools, not
linked into the Rust executable. PyQt6 has GPL/commercial terms; consult the
version's own license before redistributing those development dependencies.
Using a generator does not cure the underlying icon ownership gap. The private
reference repositories and firmware are not part of the intended public source.

The GPL grant for original project work does not authorize publication of
private material or unresolved third-party assets. Before any source/binary
release, resolve symbol rights and preferred source availability, review font
embedding/notice coverage and exact dependency/toolchain scope, inspect the
intended refs, and provide complete matching source/build information as required
by GPLv3. See [developer/RELEASE.md](developer/RELEASE.md).
