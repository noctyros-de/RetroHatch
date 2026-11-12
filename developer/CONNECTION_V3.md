<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros; original documentation only -->

# Connection V3 European hardware integration

Status: **experimental; basic offline gameplay is verified**. Ten original GPL
replacement symbols are now built in; no private LCD assets are required. See
[release review](CONNECTION_V3_RELEASE_AUDIT.md) for the V3 license decision and
pre-existing questions that remain tracked separately. Application version
is **0.9.0**; the hatching investigation changed no emulator behavior. Development branch: `feature/connection-v3-eu`.
Publication is conditionally authorized only after all licensing/validation gates
pass. The owner authorized the 0.9.0 release after a scoped V3 review; older provenance
questions remain documented rather than being treated as newly cleared.

## Firmware and board identity

The private user image `Connection v3 EU-US.bin` is 524,288 bytes, SHA-256:
`7f4da8dc89d3843b42b3ba73b9311989332706978af5367b5ef041c365a1902f`.
Reset/NMI/IRQ vectors at ROM offsets 0x7FFC/0x7FFA/0x7FFE are
0xC800/0xE247/0xE20B. No bytes have been patched, generated or recovered.

BrickEmuPy's EU and US profiles explicitly select the same ROM. The hash identifies
this image, **not a uniquely European regional dump**. RetroHatch selects the EU
board: PD pull-up 0x06 (US uses 0x02). Unknown structurally compatible images
require model confirmation; filenames do not prove identity. Upstream README
warns of a high probability of V3 dump errors. Matching the reference proves
emulator equivalence for tested execution, not ROM integrity or correct hardware.

## Architecture and origin

`app/src/core/splb32/` is a separate native Rust backend. It directly ports
azya52/BrickEmuPy's CC0 SPLB32, SPLB32sound and HT24LC08 behavior.
`UPSTREAM.json` records exact local source hashes; no unsupported commit ID is
invented. `tools/port_splb32_cpu.py` reproduces the 151 implemented CPU opcodes.
Unassigned encodings return an error before modifying CPU state.

The MCU uses a 4-MHz oscillator and exact binary-rational 32768-Hz divisor.
CPU division, STOP, subclock timers, interrupt vectors/stack, wake flags,
GPIO and LCD activation follow Python. One explicit bus-width correction exposes
only the low byte of the unbounded internal timebase counter at SFR 0x0C. An
8-bit read must not place a multi-byte value into A/N; Paramix independently
notes the same Python discrepancy at `v3cpu.c:193`. The internal counter/IRQ
phase is preserved. No Paramix implementation code was copied for this mask;
a focused synthetic regression covers the difference. The tested actual-ROM
trace is unchanged by this correction. RAM is 0x40-0x4FF (1216 bytes), LCD RAM
0x3E00-0x3FFF (512 bytes); the 0x4000-0xBFFF ROM window is banked, while
0xC000-0xFFFF uses the fixed first-bank region. A/B/C are active-high PA bits
1/2/4; release drives low. Left/Down/Right and existing aliases are unchanged.
There is no tap sensor. The disconnected IR receiver remains low on PB bit 4;
no peer communication is supplied.

The timer's external/cascade modes are TODOs in Python. Rust explicitly stops
with an error rather than entering that reference's non-advancing loop. This
limitation was not encountered in the actual firmware tests. No host-wall-clock
patches or game-clock acceleration are used. Closed applications do not advance
emulated time. Diagnostic interrupt counts currently count maskable IRQ entries.

Paramix Studio's V3 implementation was inspected as research: it credits the
same BrickEmuPy sources. No Paramix C, custom PNGs, T7 fallback icons or paused
Tama-Go/asterick code was incorporated. The isolated Tama-Go branch remains
`feature/tamago-experimental`, commit `7c96d6ee95c0f9953a76132c416f7de3e5a736c7`.

## LCD and asset distribution boundary

The SVG describes **960 matrix cells (32 x 30), plus ten fixed symbols**.
`core/splb32/lcd.rs` contains factual bit mapping and rectangle positions only.
The renderer presents the matrix in a 352 x 363 canvas with two rows of five symbols;
there are no shell graphics or invented text-label replacements. Screenshots
and recording accept the non-square framebuffer through existing host paths.

Original fixed-symbol silhouettes have unresolved independent ownership, despite
BrickEmuPy's root CC0 declaration. They are **not embedded or checked into this
integration**. A public build now uses original functional replacement drawings from
`app/assets/connection-v3/`. Their editable SVG sources and generated alpha
coverage are included under GPL-3.0-only. These are distinct replacement designs,
not tracings or a claim of authentic original icon shapes. All 970 electrical
segments are represented without a private pack.

