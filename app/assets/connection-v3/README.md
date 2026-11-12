<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Original Connection V3 replacement symbols

These ten editable SVG drawings were created for RetroHatch from generic
functional concepts. They do not copy or trace BrickEmuPy's fixed-symbol paths,
Paramix/T7 bitmaps, Bandai silhouettes, character art or firmware content.
They are original replacement designs, **not authentic original LCD artwork**.
Original RetroHatch contributions: Copyright (C) 2026 Noctyros, GPL-3.0-only;
see the complete repository-root LICENSE. There is no extra third-party license
or asset dependency for these drawings.

In electrical/display order: health, food, clean/toilet, games, connection,
discipline, medicine, lights, friends, attention. The real firmware controls
activation through the unchanged segment addresses; the connection symbol does
not imply implemented IR communication. Technical segment mapping comes from
the separately attributed CC0 BrickEmuPy reference. Matrix pixels are unchanged.

The SVG files are the preferred editable sources. `icons.alpha` is generated
40x40 alpha coverage, ten consecutive 1600-byte tiles in the order above.
Regenerate with `python -B app/tools/generate_connection_v3_icons.py`; verify
with `--check`. Cargo embeds the checked-in coverage and requires no Python,
Qt, private resources or reference checkout. No private `.rhlcd` is packaged.
