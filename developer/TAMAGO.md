# Tama-Go backend and virtual figures

Status: experimental integration for 0.10.0, recovered from the preserved local
Tama-Go branch onto the published 0.9.0 baseline. Existing Connection V3 and
older backend/save formats are retained. Current attribution is centralized in
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

## Architecture and reference scope

`app/src/core/tamago/` is a separate native Rust GPLB52320A backend. It never
executes this firmware through E0C6200, S1C33 or TG18. `cpu.rs` ports all 151
nonreserved opcodes and the fixed reference cycle table, including its decimal
flag behavior and lack of standard-6502 page-crossing cycle penalties. Unknown
instructions return errors rather than silently executing a NOP. `mod.rs`
implements the bus/peripherals; `figure.rs` validates accessories; `state.rs`
implements the append-only profile code 25 and backend schema 5.

The behavioral reference is Paramix-Studio T7Emu at
`675b904ab1c8a9086b977a82cbc47cd0c319187f`, credited to mameise and derived from
asterick/tamago. See `UPSTREAM.json` and `THIRD_PARTY_LICENSES.md`. No reference
repository was modified. Only emulator source/license files were downloaded;
no firmware, upstream images or other binary assets were fetched.

- CPU: 4,000,000 emulated cycles per second, independent of UI repaint.
- Work RAM: 1,536 bytes mirrored through $0000-$0FFF.
- LCD RAM: 512 bytes mirrored through $1000-$2FFF.
- I/O: 256 registers mirrored through $3000-$3FFF.
- Banked ROM: twenty 32-KiB banks at $4000-$BFFF, selected through $3000;
  the first 16 KiB is also mapped at $C000-$FFFF.
- IRQ vectors: sixteen priority entries from main-ROM offsets $3FC0-$3FDF.
- LCD: controller geometry 64x32, but the verified visible matrix is **48x31**
  using the reference's non-linear row order and two bits per pixel. It is not
  correct to stretch or fabricate 64 independent visible columns.
- LCD interrupt 10 and enabled NMI 6 occur every 66,666 cycles, matching the
  reference's integer 60-Hz divider; independent interrupt 13 runs at 2 Hz.
- Timer 0 source: 8 MHz; timer 1: 32,768 or 524,288 Hz as selected by $3030.
  Period/overflow handling follows the upstream 16.16 scheduler.
- STOP/wakeup and the reference's one-second key-activity safeguard preserve
  press/release behavior. No Paradise clock-poking, 3.03-MHz speed adjustment,
  framebuffer skipping, firmware patch, HLE idle shortcut or JIT is used.
- EEPROM: full 4-KiB bit-banged state machine on port B bits 2/1/0.
- DAC: register $3060 with amplifier control $3065; deterministic area-integrated
  44,100-Hz mono PCM is fed into the existing volume/mute/WinMM output path.
  The upstream optional synthetic tone replacement and presentation filters
  are not copied. Nonzero PCM and native output are verified; physical sound
  fidelity, spectral comparison and listening quality remain unverified.

## Main firmware and accessory classification

Names do not identify firmware. The verified main image is exactly 655,360
bytes and has reset vector $CC00 at offset $3FFC. Its full SHA-256 is in the
normal exact fingerprint catalog. Unknown same-structure images still follow
normal explicit association rules. Main firmware is never overwritten.

All fifteen supplied accessory files have AA 55 headers at offsets 1 and 17;
the secondary header carries the identity byte at offset 20. The file named
`Tama-Go_Debug.bin` is a 512-KiB **debug accessory** (ID FE), not a separate
boot ROM. It is refused by the ordinary standalone-ROM import path.

