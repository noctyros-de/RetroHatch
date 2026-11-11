<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Contributing to RetroHatch

Welcome. Useful contributions include accurate device research, emulator fixes,
focused tests, accessibility/usability work and clear documentation. Be kind,
describe observations precisely, and discuss disagreements respectfully.

## Access and proposals

The public repository is maintained by Finn Lülsdorf (Noctyros) at
https://github.com/noctyros-de/RetroHatch. Search existing issues, explain the
concrete problem and propose a focused PR on a topic branch. Large changes
benefit from prior design discussion. The maintainer reviews changes and
controls merges. No CLA or transfer of copyright is requested by these files.
GPL rights to lawful copies, including redistribution/forking, remain intact.

## Windows development

Install stable Rust (rustup, MSVC target), Visual Studio C++ Build Tools and a
Windows SDK. Rust 1.99.1 is the toolchain validated for this preparation; a lower
project MSRV has not been independently tested. From the source root:

```powershell
cd app
cargo fmt --check
cargo check --locked
cargo test --locked
cargo build --release --locked
```

Use `cargo fmt` to format a change before checking. Preserve Cargo.lock for this
application; explain any dependency update and regenerate its license inventory.
The normal Cargo build/tests require neither proprietary ROMs nor `references/`.
Run gameplay with the optimized executable. Put test user data outside the
checkout using `RETROHATCH_DATA_DIR` so ordinary settings/saves stay untouched.

## Architecture and directories

- `app/src/core/cpu`: instruction interpreter and atomic error semantics.
- `app/src/core/mcu`: deterministic clocks, I/O, timers, IRQ and buzzer.
- `app/src/core/profile.rs`, `tamagotchi.rs`, `state.rs`: supported hardware,
  shared device implementation and versioned complete saves.
- `app/src/host`: single-owner worker, pacing, native audio/dialogs, private
  ROM library, settings and safe save storage.
- `app/src/ui.rs`, `ui/`, `lcd_render.rs`: egui presentation and input handling.
- `app/assets/lcd`, generated geometry files: display-only data; independent
  symbol provenance/preferred vectors remain specific open review items.
- `app/tests/data`: synthetic programs and reference expectations, not firmware.
- `app/tools`, `app/examples`: differential checks and local-only diagnostics.
- `developer/`: public, sanitized technical documentation.
- `LICENSES/`, `THIRD_PARTY_LICENSES.md`: dependency inventory and full notices.

Private references, branding originals, ROMs, builds and research belong in the
optional sibling `../local/`, outside the Git repository. Public AGENTS.md and
VS Code settings are safe project instructions; machine-specific Cargo settings
remain ignored. See [workspace setup](developer/WORKSPACE.md). Public `docs/`
contains the approved logo and owner-managed gallery; place technical guides
in `developer/`, without personal paths or private data.

The README Screenshots/Gameplay Videos sections and selected gallery files are
owner-managed. Preserve their entries, order and captions. Do not add captures,
delete/rename media or regenerate the gallery without explicit authorization.
See [gallery instructions](docs/screenshots/README.md).

## Adding or changing a device

1. Establish the exact MCU, ROM format, clocks, masks, electrical inputs and LCD
   bit-to-segment mapping from legitimate technical evidence. Cite sources.
2. Reuse shared behavior only when compatibility is demonstrated. A filename,
   matching byte length, similar shell or another emulator's generic icons is
   insufficient. Do not invent missing geometry or a fake boot.
3. Add a canonical profile ID and capabilities. Preserve stable save profile
   codes and existing payload versions; add a new code instead of renumbering.
4. Add independently verified exact hashes only when there is evidence. Unknown
   images must continue through explicit, persistent model confirmation.
5. Supply focused synthetic CPU/MCU/input/LCD/save tests and Python reference
   checks where evidence exists. Keep unsupported instructions/accesses explicit
   and atomic. Preserve earlier tests and real-time pacing.
6. Validate boot/gameplay/audio/timing with legally user-supplied local firmware
   when available, while keeping firmware, traces, saves and captures out of Git.
   Distinguish verified behavior, untested behavior and evidence blockers.
7. Record asset origin, authorship, licenses and the preferred editable source.
   Record unresolved icon rights; a repository CC0 declaration alone does not
   establish rights that the upstream author did not hold.

## Optional Python/reference checks

Existing `app/tools/compare_*.py --check` and `extract_*.py --check` use an
independently obtained local BrickEmuPy checkout at `../local/references/BrickEmuPy-main` (or set RETROHATCH_REFERENCES_DIR).
The base dependency is Python's standard library; the exact SVG mask/quality
checks additionally use PyQt6, and PCM/LCD probes use locally built release
examples (`cargo build --release --locked --examples`). P2 reuses the checked P1
geometry. These developer dependencies are not part of the executable.

Obtain only source/resources whose terms permit your use; never download firmware
to run a test. Do not add a reference checkout to Git. If references or PyQt6 are
unavailable, report which optional checks were not run; never describe them as
passing. Update synthetic fixtures only after explaining a genuine reference
change, not to hide a regression.

## Quality and licensing expectations

Use professional English for code/comments, UI, documentation, test descriptions,
filenames and commit messages. Keep changes focused, maintain core/host/UI
boundaries, and explain what was verified and what remains uncertain. Add useful
tests for behavior changes rather than large matrices that duplicate an
implementation. The existing complete regression suite must keep passing.

Submit only material you have the right to license under GPL-3.0-compatible
terms. Original project contributions should use GPL-3.0-only and accurate
copyright notices. Retain third-party licenses and identify borrowed/adapted
material in the PR and attribution documents; compatible upstream code is not
automatically exclusively owned or relicensed by RetroHatch.

Do not submit copyrighted firmware/ROM dumps, private saves/library indexes,
personal data, credentials, private diagnostics or unlicensed artwork. Do not
attach those to issues. Review your staged diff and relevant history; .gitignore
cannot protect already tracked content. Document lawful sources without linking
to unauthorized firmware. See [SECURITY.md](SECURITY.md) for private reporting and
[developer/RELEASE.md](developer/RELEASE.md) for distribution obligations.
