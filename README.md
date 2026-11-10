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

## Installation

1. Open [GitHub Releases](https://github.com/Noktyros/RetroHatch/releases).
2. Download the latest **`RetroHatch-v<version>-windows-x86_64.zip`** under
   **Assets**. The source ZIP is intended for developers.
3. Extract the entire archive into a folder of your choice.
4. Double-click **`RetroHatch.exe`**.

**No Rust or Visual Studio installation is required.** The portable package
targets 64-bit Windows 10/11 with an OpenGL 3.3-capable graphics driver. Keep the
included license and attribution files with the executable. ROMs are not
included; import firmware that you have the appropriate rights to use after
starting the application. Settings, imported ROMs and saves are stored in your
Windows user-data directory, outside the extracted application folder.

Release downloads are available only to people with access while the repository
is private. If no release has been published yet, there is no Windows ZIP to
download; developers can [build from source](#build-from-source). Preparing the
workflow does not resolve the artwork-provenance blockers described above.

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
Connection and other MCU families are not implemented.

**On/Meets integration, firmware validation pending:** the reused MIT TG18 ARM
backend is connected to the existing library, color LCD, A/B/C controls, audio,
pause/reset and private saves. No suitable local On/Meets firmware was available
for a real boot or gameplay test; this is not a claim of verified playable On
support. See [TG18 integration](developer/TG18.md).

## Build from source

These instructions are for developers. Downloaded Windows release packages do
not require a Rust toolchain or Visual Studio.

1. Install Rust 1.99.1 with rustup and the `x86_64-pc-windows-msvc` target.
2. Install Visual Studio Build Tools with **Desktop development with C++** and
   a Windows SDK.
3. Open PowerShell in a writable source checkout:

```powershell
cd app
cargo build --release --locked
./target/release/RetroHatch.exe
```

Use an optimized build for gameplay. For a distribution candidate, run
`powershell -NoProfile -ExecutionPolicy Bypass -File app/tools/build_release.ps1`
from the repository root instead. This runs the same locked release build with
build-path privacy remapping and a statically linked C runtime. The
[release guide](developer/RELEASE.md) explains packaging and tag automation.

No ROM, private reference checkout or private documentation is needed to
compile. Cargo downloads locked dependencies on the first online build. The
matching source ZIP attached to a release also includes vendored dependencies
for offline Cargo builds. Native Windows dialogs and audio are the supported
host integrations; other operating systems are not validated release targets.
See [CONTRIBUTING.md](CONTRIBUTING.md) for checks.

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

On/Meets imports require an exact 8-MiB flash image beginning with `SPII`.
Nine upstream code fingerprints supply revision labels for known On (EN) and
Meets (JP) images; these are FNV-1a code fingerprints, not invented full-ROM
SHA-256 entries. Full-dump SHA-256 still isolates every local library entry and
save. An unknown TG18-shaped image asks for confirmation once. Filenames never
prove a variant; SOME is not supported. Real On/Meets firmware testing is pending.

## Controls and settings

| Action | Control |
| --- | --- |
| A / B / C | Left / Down / Right Arrow; also A/B/C or 1/2/3 and on-screen buttons |
| Tap / Knock | Up Arrow, Tap button or LCD click, only on supported sensor devices |
| Pause / Resume | Space or the Emulation menu |
| Reset | F5 or the Emulation menu, with confirmation |
| View all shortcuts | Help > Keyboard Shortcuts |

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