For private testing with appropriately authorized local assets:

```powershell
python -B app/tools/extract_connection_v3_lcd.py --check
python -B app/tools/extract_connection_v3_lcd.py --check --private-output ../local/display-assets/connection-v3.rhlcd
$env:RETROHATCH_V3_LCD = (Resolve-Path ../local/display-assets/connection-v3.rhlcd)
```

The extractor refuses icon output inside the public repository and refuses to
overwrite an existing pack. It rasterizes actual SVG paths, not approximations.
Alternatively put the authorized pack in the user-data directory at
`display-assets/connection-v3.rhlcd`. The versioned `RHV3LCD1` pack carries
352/363 dimensions, 970 unique reference bit addresses and bounded alpha tiles;
the decoder checks all mappings and bounds. Do not redistribute it without
resolving underlying rights. The original local SVG and artwork remain untouched.

## Audio and persistence

Two DAC/timer-tone channels preserve reference envelope/toggle semantics.
Transitions are area-integrated to 44100-Hz mono PCM with continuous phase,
half-gain channel mixing and 8192-unit output gain; this is a host presentation
choice, not a measured physical-speaker model. Existing Windows audio, mute and
volume controls are reused. No arbitrary smoothing is applied.

Full `.rhstate` schema **6**, profile code **26**, serializes CPU/RAM/LCD,
banking/GPIO, timer phases, IRQ/NMI/wakeup, oscillator/STOP, emulated time,
audio envelope/DAC/resampler/queue, and EEPROM contents plus in-flight I2C state.
Schema 5 / code 25 remain reserved for the isolated Tama-Go experiment. Existing
schemas and profile codes are unchanged. Firmware identity, format and SHA-256
integrity are validated before replacement. Three slots, autosave, recovery and
import/export use existing atomic asynchronous storage.

EEPROM is distinct from full-state continuation: `eeprom.rhstate` uses schema
**7**, containing only the 1024-byte nonvolatile data, ROM identity and checksum.
The ordered writer persists changed data at one-second checks and on save/exit,
even with full-state autosave disabled. Cold boot loads it before any full
resume; successful full resume then restores the complete bus and MCU. Reset
preserves EEPROM. Each file stays under `saves/{profile-id}-{rom-sha256}/` in
Windows user data. It cannot overwrite a ROM or another firmware's save.
Up to one second of EEPROM changes may be lost after an abrupt process kill;
safe writes do not imply durability before a queued write completes.

## Validation evidence (12 November 2026)

- Unchanged real firmware boots and shows an animated egg in the native Windows
  application; B opens the clock and A/B/C alter setting screens. Correct model
  import and library restart were observed. No firmware was distributed.
- 30 seconds, 60 half-second checkpoints with scripted A/B/C: physical/logical
  PC, A/X/Y/SP, exact fractional oscillator cycles, all RAM, LCD RAM, EEPROM and
  all 64 SFR reads match unmodified BrickEmuPy. This exposed and corrected the
  reference's write-only channel-0 data register (reads zero).
- The independently compiled Paramix C core (batch skipping disabled) also
  matches all 60 real-ROM checkpoints for cycles, CPU registers, RAM, raw LCD
  and EEPROM. The same setup/incubation sequence at 510 seconds produces the
  same final LCD/EEPROM as Rust. The private MSVC build needed only an
  `offsetof` compatibility macro supplied in compiler flags; upstream sources
  were not edited or included in RetroHatch.
- 5960 actual reference DAC transitions produce **1,323,000 identical PCM16
  samples**, maximum difference zero, including button sounds. Windows audio
  output opened successfully in the pacing test. Human listening/physical
  buzzer fidelity remains unverified.
- A monotonic 60-second worker/audio run measured **60.032593 s wall time /
  60.025306 s emulated**, **99.9879%** average speed. Core work total 128.15 ms;
  snapshots 688.55 ms. No real-time throughput shortage was observed.
- Full-state continuation was compared byte-for-byte every five seconds in
  real-ROM probes. Two actual native launches restored and saved progress at
  **158.324126 s** and **160.380322 s** from a 150-second checkpoint. The
  application stayed responsive and advanced during a two-second minimize test.
- Native client captures show unclipped LCD and A/B/C controls. Native scripted
  screenshot key messages did not produce files in this desktop focus context;
  non-square UI/library routing is covered by automated UI tests; capture paths
  use the shared image/encoder infrastructure,
  not claimed as a successful native shortcut test.
