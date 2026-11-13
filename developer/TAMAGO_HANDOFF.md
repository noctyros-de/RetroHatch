<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Tama-Go handoff

Development was preserved on 12 November 2026 in the unchanged local branch
`feature/tamago-experimental`, commit `7c96d6ee95c0f9953a76132c416f7de3e5a736c7`,
based on published 0.8.0. On 13 November the owner authorized recovering it onto
the published 0.9.0 baseline and releasing it as **experimental** in 0.10.0,
accepting the documented unspecified BSD variant without declaring it resolved.
Connection V3 stays present with separate profile code 26/schema 6; Tama-Go
retains code 25/schema 5. No firmware, figure dumps or private saves are included.

## Working implementation and evidence

- Separate native Rust GPLB52320A/6502 backend, 151 reference opcodes, banked ROM,
  RAM/GPIO, EEPROM, timers/interrupts, STOP/wakeup, DAC audio and deterministic clock.
- Actual local main firmware boots, completes setup and hatching, and displays a
  controllable pet. The visible four-level matrix is **48x31**, despite a 64x32
  controller. Ten fixed electrical icon states currently use text labels.
- Thirty actual-firmware checkpoints exactly match the C reference CPU/RAM/LCD/I/O.
- Native Windows library loading, arrow buttons, pause/reset, screenshots, close/
  reopen with the selected figure and continued execution while minimized tested.
- All three manual slots, save import/export, autosave infrastructure and automatic
  resume are integrated. Schema 5/profile code 25 preserve complete machine and
  selected figure state; old backend save formats are unchanged.
- All twelve regular figure identities and both full dumps have genuine firmware
  SPI/content/menu tests. Makiko-to-full-Chamametchi live swapping preserves EEPROM
  and does not reset the CPU; insertion includes a 2.5-second emulated absence.
- Measured 60.105824 s real / 60.103026 s emulated = 99.9953% average speed.
- Last full validation: 220 Rust tests passed, one interactive browser test ignored;
  existing Python comparisons and 25 tooling tests passed, including the new C
  comparison. Dependency notices checked for 106 packages. Windows release built.
  These are existing results, not tests rerun by this branch-preservation task.

## Firmware and figure formats

No firmware/accessory/save files are committed. Originals remain in the private
workspace's `local/roms/Tamagotchi TamaGo/`. Exact hashes are in `TAMAGO.md`.

| Image | Validated structure / scope |
| --- | --- |
| Main `tamago.bin` | 655,360 bytes; reset vector CC00; full SHA-256 catalog identity |
| Figures 03-08 | Mametchi, Memetchi, Kuchipatchi, Chamametchi, Violetchi, Ringotchi; 349,536-byte short captures |
| Figures 09-14 | Kuromametchi, Sebiretchi, Ponytchi, Ichigotchi, Shimashimatchi, Makiko; 524,288 bytes |
| Full Mametchi / Chamametchi | 4,194,312 bytes; first 4 MiB exposed as flash, eight trailing capture bytes preserved in the original imported file; parser also accepts 4,194,304-byte raw images, not separately firmware-qualified |
| `Tama-Go_Debug.bin` | 524,288-byte accessory, ID FE; not standalone main firmware; firmware reads it, diagnostic-mode activation unverified |

Accessories use AA 55 at offsets 1 and 17 and an ID at offset 20. SPI command 03
uses full-duplex command/address responses and an incrementing 24-bit flash read.
They are private accessories under Emulation > Tama-Go Figure, never standalone
models. Imported source-hash files live in the user's `tamago-figures/` directory;
originals are unchanged. Saves may embed figure data and must remain private.

## Outstanding work and provenance

Read **TAMAGO_LICENSE_AUDIT.md first**. The new author finding is Bryon Vandiver
(`asterick`): all 85 reachable JavaScript commit-author records name him. Four
historical package.json versions say only BSD. The exact BSD version, complete
copyright notice and permission scope are unresolved; no incompatible clause
has been proven. The affected inherited CPU/bus/EEPROM/LCD expressions are not
independent merely because they were translated to Rust.

Paramix-Studio's pinned root GPLv3 predates its T7 import, but mameise cannot grant
rights belonging to predecessors without authority. Obtain the exact original
grant from asterick and intermediate Android/Pebble/ESP32/C-port provenance from
mameise, with revision/file coverage. No upstream author has been contacted.
A compatible clarification may allow keeping this implementation and completing
notices. Otherwise replace the affected portions from appropriate licensed code
or a documented independent implementation; replacing only the CPU is insufficient.