| Local file | Bytes | SHA-256 |
| --- | ---: | --- |
| `tamago.bin` | 655,360 | `197d9aa7c5f0bea21b2f6f8d1ce53de5fee09cfba708731b5380c68dee255444` |
| `Tama-Go_Debug.bin` | 524,288 | `dea4a9d26d1a45943d4791a5e00f372f7c25fb02ae20771b9c31a6217f54368e` |
| `(03)_Tama-Go_Mametchi.bin` | 349,536 | `2de8479ad6c0e518477dd36f3cc23bfe1fb20878adc3370378cbee40380a1e1b` |
| `(04)_Tama-Go_Memetchi.bin` | 349,536 | `7482e312263d4ac9cb05327986832af05ae9906e75cb99324dd75739c06d4115` |
| `(05)_Tama-Go_Kuchipatchi.bin` | 349,536 | `b398a3d646950b5a802fdd40c13547a46eed442be70e8935f62ac9a80eced6ca` |
| `(06)_Tama-Go_Chamametchi.bin` | 349,536 | `eb180f5842f5cd1358a33b3a18688a58b5c67414cdfb8a54ae16d17496c55198` |
| `(07)_Tama-Go_Violetchi.bin` | 349,536 | `d0b3088923fa2c1c99302e7ec34defa0f52fe6ee27bab9c41b2a1c67817c2c0d` |
| `(08)_Tama-Go_Ringotchi.bin` | 349,536 | `41c1be83fc42279d808461012165eb24a0690dcba228dbb8479708d4c717f2d8` |
| `Tama-Go_Chamametchi_Figure.bin` | 4,194,312 | `3da03a64db45643aeafead62ae1fd40ff0bae37e5e398fd0bd534baad2c92d38` |
| `Tama-Go_Mametchi_Figure.bin` | 4,194,312 | `4b10a1fb16d971ce09feb6e2a350dcc9b5c0ba19c15b4c4f24951630d12bbd01` |
| `(09)_Tama-Go_Kuromametchi.bin` | 524,288 | `9ea5f22564ba75b0755d2071b6e6240539da1bf7c5c1b67aff6f5cdbdb495258` |
| `(10)_Tama-Go_Sebiretchi.bin` | 524,288 | `ebb7f59b7e061d419b09056cd5530c7ab096fc20a948a5ae84c49a4f7f13484f` |
| `(11)_Tama-Go_Ponytchi.bin` | 524,288 | `ce78d5a7f3944c5e2b38faabc17603f7697b8d3d0e3fcff066d1e864794ad30e` |
| `(12)_Tama-Go_Ichigotchi.bin` | 524,288 | `3e751a8914c7309ef9f03a6cc32c57aea2a222e193d7a276e901c72b32aa4acc` |
| `(13)_Tama-Go_Shimashimatchi.bin` | 524,288 | `86abe80e06985573510b1bfbaecd6377252bd8f1a66213a7b9174dd2120a61fc` |
| `(14)_Tama-Go_Makiko.bin` | 524,288 | `99bfbe52b3f99dc5001397919778c2c8681a15227569695f9a87b13617ae29e5` |

The six wave-one short captures contain 349,536 bytes, the six wave-two images
contain 524,288 bytes, and the two full captures contain 4,194,312 bytes.
Full captures have the same byte-zero prefix as their corresponding short
images, followed by additional nonempty flash data. The extra eight bytes are
**trailing capture bytes**, not a prefix to strip. Preserve their original file
and identity; expose the first 4 MiB as flash. No figure is decoded as executable
main firmware. A structurally consistent unknown revision uses its header ID
as the accessory name, not a claim of independently verified dump identity.

## Virtual figure slot

Use **Emulation > Tama-Go Figure > Import Figure...**, then choose a stored
figure or **No Figure**. The menu exists only while Tama-Go is loaded. Imported
files are copied by content hash into `tamago-figures/` under the existing private
user-data root, never moved or added to Git. Source hashes are verified again
when selecting an internal copy. An explicit reimport from a validated original repairs a corrupted internal copy.
Duplicate content shares one file; regular
and full captures remain distinct. Figure selection never resets the device,
erases EEPROM or changes the firmware file.