- Focused synthetic coverage: 604 reference cases across all 151 CPU opcodes;
  bank/RAM/LCD boundaries; STOP/wake; timer/audio/IRQ continuity; I2C block/page
  and sequential-read wrap; in-flight EEPROM restoration; atomic reserved-opcode
  errors; corrupt/foreign save rejection; private LCD mapping/bounds; library,
  slots 1-3, import/export, restart, separate EEPROM file and corruption rejection.
- Full final Rust regression run passed: **186 library + 38 UI tests**, one
  existing ignored test. A subsequently added byte-bus regression and all eleven
  SPLB32 tests also passed. All twelve selected existing/new Python checks passed,
  including E0C6200/MCU/devices, S1C33 and buzzer PCM. Windows release build passed.

## Hatching investigation: root cause established

The earlier apparent hatching failure was an **incorrect test procedure**, not
a demonstrated CPU, timer, GPIO, EEPROM or ROM failure. Earlier scripts confirmed
the initial hour **0**, which this firmware uses as "clock never set", or changed
month/day without changing the hour. Merely waiting longer cannot finish setup.
The existing functioning emulator required no patch or architectural change.

A controlled pair of cold boots differed only by one genuine A pulse in the
selected hour field. No RAM/EEPROM values were injected, no firmware-clock variables or oscillator ratios were changed, and no existing
save was modified. Long firmware probes ran unpaced; wall-clock pacing was
measured separately through the unchanged host worker. Fresh controls used the reference's
zero-filled EEPROM default; successful tests used only firmware-generated writes.

| Observation | Zero-hour control | Valid-hour run |
| --- | --- | --- |
| Before the isolated A pulse at 8 s | Same RAM and execution | Same RAM and execution |
| RAM 0x85, hour | 0 | 1 after A |
| RAM 0xB9 bit 2, clock-stop guard | Remains set | Cleared after minute confirmation at 12 s |
| Seconds at 0x81/0x82 | Stay zero | First increment around 12.500 s, then one per emulated second |
| Mode at 0x70/0x71 after birthday confirmation | Remains 0x01/0x01 | Changes to 0x0A/0x22: USER NAME |
| Hardware 1024-Hz timebase at 20 s | 20,481 ticks | 20,481 ticks |
| Username completion | Not reached | Five real A/B letter selections |
| EEPROM | All zero | First nonzero data around 39 s |
| Incubation | Not started | Egg, then hatch animation/baby around 100-102 s |
| Following state | Setup waiting | Pet NAME input, then normal care/menu operation |

Firmware observation identifies two relevant guards at **physical ROM offsets**
0x0D568 and 0x66B85. The first tests the hour before clearing RAM 0xB9's clock-stop
bit; the second tests it before entering username mode. With hour zero it returns
without advancing setup. With hour one the same code takes the normal path.
At roughly 8.050 s the ordinary key handler writes hour 1; at roughly 12.051 s
minute confirmation releases the guard. These are observations, not firmware
patches. The hardware timer/CPU instruction counts advance in the failing control,
so a stopped game clock must not be mistaken for stopped MCU time or a deadlock.

