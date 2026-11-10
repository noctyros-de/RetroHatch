<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Public-project licensing and presentation audit

Date: 11 November 2026. Scope: current local source/index, selected upstream
licenses, current public GitHub metadata, packaging rules and documentation.
Pre-existing uncommitted capture work is preserved. This is an evidence review,
not a legal-clearance certificate or a new emulator/gameplay validation run.

## Public status and actual release

The unauthenticated [GitHub API](https://api.github.com/repos/noctyros-de/RetroHatch)
reports `private: false`, full name noctyros-de/RetroHatch and default branch
main. The public [release list](https://github.com/noctyros-de/RetroHatch/releases)
has one published non-prerelease, tag `RetroHatch`, published 10 November 2026.
Its only uploaded asset is `RetroHatch.exe`; its tagged Cargo version is 0.1.0
and release body is empty. No uploaded Windows ZIP, separate notices/source
bundle, hashes or BUILD_INFO is present. GitHub's automatic tag source archive
is available; the exact binary/source correspondence and preferred source
completeness have not been established. Do not infer a violation or clearance
solely from an asset list, but the owner should review this existing release's
notice delivery and exact Corresponding Source evidence.

The local source is 0.2.0 and includes uncommitted capture work. This audit does
not bump it, rebuild/publish a binary, change remotes or change visibility.
The origin URL still names the former account; only project links/metadata
are corrected to the verified public address. The remote README was inspected:
it has older capture documentation and no selected promotional gallery. Local
capture documentation and all existing content are retained; no remote file is
overwritten, fetched into the worktree or merged automatically.

## Licensing and incorporated material

The root LICENSE matches the current official GNU GPLv3 text (normalized
line endings), verified against https://www.gnu.org/licenses/gpl-3.0.txt.
Original contributions remain GPL-3.0-only, Finn Lülsdorf / Noctyros.
Original upstream notices and complete license files are not changed.

| Component | Evidence and actual reuse | Review result |
| --- | --- | --- |
| BrickEmuPy / azya52 | Upstream [CC0](https://github.com/azya52/BrickEmuPy/blob/main/LICENSE); identical retained local license. E0C6200 CPU/MCU/timers/GPIO/buzzer semantics ported to Rust; device values, segment mapping and LCD geometry/masks adapted. | CC0 declaration is documented; it does not prove rights in every underlying icon or trademark. |
| TG18-Emu / oreo-fox | [MIT](https://github.com/oreo-fox/tg18-emu/blob/main/LICENSE), Copyright (c) 2026 oreo-fox. Eight library Rust files match local upstream after newline normalization; snapshot.rs and sound.rs contain separately attributed MIT integration changes. Cargo metadata differs; UPSTREAM.json records origin/scope. | MIT notices are retained in the library and LICENSES/TG18-Emu-MIT.txt and are included by packaging. Host/UI integration remains GPL-3.0-only. No frontend, ROM or screenshots copied. |
| tamaemu / maragotchi | [GPLv3 LICENSE](https://github.com/maragotchi/tamaemu/blob/master/LICENSE) retained unchanged; native S1C33 CPU/memory/NOR/LCD/peripheral port, revision recorded in app/src/core/s1c33/UPSTREAM.json. | GPLv3 can be combined with the project's GPLv3 code; the README contradiction below is still unresolved. No or-later grant or invented original author/year is asserted. |
| Cargo/runtime/fonts | 106 locked Windows normal/build dependency packages; complete inventory/notices and Rust standard-library notices retained. | No mandatory GPL-2.0-only choice identified. self_cell uses Apache-2.0; compound Unicode/font terms remain applicable. This is not an all-target/all-feature compatibility guarantee. |
| Branding/promotional images | Owner-provided public logo; optional local ICO; no new gallery image selected. | Record the asset license/creation permissions and any third-party content. No artist or exclusive ownership is invented. |
| Other research | TamaLIB/TamaTool, MAME, Epson and Paramix references as distinguished in THIRD_PARTY_LICENSES.md. | No additional implementation/artwork from those research-only sources identified; their licenses are not applied to RetroHatch as a blanket grant. |

### Concrete open rights/source questions

1. **Classic LCD icons and preferred inputs.** app/assets/lcd/*.alpha,
   app/src/lcd_masks.rs, lcd_geometry.rs and the four *_lcd_geometry.rs files
   derive from BrickEmuPy Tamagotchi SVG display groups. No shell is embedded.
   User firmware determines segment activation and dynamic matrix graphics;
   the icon contours and fixed segment positions are distributed independently
   of ROMs. The local SVGs have no separate per-asset rights-chain statement.
   CC0 covers rights held by its affirmer, not otherwise held third-party rights.
   Original editable vectors remain in excluded references/. Establish icon
   provenance and the preferred editable source required for the release.
2. **tamaemu no-charge prose.** Its current [README Licence section](https://github.com/maragotchi/tamaemu#licence)
   says not to charge and presents this as a GPL requirement. GPLv3 section 4
   explicitly permits conveying copies for a price. No such ban exists in the
   retained GPL text and none is adopted by RetroHatch. This is a concrete
   ambiguity in upstream prose/intent, not a conflict inherent in combining
   GPLv3 programs. No upstream clarification or separate permission was found;
   obtain clarification rather than silently deleting or enforcing that prose.
3. **SLEIGH derivation.** tamaemu/src/cpu.c credits GMMan/s1c33_sleigh and names
   a helper source. No SLEIGH file is directly copied here; its separate rights
   have not been established (GitHub license endpoint returned 404). Determine
   whether upstream reused protectable implementation beyond ISA facts and
   retain any required grant/notice. No conclusion of infringement is made.
4. **Asset/font and binary delivery.** Preserve the compound font notices
   (including OFL/UFL/Bitstream conditions) and record owner branding rights.
   Review preferred-source completeness and exact EXE/source/notice delivery
   for the existing single-EXE release. A public repository is not proof that
   all these requirements have been met.

These are specific unresolved questions. This audit neither categorically
prohibits publication of the whole project nor certifies it legally cleared.
Full details and retained terms are in THIRD_PARTY_LICENSES.md; duplicate long
legal explanations are avoided in the README.

## Supported models and evidence

DeviceProfile::ALL exposes eight profiles: P1, P2, Genjintch, Angel, Morino,
Umino, On/Meets and P's. On dispatches to the MIT TG18 ARM backend, with actual
LCD, input, RTC/timing, audio and save adapters; P's uses the native S1C33 port.
Existing integration tests and prior documented P1/P2/sensor/P's firmware tests
are retained. On/Meets is added to Supported Devices based on actual integration
and owner-reported working use. No new boot or all-variant gameplay test was
run in this audit. Help and the TG18 guide use the same qualified status.
TamaOtch remains unsupported; other S1C33 board configurations, SOME and
unimplemented connectivity/DLC are not promoted to supported devices.

## Index, local privacy and gallery

The starting index has 203 files; adding the gallery guide brings it to 204.
No tracked ROM/firmware dump, save/library data, reference/resource checkout,
credential or build-artifact path was found. Bounded current-index and working
text scans found no tested private-key/token signature or personal build-path
prefix. These checks do not rule out every unknown/encoded secret. The public
logo is byte-unchanged and has no text/EXIF metadata chunks carrying such data.
Author/developer names are intentional project attribution, not a secret leak.

One local Codex tool ref still contains excluded docs/ and AGENTS.md trees,
as previously noted by the private historical audit. No branch-history rewrite
or remote-history audit is performed here. Do not mirror every local ref or
publish .git; do not infer that the tool ref was uploaded. Existing local files,
Git history/index contents and remotes are not automatically removed or rewritten.
Only the explicitly requested gallery guide is staged for tracking.

The README Screenshots and Gameplay Videos sections have stable USER-MANAGED
markers. docs/screenshots/README.md explains manual additions. No generated
capture, user screenshot or video is selected or copied. Narrow .gitignore and
packaging exceptions admit only the guide and top-level selected raster images;
private docs, resources, nested captures, ROMs and MP4s remain excluded. Local
AGENTS.md permanently protects entries, order, captions, files and remote/local
differences. The public contributor guide repeats that owner-managed boundary.

## Distribution and checks

The current packager retains GPL, copyright/third-party/font/runtime notices,
embedded runtime assets, public guides and exact build/source information. It
creates a matching vendored source ZIP and hashes only when explicitly invoked;
it refuses overwriting output. Its public-gallery allowlist now includes the
approved guide/images in source and Windows packages so README paths stay valid.
The tag workflow still requires the Cargo version, validates an offline source
build and uploads the corresponding source. It is not invoked by this audit.

Focused validation: ten Python packaging/privacy tests; the 106-package license
inventory check; five Help/metadata tests; cargo fmt --check; local Markdown/image
link and Git-ignore checks. No expensive full suite/release rebuild or firmware
test is needed for these documentation, URL and presentation changes. Application
version remains 0.2.0. Existing emulator/capture work, ROMs, saves and media are
preserved. No commit, push, tag, release, visibility/account/remote change occurs.

## Files updated by this audit

README.md, CONTRIBUTING.md, COPYRIGHT.md, THIRD_PARTY_LICENSES.md,
app/THIRD_PARTY_NOTICES.md, .gitignore, app/Cargo.toml (repository URL only),
app/src/host/platform.rs and app/src/ui/help.rs (link/status text), its Help
tests, app/tools/package_release.py and test_package_release.py, and the
architecture, TG18, branding and release guides under developer/.
This report and docs/screenshots/README.md are new. Local-only AGENTS.md gains
the permanent gallery policy; docs/OPEN_SOURCE_AUDIT.md gains a historical
scope marker. Pre-existing uncommitted capture files/changes are preserved.
No license text, ROM/save, reference checkout, gallery image or emulator core
is modified. Version and Cargo.lock remain unchanged by this audit.
