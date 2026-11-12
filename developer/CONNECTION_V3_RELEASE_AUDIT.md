<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Connection V3 publication gate

Status: **V3-SPECIFIC LICENSE REVIEW PASSED; v0.9.0 publication authorized subject
to final validation**. The owner explicitly distinguished pre-existing documented
provenance questions from newly introduced Connection V3 risks. They remain open
and are not described as legally cleared. No concrete new V3 licensing violation
or material conflict was identified. The prior attempt held version 0.8.0 under
a broader gate; this preparation advances to 0.9.0 under the revised instructions.
The isolated Tama-Go branch is unchanged and excluded.

## V3 symbols: origin, evidence and selected solution

The optional private `connection-v3.rhlcd` contains exact coverage of ten paths
from BrickEmuPy `assets/Tamagotchi_v3.svg`. Its root CC0 declaration does not
independently establish ownership of Bandai-style symbol silhouettes. That pack
and those paths are not copied into this public integration.

Paramix Studio was examined at revision
`675b904ab1c8a9086b977a82cbc47cd0c319187f`:

- `v3/src/app/icons_v3.h` explicitly says its 20x20 icons were drawn by mameise.
- `v3/gen/icons.py` identifies PNG inputs `icon_0.png` through `icon_9.png`;
  missing inputs fall back to `t7/src/app/icons.h`.
- The V3 file history at that revision contains introduction commit
  `7f8932bad8635cbefa6295e5fb62a0c9089519b1` (mameise, 24 October 2026,
  "Paramix Studio 1.6.9"). No separate explicit copyright year appears in the
  bitmap header; commit authorship is evidence, not proof of every underlying right.
- V3 indices 1/2/6/8/9 are identical to the same T7 indices; V3 index 4 equals
  T7 index 7. This proves **six** identical rows, not which missing-file fallback
  was taken. The T7 header separately calls these mameise's ESP32 drawings.
- Root `NOTICE.md` grants GPL version 3 or later; its V3 section credits the
  CC0 core/mapping and attributes the C port, host and icons to mameise. GPLv3
  terms are compatible in principle with choosing version 3 for RetroHatch.
  No BSD-core code is incorporated merely by researching those image arrays.
- The pinned complete, non-truncated tree contains the generator and generated
  header, but none of the named original PNG inputs. Whether the header itself
  is the preferred editable form for those drawings is unconfirmed. Public
  headers, an author comment and a compatible license alone do not establish
  source completeness or all underlying artwork rights.

**Decision:** do not ship Paramix/T7 image rows, generated coverage, private
BrickEmuPy silhouettes or reconstructed PNGs presented as original sources.
No assertion is made that Paramix's own artwork is unlicensed or GPL-incompatible.
Instead, ten original generic functional drawings were created for RetroHatch.
Their preferred editable sources are the ten SVG files under
`app/assets/connection-v3/`, licensed **GPL-3.0-only** with original project
contributions attributed to Noctyros. The generator and generated `icons.alpha`
are included. These are visibly different replacement designs, not an assertion
of authentic original Bandai LCD icon shapes. Matrix wiring and all firmware
segment activation remain unchanged; both rows contain five corresponding symbols.
No private resource or reference checkout is needed by Cargo.

Pinned research evidence SHA-256 (research files themselves stay private):

- `icons.py`: `75e1abdbcbe141b2b69c3dbbeed7aed4a4767b63f8c7da2e27eedccae923970c`
- `icons_v3.h`: `ff1c7bcb223b4c04c2328c574aa0f356c60237308c929f612c3dca9f68b394f3`
- `t7-icons.h`: `9812bdcc01d108f0aaadd5bc418dc2a98bd6dba0d86bdd720d4ddc7a88deee4b`
- `LICENSE`: `8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903`
- `NOTICE.md`: `ad4947568f5655d4424c502b87924b2c022751220d0deadf60e450a96d3a959d`

## SPLB32 backend

The backend is a direct Rust translation of azya52/BrickEmuPy SPLB32,
SPLB32sound and HT24LC08. `app/src/core/splb32/UPSTREAM.json` records exact
source hashes. The local upstream complete CC0 license is byte-identical to
`app/LICENSES/BrickEmuPy-CC0-1.0.txt`. Preserve that text and acknowledgement;
original Rust integration contributions use GPL-3.0-only. No Paramix C/core,
T7/asterick code, firmware or figure data was incorporated. The timebase byte-read
correction is an independently implemented hardware-width mask corroborated by
Paramix commentary; it does not import its C implementation. No new dependency
or additional third-party license is introduced by the original replacement art.

## Pre-existing provenance questions tracked separately

The V3 replacement removes **its** private-art dependency. It does not resolve
pre-existing components that the proposed Windows executable still contains:

