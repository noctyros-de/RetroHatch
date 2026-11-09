<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros -->

# RetroHatch

RetroHatch is a Windows-focused Tamagotchi emulator written in Rust, with a
minimal native egui interface and a deterministic E0C6200 CPU/MCU core. Its goal
is accurate, approachable emulation and a maintainable foundation for community
contributions, without decorative device shells.

**Development status:** this repository is currently private. These files
prepare a future open-source release; they are not a release announcement.
Unresolved LCD artwork provenance currently blocks public distribution of the
affected source assets and binaries. See [third-party licensing](THIRD_PARTY_LICENSES.md)
and the [release checklist](developer/RELEASE.md).

## Supported devices

| Device | Implemented and locally verified |
| --- | --- |
| Tamagotchi P1 | Japanese user-provided firmware boot, input and gameplay; shared complete saves |
| Tamagotchi P2 | Boot, hatching, buttons/LCD/audio, reference comparison, timing and complete saves |
| Tamagotchi Genjintch | Dedicated LCD, buttons and tap sensor; local firmware/reference validation |
| Tamagotchi Angel (Angelgotchi) | Dedicated LCD, buttons and tap sensor; local firmware/reference validation |
| Tamagotchi Morino | Dedicated LCD, buttons and sensor; local firmware/reference validation |
| Tamagotchi Umino (Ocean) | Dedicated LCD, buttons and tap sensor; local firmware/reference validation |

Verification covers the available local images and focused gameplay sequences,
not every regional revision, minigame or complete device lifetime. TamaOtch is
a recognized **unsupported research target**: its exact LCD mapping, geometry
and board settings are unresolved. Recognized TamaOtch firmware is refused with
an explanation; another device's display is not substituted. Tamagotchi
Connection/ON/Meets and other MCU families are not implemented.

## Build and run on Windows

There is no public binary download provided by this preparation task. To build
an authorized local source checkout:

1. Install stable Rust with rustup, including the MSVC Windows target, and
   Visual Studio Build Tools with Desktop development with C++ and a Windows SDK.
2. Use an ordinary writable checkout. The validated toolchain is Rust 1.99.1;
   the locked GUI dependencies declare their own minimum compiler requirements.
3. In PowerShell, from the repository root:

```powershell
cd app
cargo build --release --locked
./target/release/RetroHatch.exe
```

Use the optimized build for gameplay. Future distribution candidates must also
use the build-path privacy wrapper described in [developer/RELEASE.md](developer/RELEASE.md). No ROM, private reference checkout or
private documentation is needed to compile. Cargo downloads the dependencies
specified in Cargo.lock on the first online build. Native Windows dialogs and
audio are the supported host integrations; other operating systems are not
validated release targets. See [CONTRIBUTING.md](CONTRIBUTING.md) for checks.

## Import your own firmware

Choose **File > Import ROM...**. RetroHatch validates the unpacked big-endian
12-bit word structure and computes SHA-256. A verified known hash identifies
its device; genuinely ambiguous content asks for the model once and remembers
that association locally. Filenames do not establish identity or region.

The application copies the image into its private local library without moving
or deleting the original. **My Tamagotchis** starts imported devices with one
click. **Manage ROMs...** edits associations and repairs/removes internal
copies; removal never deletes original ROMs or saved games. Identical imports
share content storage. P1/P2 expect 12,288-byte unpacked images; the four
supported sensor-equipped profiles expect 16,384 bytes. Size alone does not
prove that a model is supported. Packed dumps are not accepted.

## Controls and settings

| Action | Control |
| --- | --- |
| A / B / C | Left / Down / Right Arrow; also A/B/C or 1/2/3 and on-screen buttons |
| Tap / Knock | Up Arrow, Tap button or LCD click, only on supported sensor devices |
| Pause / Resume | Space or the Emulation menu |
| Reset | F5 or the Emulation menu, with confirmation |
| View all shortcuts | Settings > Keyboard Shortcuts |

Settings preserves five pastel themes, button-label visibility, mute/volume,
autosave and exit-save preferences. Green is the new-installation default.
Diagnostics are optional and hidden by default. Emulation continues without
window focus and while minimized, unless explicitly paused. Closing the app
stops emulation; it does not simulate offline growth while closed.

## Save states and local privacy

File provides Save Now, three manual slots, slot loading and save import/export.
Full states include CPU, memory, peripheral/timer/interrupt/oscillator state,
audio generation and emulated time. ROM SHA-256 and device profile separate
save identities, including P1 versus P2. Integrity/version/identity checks reject
incompatible states. Autosave is separate from manual slots; options are Off,
1, 5, 10, 15 or 30 minutes. Save on Exit is independently enabled by default.

Normal Windows data lives under `%LOCALAPPDATA%\RetroHatch`: configuration,
`rom-library`, and per-ROM/device `saves`. Compatible autosaved progress resumes
on reopening. `RETROHATCH_DATA_DIR` is a deliberate development override; keep
it outside a source checkout. The emulator has no implemented cloud sync or
ROM upload service. Treat saved states, library metadata and diagnostics as
private. See [privacy and architecture](developer/ARCHITECTURE.md).

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md), use English, and submit small changes
with reproducible tests and hardware/provenance evidence. While the repository
remains private, access is controlled by the owner. Future public contributions
will follow the same issue/PR guidelines; no repository setting is changed here.

## License and acknowledgements

Original RetroHatch contributions are Copyright (C) 2026 Noktyros and licensed
under **GNU GPL-3.0-only**, not an automatic grant of later GPL versions. See
[LICENSE](LICENSE), [COPYRIGHT.md](COPYRIGHT.md),
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) and [LICENSES](LICENSES/).
Third-party code, fonts and other data retain their own notices and terms.
The GNU license text is not a clearance of unresolved artwork rights.

RetroHatch is an unofficial fan project. It is not affiliated with, endorsed
by, or sponsored by Bandai. Tamagotchi and related names belong to their
respective owners. Copyrighted ROMs and firmware are **not included**; users
must obtain appropriate rights to use their own images. Do not request or
submit firmware downloads, dumps or private saves.