Port A bits 5/6 report presence with the reference power/direction masks. Port B
bit 4 is active-low chip select. Writes to $30B3 implement full-duplex command 03
plus a 24-bit address; $30B6 dequeues RX bytes, including the four command/address
responses, and $30B7 exposes the ready bit. Data advances only on SPI transfers,
not by copying game data directly into RAM or unlocking unrelated content.
Out-of-range accessory reads return zero as in T7Emu; bytes missing from a short
capture cannot be recovered. Switching exposes an absent figure for 2.5 seconds
of emulated time, matching the upstream host's safe removal/insertion interval.

## Saves and existing frontend

Host dispatch shares the existing ROM library, worker/pacer, pause/reset,
three manual slots, independent autosave, save-on-exit and import/export.
Left/Down/Right remain A/B/C; Up has no Tama-Go sensor effect. The native RGB
presentation preserves four grayscale levels, 48x31 aspect and nearest-neighbor
scaling. LCD PNG capture is exactly 48x31; existing full-window capture and video
paths accept this non-square framebuffer. They do not include fabricated icons.

Schema 5 uses the RTHSTATE integrity envelope and main firmware SHA-256. It
serializes CPU, all RAM/registers, bank, input/STOP state, time/divider phases,
IRQ state, timer deadlines, EEPROM data plus its in-flight bus state, complete
SPI FIFO/command/address, DAC/sample integration state and pending PCM. The
selected figure bytes/source identity and pending insertion deadline are saved
as well, so resume is independent of whether an accessory source was moved.
These states are private and may contain copyrighted figure data. Do not share
or package them. Restore parses a fresh machine, bounds its fields and checks
the checksum/ROM/schema before replacing the running instance. Existing backend
schemas/profile codes and saved games remain unchanged.

## Validation performed

- All 151 defined opcodes execute synthetic cases; reserved encodings reject
  atomically. Focused bus, display, timer, SPI and state tests pass.
- The genuine main firmware boots, accepts setup inputs and hatches into a
  visible pet. Status, food, connection, figure/shop and discipline/menu screens
  were captured privately. The ten electrical icon states change with menus.
- Thirty actual-firmware one-second checkpoints match the C reference CPU,
  complete work RAM, LCD RAM and I/O exactly. The C driver supplies the same
  deterministic host scheduling; this is not a physical timing certification
  or an independent verification of the shared scheduler design.
- Every regular figure (Mametchi, Memetchi, Kuchipatchi, Chamametchi, Violetchi,
  Ringotchi, Kuromametchi, Sebiretchi, Ponytchi, Ichigotchi, Shimashimatchi, Makiko)
  and both full captures passes actual firmware SPI reads and content/menu
  navigation. Distinct figure-specific characters/buildings are visible.
  The debug accessory is read (12 bytes); its diagnostic activation is not
  verified. A filename/import success is not counted as a gameplay result.
- All fifteen accessory runs continue for 120 emulated seconds plus menu
  branches. Each of ten menu snapshots is restored into a fresh machine and
  compared after another second: exact full-state continuation.
- An actual running Makiko-to-Chamametchi full-dump swap performed 9,430 SPI
  data reads, retained the running game and did not erase EEPROM at either
  insertion/removal boundary. A 2.5-second absence separates insertions.
- Synthetic worker tests exercise all three slots, import/export, figure removal,
  restore and a new worker's automatic resume. The actual Windows EXE opens the
  user firmware through the private library, restores the selected figure,
  accepts arrow-key input, creates valid LCD/full PNGs, remains running while
  minimized, saves on close and resumes after a second native launch.
- Integrated native-audio worker measurement: **60.105824 seconds wall time,
  60.103026 seconds emulated, 99.9953% average real-time speed**. No artificial
  game-clock acceleration or UI-driven timer is applied.

Private captures, traces, comparisons and firmware inventory remain in the
workspace's private test-data area. The opt-in `app/examples/tamago_probe.rs`
and `app/tools/compare_tamago.py` reproduce targeted checks with supplied files.
No firmware or ROM-derived screenshot was added to this repository.