1. `app/assets/lcd/*.alpha`, `app/src/lcd_masks.rs`, `app/src/lcd_geometry.rs`,
   and Genjintch/Angel/Morino/Umino geometry derive fixed silhouettes from the
   BrickEmuPy SVGs. Independent artwork provenance and preferred editable
   sources remain unverified. Obtain an adequate rights/source statement or
   replace that older fixed artwork with cleared, editable alternatives.
2. `app/src/core/s1c33/` derives from maragotchi/tamaemu. The pinned README's
   Licence section still says not to charge, while its complete GPL text
   permits charging. This is an unresolved upstream wording/intent ambiguity,
   not a finding that GPLv3 forbids combining the cores. Clarification is still
   tracked separately; the owner did not declare the ambiguity resolved.
3. The S1C33 upstream header also credits GMMan/s1c33_sleigh. Whether protectable
   implementation beyond instruction-set facts was reused, and any necessary
   permission, remain unestablished. This is not a conclusion of infringement.

These exact questions were already recorded in `developer/OPEN_SOURCE_AUDIT.md`
and `THIRD_PARTY_LICENSES.md`. This task does not silently clear them, remove
working device families, or broaden into replacing their artwork/cores. A clean
build, complete Cargo inventory or a previous public release is not rights proof.
Under the current owner instructions, these previously documented questions do
not automatically block the new V3 release. A newly identified concrete violation
or material conflict would still stop publication.

## Publication procedure

Preserve the original V3 SVG replacements and generator in the matching source
archive. Synchronize version 0.9.0, Cargo.lock and dependency inventory; run final
regression, license, privacy and Windows checks. Exclude the unrelated
`developer/PROJECT_STATUS.md` draft and all private directories. Integrate only
verified V3 changes into main. The existing main workflow creates the normal
release/tag; never manually create or move a version tag. This document records
the scoped review and owner's release decision, not whole-project legal clearance.

## Technical checks completed for this preparation

- Full Rust suite: **188 library + 38 UI tests passed**, one existing ignored
  test. Formatting and locked compilation passed.
- All ten public replacement glyphs were tested for unique firmware addresses,
  canvas bounds, nonempty coverage, and activation restricted to their own tiles.
- Original SVG-to-coverage regeneration check passed. The actual local ROM passes
  the negative/positive setup, username/EEPROM and hatching regression with no
  private icon pack; full-state continuation is checked every five emulated seconds.
- License inventory: **106 locked Windows packages verified**. Audit-tool tests
  (2), packaging tests (10) and automatic-release tests (8) passed. These checks
  validate inventory/packaging behavior, not the outstanding artwork/intent questions.
- A **266-file public-source candidate** was copied from tracked and intended
  untracked files, excluding the unrelated status draft, all private directories,
  firmware, save files, private icon packs and binaries. No Git index was changed.
  It built successfully with `cargo build --release --locked --offline`, with
  private reference/resource paths absent and branding disabled. Its isolated
  public-symbol test also passed. An initial test-link failure was a transient
  Windows file lock from reusing the target directory while the original test EXE
  was still running; a sequential retry passed without source/test changes.
- The resulting clean-source Windows EXE was launched twice with new private
  runtime data and no private LCD assets. Actual gameplay/health display persisted
  at 216.000006 -> 218.055355 -> 220.077374 seconds; the separately written EEPROM
  exactly matched the firmware-generated image. Original firmware hash is unchanged.
- The optimized static-runtime Windows build was also run with private branding
  disabled. The earlier attempt did not publish or update main under its previous gate.
  Current publication is authorized only after the final 0.9.0 checks.

The original SVGs, renderer and documentation form a reviewable local V3 change;
they are not a statement that the complete application has passed legal clearance.

## Final 0.9.0 pre-commit validation

The scoped V3 review identified no new unresolved third-party component or concrete
licensing conflict. Prior questions remain in the separate audits. Version 0.9.0
and Cargo.lock are synchronized; the regenerated inventory verifies 106 packages.
The full Rust suite passed 226 tests with one existing ignored test, plus the
Python CPU/MCU/device/audio comparisons, SVG regeneration, audit tests (2), updated
packaging tests (11) and automatic-release tests (8). The real private ROM again
passes the zero-hour control and successful setup/hatching/continuation regression
without private icons. The optimized, unbranded static-runtime Windows EXE also
resumed gameplay and separate EEPROM across two fresh native launches.

The publication candidate contains 55 reviewed changed/new project files. No
firmware, saves, local test data, reference checkout, secrets or personal paths
were selected; the existing synthetic privacy-test path is intentional. The
unrelated PROJECT_STATUS draft is excluded. Required V3 SVG/coverage files are
explicitly allowlisted and required by the source packager. Matching ZIP/source
packaging and the independent offline build are verified before publication by
the local clean-candidate check and the unchanged automatic release workflow.