The pinned Paramix frontend independently documents RAM 0x85 as hour 1-12, with
zero meaning never set, and waits for changing seconds before synchronizing its
host clock. Its automatic host-time injection was **not used** in these tests.
See [Paramix source](https://github.com/mameise/Paramix-Studio/blob/675b904ab1c8a9086b977a82cbc47cd0c319187f/v3/src/app/main.c),
clock comments and `clk_running_val` near lines 554-560.

### Reproducing correct setup

From a fresh egg: B opens the clock; B confirms month; B confirms day. In the
hour field use A to select **1 through 12**, then B to confirm the hour and B
to confirm minutes. Confirm birthday month/day with B, then select five username
letters using A/B. Wait for incubation/hatching, and enter the pet's five-letter
name. C cancels or moves back; do not accidentally cancel initialization.

This agrees with the operator sequence in the original
[Bandai V3 instruction sheet, setup steps 3-7](https://manuals.plus/m/c974fddb1cb920ef1d63d1608dec548dc902c8227f1f339e9e1ca1982cdf3f22.pdf).
Do not replace these inputs with writes to clock, state or hatch variables.

One focused opt-in regression preserves this finding without embedding firmware:

```powershell
python -B app/tools/check_connection_v3_setup.py --rom "../local/roms/Connection v3 EU-US.bin" --output ../local/test-data/v3-setup-new
```

Use a **new** private output directory. The tool refuses existing directories
and public-repository output. It checks the exact ROM hash, a zero-hour negative
control, clock guards, username mode, EEPROM initialization, exact 60-second
clock advancement and post-incubation LCD transition. The underlying existing
probe checks byte-exact full-state continuation every five emulated seconds.
Visual inspection of the actual hatch frames is additional evidence, not replaced
by a generic LCD-change assertion.

### Reference and gameplay validation

- Corrected input sequence: **240 half-second checkpoints over 120 seconds**
  match unchanged BrickEmuPy for physical/logical PC, A/X/Y/SP, cycle totals,
  all RAM/LCD/EEPROM and 64 SFR reads. No meaningful divergence was found.
- The pinned Paramix C core independently matches all 240 checkpoints for cycles,
  registers, RAM, raw LCD and EEPROM, including real hatching. It uses no host
  clock synchronization. Agreement is evidence for this execution, not proof
  that every device behavior or ROM byte is correct.
- At emulated 60 s the game clock reads 01:00:47; at 120 s it reads 01:01:47.
  This confirms **60 actual game-clock seconds per 60 emulated seconds**.
  The earlier unchanged worker/audio measurement remains 99.9879% real-time speed.
- The real firmware shows USER NAME, accepts the username, writes EEPROM, shows
  the egg/hatching/baby and pet NAME, accepts that name, and enters care menus.
  Food selection and eating animation were observed. A subsequent health screen
  shows **one filled hunger heart**, compared with zero before feeding. A/B/C
  selection, confirmation and cancellation operate normally.
- EEPROM changes from the zero-filled control to 11 nonzero bytes after username
  setup and 25 after pet naming. Actual contents remain private. Full-state
  continuation remains byte-identical across timer/EEPROM/audio activity.
- Two **new, isolated** native Windows user-data sessions resumed actual gameplay
  at 216.000006 -> 218.019558 -> 220.056683 emulated seconds. The visible health
  screen retains the fed hunger heart after reopening; the separately written
  `eeprom.rhstate` retains all 25 nonzero bytes and matches the firmware-generated
  EEPROM image exactly. No previous test save or user save was overwritten.
- Button/name/care sequences generated nonzero PCM; audio synthesis and host
  timing are unchanged. Physical speaker fidelity/listening was not newly tested.

- Investigation regression checks passed: `cargo fmt --check`, `cargo check
  --locked`, **187 core/host tests** (one existing ignored test), all five focused
  V3 host/LCD/UI tests, and the opt-in cold-boot negative/positive setup regression.
  This investigation adds only a diagnostic tool and documentation; no emulator
  instructions, hardware behavior, UI controls or runtime workaround were changed.

### ROM integrity and remaining limitations

SHA-256, SHA-1 `0c11cf7c91d1c11d0afa9cab9b8c70f386fd1ad6` and CRC32 `301f0e87`
identify this local image. Targeted searches found **no independent, trusted
matching checksum or second verified dump**. Its size, reset vectors, bank
layout and tested execution are compatible with the selected profile. None of
those checks certifies every byte. Firmware was read only and its original hash
was rechecked after testing.

BrickEmuPy's [README](https://github.com/azya52/BrickEmuPy) warns about possible
dump errors. [Issue #40](https://github.com/azya52/BrickEmuPy/issues/40) requests
a redump but provides no description, checksum, erroneous offsets or correction.
Neither establishes corruption of this particular image. The local 32-KiB banks
are not identical duplicate halves; zero/0xFF regions alone are not evidence of
bad dumping. **No confirmed corruption was found.** The earlier hatching symptom
is now explained by setup and does not justify patching the ROM.

Basic offline play is demonstrated with the original private firmware. The
public original-symbol renderer also passes the opt-in setup/hatching checks. This is not exhaustive lifetime/minigame validation or complete
public distribution clearance. Rights to the private exact-symbol pack remain unresolved; that pack is not
shipped. Public builds include original GPL replacement symbols and their sources.
IR communication and external/cascaded timer modes remain unsupported, and not
every icon/physical sound characteristic has been verified. Tama-Go stays isolated.
The hatching investigation changed no core behavior. Subsequent release
preparation added only original public symbol rendering; it did not alter CPU,
MCU, firmware, timing or save formats. The scoped release decision and separately tracked older questions are documented
in CONNECTION_V3_RELEASE_AUDIT.md; the prepared version is 0.9.0.