## Remaining technical limitations

1. Exact original geometry for ten fixed LCD symbols is not established by the
   inspected references. T7Emu/asterick use substitute UI artwork; RetroHatch
   deliberately displays electrically driven text labels instead. It must not
   be advertised as pixel-exact reproduction of those fixed symbols.
2. Figure content/menu access is verified, but every complete minigame, shop
   purchase, long-term progress path and extra game beyond short captures is
   not. Full flash is addressable; this is not proof every extra game works.
3. Debug-cartridge mode is not confirmed. IR/device links and TamaTown online
   interaction are outside this implementation. No fake connection success is
   injected. Full device lifetime, battery behavior and offline catch-up are
   not qualified.
4. Sound fidelity is not physically compared. Video still has no game audio.

Keep the feature experimental until the remaining technical qualifications are
completed. Licensing evidence and the separate owner publication decision remain
in `TAMAGO_LICENSE_AUDIT.md`; the unspecified BSD variant is not resolved.

## Earlier experimental-branch checks

`cargo fmt --check`, `cargo check --locked`, the complete `cargo test --locked`
suite (220 passed, one deliberately ignored interactive-browser test), the
existing Python CPU/MCU/device/S1C33/audio comparisons, 25 Python tooling tests,
and the new Tama-Go C differential check passed. The locked Windows dependency
notice check verified 106 packages. The optimized Windows release build succeeded
through the existing static-runtime/path-remapping build script; no ZIP/release
was published. This dependency-notice success does not resolve the separate
upstream BSD-variant uncertainty.

Native window/LCD/control layout and generated PNGs were inspected. The egui
test exercises the real figure submenu and No Figure action. A later attempt to
inspect its native popup using physical mouse input was stopped because Windows
refused foreground activation; no mouse click was sent after that check failed.
Further manual popup/long-session testing is still appropriate.

## 0.10.0 integrated release validation ? 13 November 2026

Recovered the preserved experiment onto 0.9.0 without changing the V3 CPU/LCD
assets or save schema. Tama-Go remains experimental and retains its existing
schema 5/profile 25; V3 retains schema 6/profile 26 and its separate EEPROM.

- Actual unchanged main firmware: 30 CPU/RAM/LCD/I/O checkpoints exactly match
  the pinned T7Emu C oracle. Cold native Windows launch displays the real LCD.
- All 15 available accessory files rerun through 120-second setup/gameplay and
  ten menu branches each, with byte-exact one-second save continuation. Twelve
  ordinary figures and two full captures have actual SPI/content/menu access;
  the debug accessory is read but diagnostic activation remains unverified.
- Makiko to full Chamametchi live switch: 9,430 SPI reads, EEPROM never erased,
  no reset. Private original firmware and all accessory hashes remain unchanged.
- Real worker with Windows audio: 60.084670 wall seconds / 60.076262 emulated
  seconds = 99.9860% average speed. No clock scaling or timing changes were made.
- Native Windows release launched cold, from a real figure-menu state, and again
  via last-ROM resume. Saved time advanced through 9.074654, 161.709558 and
  170.692793 seconds; state identity and checksums remained valid. Windows were
  captured and inspected with LCD/buttons/footer visible and no clipping.
  Posted native key events do not independently establish physical-key response;
  actual firmware input sequences and existing egui-to-worker tests cover input.
- Existing Connection V3 valid setup, EEPROM, hatch display transition, exact
  60-second game clock and five-second save-continuation regression passed.
- All existing E0C6200 MCU/device, S1C33, buzzer PCM and SPLB32 Python comparison
  checks passed. The licensing/packaging/release tooling suite passed 26 tests;
  the synchronized inventory contains 106 Windows dependency packages.

Private test evidence is under `local/test-data/tamago-v0100/`, never packaged.
The focused tests do not establish exhaustive figure content, debug mode,
physical buzzer fidelity, IR or lifetime compatibility. Text labels remain a
substitute for unestablished original fixed-icon silhouettes.