Other unfinished qualification: original fixed-icon geometry; exhaustive short/
full-figure minigames and extra content; shop purchases/long-term progression;
debug cartridge; physical audio comparison and extended gameplay. IR, device
links and TamaTown online interactions are not implemented. Native layout was
visually inspected and egui submenu actions tested; later physical mouse-popup
inspection was blocked by Windows foreground activation. Do not overstate this.

## Where to continue

- `app/src/core/tamago/{cpu,mod,figure,state}.rs` and `UPSTREAM.json`: backend,
  state schema and exact upstream source hashes.
- `app/src/host/figures.rs`: private accessory storage, hashing and safe repair.
- `app/src/host/{emulator,runner,library,fingerprints}.rs`: dispatch, pacing,
  saves, figure commands and ROM identity; profile/state IDs in core files.
- `app/src/ui.rs` and `ui/{help,screenshots,recording}.rs`: existing frontend.
- `app/examples/tamago_probe.rs`, `app/tools/compare_tamago.py`,
  `tamago_reference.c`, `tamago_reference_host.c`: opt-in actual-firmware checks.
- `developer/TAMAGO.md`, `developer/TAMAGO_LICENSE_AUDIT.md`, root third-party
  notices and `LICENSES/Paramix-Studio-GPL-3.0.txt`: findings, boundaries and grant.
- Private `local/test-data/tamago-research/`: immutable source-only reference
  downloads, firmware inventory, boot/figure/swap/native captures and test logs.
  `local/test-data/tamago-license-audit/`: historical/provenance evidence.
  Never import either directory into Git or a release package.

## Resume steps

1. Read the dated audit and owner decision above; do not restart the historical
   investigation without new evidence. Preserve accurate upstream notices.
2. In the public project directory, inspect `git status --short` and preserve any
   unrelated work, continue from the integrated development branch.
   Locate this checkpoint with `git log -1 --oneline feature/tamago-experimental`.
3. Read this handoff and the two Tama-Go reports. Do not redownload firmware or
   change originals. The unrelated `developer/PROJECT_STATUS.md` draft is excluded
   from this feature commit and stays untracked, byte-for-byte, across switches.
   Its existing branch-only Tama-Go link was deliberately not rewritten.
4. In `app/`, run `cargo fmt --check`, `cargo check --locked`, then focused tests
   with `cargo test --locked tamago`. Before declaring completion run the entire
   suite and relevant existing Python checks documented in TAMAGO.md.
5. Use private disposable runtime data (`RETROHATCH_DATA_DIR` and
   `RETROHATCH_CAPTURE_DIR`) for destructive real-ROM tests. Example from `app/`:
   `cargo run --release --locked --example tamago_probe -- <ROM> <PRIVATE_OUTPUT> <FIGURE-or-dash> 120 menus`.
   For the C comparison supply `--reference` pointing to the private Paramix `t7`
   tree and `--native` pointing to a no-figure 30-second probe's checkpoints.bin.
6. Complete the remaining qualifications honestly, then follow the existing
   version policy only for a completed implementation. Rebuild Windows using
   `powershell -NoProfile -ExecutionPolicy Bypass -File app/tools/build_release.ps1`
   from the repository root. No push/release is authorized by this handoff.

## Isolation and artifact safety

The feature checkpoint contains source, synthetic tests and documentation only.
It does not track the independent PROJECT_STATUS draft, ROMs, figures, saves,
reference checkouts or real-game test data. At the November 12 pause, main and
v0.8.0 were untouched; the preserved experimental branch remains unchanged.
The November 13 integration starts from the published 0.9.0 main baseline.
The current owner request authorizes a validated 0.10.0 publication; the older
pause instruction does not prohibit that authorized integration.

The earlier experimental EXE and unrelated report backup remain private in
`local/backups/tamago-pause-20261112/`. Always build the selected source revision
before packaging; do not package a stale binary. Never include the untracked
`developer/PROJECT_STATUS.md` draft or private test evidence unintentionally.

Current integration test evidence and remaining technical scope are recorded in
`TAMAGO.md`, section "0.10.0 integrated release validation". The preserved
experimental branch remains unchanged; resume future work from main after 0.10.0.
