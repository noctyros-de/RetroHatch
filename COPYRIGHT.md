<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Copyright and licensing scope

Copyright (C) 2026 Finn Lülsdorf / Noctyros (the project owner's confirmed attribution)
for original RetroHatch contributions. Other contributors retain copyright in
their contributions. Git authorship metadata is not proof that every file is
exclusively owned by the project owner.

Original project code, tests, tools and newly authored documentation are offered
under SPDX `GPL-3.0-only`. Original-contribution SPDX headers preserve existing
BrickEmuPy derivation comments. The overall program's GPL terms do not erase
third-party rights, CC0 provenance, separate font conditions or mandatory notices.

| Material | Scope |
| --- | --- |
| Handwritten Rust port, integration, host/UI, tests and developer tools | GPL-3.0-only for original RetroHatch contributions, with upstream provenance retained |
| BrickEmuPy CPU/MCU behavior and derived technical mapping | Upstream CC0-1.0 declaration; see the unchanged full license and attribution |
| Generated LCD geometry/spans and alpha masks | Upstream-derived data with unresolved independent icon provenance; no exclusive Noctyros authorship claim or fabricated GPL grant |
| SPLB32/Connection V3 CPU, MCU, sound and HT24LC08 | Direct CC0 BrickEmuPy translation by Noctyros; no exclusive authorship claim over upstream behavior; fixed icons excluded from this integration |
| Connection V3 replacement SVG symbols and coverage | Original RetroHatch functional drawings, Copyright (C) 2026 Noctyros, GPL-3.0-only; editable sources included, no upstream silhouettes copied |
| TG18 ARM library in `app/third_party/tg18-core/` | MIT; Copyright (c) 2026 oreo-fox; separately marked MIT integration additions by Noctyros |
| S1C33 CPU/memory/LCD/peripheral port | Derived from maragotchi/tamaemu (declared GPLv3); GPL-3.0-only Rust translation by Noctyros; unresolved upstream README/provenance issues are documented separately |
| Cargo libraries and embedded fonts | Their own licenses/notices; see the generated Windows inventory and full notices |
| Rust standard-library components | Toolchain attribution and applicable component licenses retained separately |
| ROMs, firmware, private saves and reference checkouts | Excluded from the public source distribution; no rights granted by this project |

The GPL document itself is Copyright (C) 2007 Free Software Foundation, Inc.
That identifies the license document's author, not the author of RetroHatch or
its third-party implementations. The root LICENSE is the complete, unmodified
official GNU version-3 text; the **only** version choice comes from the project's
license declarations, not from editing that text.

Preserve copyright/license notices when contributing. Do not add a project-wide
notice to borrowed files that implies ownership of their underlying material.
Public availability is not proof of third-party rights. Track unresolved
provenance in THIRD_PARTY_LICENSES.md and developer/OPEN_SOURCE_AUDIT.md.
