# Third-party notices

Original RetroHatch contributions are GPL-3.0-only; see ../LICENSE and
../COPYRIGHT.md. That declaration does not replace upstream rights or notices.

The E0C6200 instruction/MCU/I/O/timer/buzzer behavior and technical device/display
data are adapted from azya52/BrickEmuPy, which declares CC0-1.0. The complete
unchanged license remains in LICENSES/BrickEmuPy-CC0-1.0.txt.

Display-only resources cover P1/P2 (the reference explicitly shares one SVG),
Genjintch, Angel, Morino and Umino. Yasashii adds only its 512 matrix coordinates
and electrical mapping; its six GPIO LEDs use original host UI indicators, not
upstream artwork. No shell or decorative background is embedded.
The independent provenance and preferred editable source of derived icon
silhouettes remain unresolved review items. No exclusive project
authorship or fabricated GPL license is claimed for them.

The On/Meets ARM library is incorporated from oreo-fox/tg18-emu under MIT,
Copyright (c) 2026 oreo-fox. Its complete license is in
third_party/tg18-core/LICENSE and ../LICENSES/TG18-Emu-MIT.txt. MIT integration
changes in that library retain separate attribution; the host adapter is GPL-3.0-only.
No upstream desktop frontend, logo, screenshots or ROM data are included.

The S1C33 CPU/memory/LCD/peripheral Rust translation derives from maragotchi/tamaemu
(original author: maragotchi), which declares GNU GPL-3.0. The exact upstream
text is retained in LICENSES/TamaEmu-GPL-3.0.txt. Its contradictory README
no-charge wording and the credited SLEIGH provenance remain release-review
issues; see ../THIRD_PARTY_LICENSES.md. Only the locally verified P's profile is exposed; other S1C33 boards remain unverified.

TamaLIB/TamaTool, MAME, Epson
and Paramix Studio supplied research context or factual corroboration; no
implementation or icon set from those research-only sources is incorporated.
No copyrighted firmware is included; stored test programs are synthetic.

The complete public attribution, actual Cargo/font/runtime obligations and
release limitations are in ../THIRD_PARTY_LICENSES.md and ../LICENSES/.
Future binary packages must include those documents and full notices alongside
the project GPL text. This local build is not a public-distribution clearance.

The six Japanese Meets editions reuse the same MIT TG18 backend. RetroHatch
preserves serialized timer/IRQ counters on restore instead of recomputing them;
this integration correction is recorded in third_party/tg18-core/UPSTREAM.json.
