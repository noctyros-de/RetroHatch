<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Tama-Go license and copyright audit

Audit date: 2026-11-12. Scope: the uncommitted Tama-Go integration on RetroHatch
0.8.0, not a new legal clearance of the entire emulator. This is an evidence-based
publication assessment, not a binding legal opinion. No emulator code, version,
ROM, figure, existing notice, reference checkout or Git history was changed.

## Publication decision update ? 13 November 2026

The owner explicitly authorized including Tama-Go in 0.10.0 despite the recorded
unspecified BSD variant. This supersedes the earlier operational hold below,
not the underlying findings: the BSD variant, original notice and intermediate
permission scope remain unresolved. No specific incompatible clause or prohibited
redistribution was established in this audit. Current attribution is centralized
in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md); do not describe this
owner decision as legal clearance. The following assessment is retained intact
as the dated historical record.

## Decision

**Do not publish the current Tama-Go-containing binary or source package yet.**
The main missing evidence is the exact license and required copyright notice
for the asterick/tamago implementation inherited by the translated CPU, bus and
EEPROM code. Its historical `BSD` declaration is real evidence of licensing
intent, but does not identify the BSD variant or its required notices. It is
not evidence that the author chose an incompatible license, nor proof that the
code is unlicensed. Permission sufficient for this distribution is unresolved.

The existing implementation can be kept intact while clarification is sought.
If a compatible grant covering the actual ancestors is confirmed, the least
disruptive resolution is to preserve the implementation and complete its notices.
There is no demonstrated need to rewrite the whole Tama-Go backend today.
There is also no basis to promise that adding an arbitrary BSD text will cure it.

Classification meanings:

- **Cleared:** adequate compatible-license/provenance evidence for the identified
  material, subject to the explicitly stated scope; not proof of every possible
  employment, patent, trademark or contractual right.
- **Unresolved:** additional provenance, a precise license or permission is needed.
- **Blocked:** do not distribute the affected deliverable under present evidence.
  Here the aggregate release is blocked by unresolved upstream permission; no
  specific incompatible BSD clause or adverse rights-holder claim was found.

## Evidence and historical origin chain

### 1. Original JavaScript implementation

Repository: [asterick/tamago](https://github.com/asterick/tamago).
Audited revision: [`905de5cff0226ce5ae88ac7b8164c3df0cd3d7cb`](https://github.com/asterick/tamago/commit/905de5cff0226ce5ae88ac7b8164c3df0cd3d7cb),
2013-12-02. Earliest reachable commit:
[`85eaefdcf8b84eeb15a379d016c534d330cd7f89`](https://github.com/asterick/tamago/commit/85eaefdcf8b84eeb15a379d016c534d330cd7f89),
2013-06-27.

All 85 reachable commit-author records examined name **Bryon Vandiver**;
associated GitHub identities are `asterick` or absent. This newly establishes a
publicly documented author name, not exclusive legal ownership or a verified
absence of employer/assignment rights. No other code author was identified in
that reachable history. No personal email address is needed for this audit.

Material predecessors of the RetroHatch core include:

- `src/tamago/cpu/6502.js`, `address.js`, `operations.js` and
  `src/tamago/data/instructions.js`: interpreter, addressing, operations and table.
- `src/tamago/cpu/tamagotchi.js`: banked memory, RAM sizes, clock and LCD row order.
- `src/tamago/cpu/registers.js`: GPIO and interrupt-register behavior.
- `src/tamago/cpu/eeprom.js`: power/START/STOP/ACK/read/write state machine.
- `src/tamago/main.js`: matrix extraction and electrical icon-state decoding.

The pinned [package.json](https://github.com/asterick/tamago/blob/905de5cff0226ce5ae88ac7b8164c3df0cd3d7cb/package.json#L19-L20)
contains an empty `author` field and `license: BSD`. The same BSD declaration is
present in all four historical versions returned for that path: the initial
commit, `294a43257beb7a3aec08738a31a40a68b1073e22`,
`3672edd6df9f491099108fa443e662e542aaa802` and the pinned revision.
No complete license/copyright-notice file was found in the initial or pinned
tree; queries for the reachable root `LICENSE`, `LICENSE.md` and `COPYING` paths
returned no history. The relevant current source headers provide no fuller grant.
This is a bounded historical search, not proof that no notice ever existed in
an unexamined branch, external distribution or private permission.

An early checked-in technical reference, `source/files/6502.html`, identifies
[e-tradition's 6502 instruction reference](http://e-tradition.net/bytes/6502/6502_instruction_set.html)
as its source. `source/files/GPLB5x.txt` contains early hardware research.
These help trace technical inputs; neither document was copied into RetroHatch.
No evidence establishes an older copied 6502 program behind this JavaScript.
That absence of evidence must not be turned into an authorship warranty.

### 2. Intermediate C ports and the named T7Emu project

The pinned [T7Emu repository](https://github.com/mameise/T7Emu/tree/5d8653a0ff0b1893e59e0e17433286709c9a787f)
has one reachable commit, dated 2026-09-28, containing only `Readme.md`.
It describes an Android APK and says source will be published after cleanup.
It provides neither the C implementation nor a license grant for it. No code
was copied into RetroHatch from that repository; no APK was downloaded for this
audit. A public APK is not a substitute for missing source/provenance evidence.

Paramix's `t7/README.md` explicitly describes its core as a C port of
asterick/tamago taken from a T7Emu app. Headers also mention an ESP32 compatibility
layer and a TamaGo-for-Pebble predecessor. These are documented intermediate
origins, but their exact source revisions and separate contributor grants could
not be established from the supplied sources.

The public `mameise/Tamagotchi-Emulator-Pebble` tree at
`2f462dad1834d0e3c75b1c0844e2c910eaf18ed7` was checked as a candidate. Its README
identifies a StefanBauwens/TamaLib **P1/P2** fork, not this Generalplus/V7 port.
Its TamaLib GPLv2 files are not evidence of the Tama-Go core's license. Do not
attribute that separate project to this backend or infer a GPLv2 conflict from
its existence. Paramix's idle-loop HLE/JIT and bridge implementation were not
ported; the retained STOP/key behavior still uses its documented reference.

**Unresolved:** ask mameise to identify the actual intermediate C/Pebble/ESP32
ancestors and contributors, and whether the shipped T7 code includes other
people's protected contributions. No public evidence verifies all rights behind
those transitions. No upstream author was contacted during this audit.

### 3. Paramix-Studio revision actually used

Repository: [mameise/Paramix-Studio](https://github.com/mameise/Paramix-Studio).
Pinned commit: [`675b904ab1c8a9086b977a82cbc47cd0c319187f`](https://github.com/mameise/Paramix-Studio/commit/675b904ab1c8a9086b977a82cbc47cd0c319187f),
identified by GitHub as Paramix Studio 1.7.4; its commit author is `mameise`.
No civil name or exclusive copyright ownership is inferred from that account.

The full [root LICENSE at that revision](https://github.com/mameise/Paramix-Studio/blob/675b904ab1c8a9086b977a82cbc47cd0c319187f/LICENSE)
is GNU GPL version 3. Its only recorded path-history change is its creation in
root commit `e61c989c160fe01d96ac5e0994e1dd9bf74a1d44` (2026-10-13).
T7 CPU/memory/sound source first appears in the reviewed path histories at
`323b43a6455f51092c27ce0fabe45b9ae30563ee` (2026-10-15). Thus the GPL file
predates, rather than postdates, this repository's T7 source import.

The retained local copy [LICENSES/Paramix-Studio-GPL-3.0.txt](../LICENSES/Paramix-Studio-GPL-3.0.txt)
is byte-identical to the recorded upstream LICENSE; SHA-256:
`8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903`.
All ten original source/license hashes in
[UPSTREAM.json](../app/src/core/tamago/UPSTREAM.json) were checked successfully.
Root GPL terms are compatible with choosing GPL version 3 for RetroHatch's
combined work, to the extent mameise can license the relevant contributions.
The generic GPL text alone does not establish an upstream 'only' versus
'or later' author election; RetroHatch's own election is GPL-3.0-only.
The Free Software Foundation copyright in that document covers the license
text, not the T7 implementation.

The C CPU and memory headers explicitly credit asterick/tamago; the EEPROM
header calls itself a faithful port. `t7/src/app/t7app.c` explicitly names
mameise. `t7/src/core/t7_sound.c` documents timer/DAC behavior reconstructed
from firmware, referring to a `docs/TON_FINDINGS.md` that is absent from the
inspected tree. Its provenance explanation is useful but cannot prove sole
authorship. The root GPL is adequate license-choice evidence for original
Paramix contributions, not a grant from every predecessor's rights holder.

## Actual RetroHatch file-by-file findings

Paths below are repository-relative. 'Original' means newly authored expression
for this integration, not independently discovered hardware or a clean-room
implementation. A translation remains a derivative even when every Rust line
was typed locally. File-level classifications distinguish the portions described;
the integrated binary still contains the unresolved core.

| Exact path / component | Origin and evidence | Assessment |
| --- | --- | --- |
| `app/src/core/tamago/cpu.rs` | Rust port of `t7/src/core/t7_cpu.c`; 151-opcode dispatch/table and addressing/ALU/stack/IRQ behavior inherit the explicitly credited JavaScript core. Retains distinctive reference decimal handling, not merely standard CPU facts. | **Unresolved** BSD terms/notice and intermediate rights; withhold this translated file from public distribution pending resolution. |
| `app/src/core/tamago/mod.rs`: `Machine` bus methods, `Eeprom::update`, GPIO, `vector`, `pixels`, `icons` | Ports/adapts `t7_memory.c`, `t7_eeprom.c`, `t7.c` and hardware headers; the EEPROM state transitions and LCD order trace back to the JavaScript files above. Rust ownership/bounds are local additions. | **Unresolved**, particularly CPU-bus/EEPROM inheritance. The mixed file cannot be classified as entirely independently written. |
| `app/src/core/tamago/mod.rs`: `spi_tx`, figure presence and chip-select logic | Adapted from Paramix's full-duplex SPI implementation in `t7_memory.c`; command 03, address bytes, FIFO and echo behavior preserve that design. `set_figure`'s absence interval follows `t7app.c::fig_choose`. | **Cleared** GPL compatibility for mameise's original contribution; **Unresolved** confirmation of intermediate authorship. It is also physically in the mixed unresolved file. |
| `app/src/core/tamago/mod.rs`: `recalc_timers`, `advance_cycles`, `render_audio` | Timer sources/16.16 scheduling and DAC behavior adapt `t7_sound.c`/`t7.c`; Rust uses its own integer sample integration/host output and does not copy upstream tone synthesis, filters, audio ring or waveform assets. | **Cleared** GPL terms for identified original Paramix portions; **Unresolved** rights chain where original audio provenance is not independently confirmed. Not a port of Bandai music data. |
| `app/src/core/tamago/figure.rs` | RetroHatch-authored parser, validation, naming and original-source SHA identity; sizes/headers/trailing-byte handling came from local inspection and the reference protocol. Character names identify accessories; there are no character bitmaps. | **Cleared** for identified original code and factual metadata. This does not license figure firmware or certify lawful acquisition of a dump. |
| `app/src/core/tamago/state.rs` | Original explicit RetroHatch schema 5, serialization, integrity checks and fresh-machine restoration; no C state struct dump is copied. Encodes ported machine fields and may include user figure contents at runtime. | **Cleared** original serializer; aggregate core remains on hold. Generated saves are private, not automatically GPL-licensed firmware redistributions. |
| `app/src/host/figures.rs` | Original private content-addressed storage, bounded reads, duplicate/repair handling and listing built on existing RetroHatch host services. | **Cleared** for this original host code; it must not ship imported files or user paths. |
| `app/src/host/emulator.rs`, `runner.rs`, `library.rs`, `fingerprints.rs`, `mod.rs`; `app/src/core/mod.rs`, `profile.rs`, `state.rs` | Original dispatch/worker/library/profile/save-ID integration. The exact SHA fingerprint and profile metadata are factual identifiers, not firmware bytes. | **Cleared** for these original integration changes. Referencing an unresolved backend does not clear the resulting combined executable. Existing shared components retain their prior notices. |
| `app/src/ui.rs`, `app/src/ui/help.rs`, `app/src/ui/help/tests.rs`, `app/src/ui/recording.rs`, `app/src/ui/screenshots.rs`, `app/src/lcd_render.rs` | Original figure controls, experimental category, framebuffer size routing and existing egui capture/scaling integration. Tama-Go uses firmware matrix pixels and text for fixed symbol states. | **Cleared** for identified original Tama-Go additions. This is not a re-clearance of older classic-LCD assets in the same application. |
| `app/tools/tamago_reference_host.c` | Substantially directly copied from `t7/host/hcompat.c`, with a new provenance header and include path adjustment. It is **not** wholly original RetroHatch code. | **Cleared** under the pinned root GPL for identified Paramix host glue; preserve upstream credit/full GPL and confirm predecessor claims if mameise identifies any. |
| `app/tools/tamago_reference.c` | RetroHatch-authored test driver/scheduler calling external T7 C APIs; scheduling is reference-adapted, not proof of independently designed emulation. It embeds no upstream CPU body. | **Cleared** original test glue under project GPL; an actual oracle build uses external source subject to its own unresolved chain. |
| `app/tools/compare_tamago.py`, `app/examples/tamago_probe.rs` | RetroHatch-authored validation tooling; setup input sequences adapt `t7/host/setup.txt`, not firmware bytes. The Python tool adapts temporary external copies for MSVC. The external implementation itself is not embedded in these files. | **Cleared** for identified original tools; never package generated real-firmware checkpoints, captures or saves as source fixtures. |
| `app/src/core/tamago/UPSTREAM.json`, `LICENSES/Paramix-Studio-GPL-3.0.txt`, `developer/TAMAGO.md`, Tama-Go notices in README/THIRD_PARTY_LICENSES.md | Source provenance, preserved GPL text and original documentation. These record the issue but cannot confer missing rights. | **Cleared** as evidence/documentation, not a legal cure. |

## Assets, hardware facts and firmware

No Tama-Go SVG, original fixed-icon artwork, Font Awesome file, shell image,
firmware sprite table or audio waveform asset is incorporated by this integration.
The ten icon intensities are extracted from LCD RAM; the UI substitutes ordinary
text labels. Matrix graphics and figure-specific graphics/sounds are produced
from the user's runtime firmware. This observation is specific to Tama-Go, not
an assurance about older BrickEmuPy-derived classic icon resources.

The historical upstream tree lists main/figure `.bin` files. Their mere presence
there does not license Bandai firmware. No such blob was downloaded or bundled
in this audit. Firmware, full/short figure files, real-game captures and saves
remain outside the public repository. Runtime schema 5 saves embed figure data;
those save files require the same privacy/publication exclusion as ROMs.
Bandai and any other actual content rights holders retain their rights; individual
asset authors or rights assignments were not established and are not invented.

CPU instructions, register addresses, pin behavior, timings and LCD wiring are
functional facts distinct from source expression. However, copying an interpreter,
state-machine organization or expressive table is not automatically a new,
independent implementation just because its subject is hardware. EU Directive
2009/24/EC distinguishes underlying ideas/principles from protected program
expression and treats translations/adaptations as restricted acts subject to its
exceptions. This is a framework, not a jurisdiction-independent determination
that every constant or reverse-engineering activity is cleared.

## Why 'BSD' remains insufficient

[The FSF's BSD discussion](https://www.gnu.org/licenses/bsd.en.html) and
[license comparison](https://www.gnu.org/licenses/license-list.html) distinguish
permissive revised/simplified BSD forms from the original advertising-clause
BSD. The former are generally GPL-compatible; the latter is not. No evidence
here establishes that asterick chose the incompatible original form. Equally,
no evidence justifies silently choosing BSD-2-Clause, BSD-3-Clause or 0BSD on the
author's behalf. A missing full notice is not cured by guessing a familiar one.

A later project's GPL notice can license its author's additions, but cannot
supply rights or remove conditions belonging to earlier authors. Likewise,
RetroHatch's SPDX headers, successful compilation, correct firmware comparison
and Cargo license checks do not resolve this source ancestry. GitHub's
[licensing guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)
distinguishes public viewing/forking from a general reuse license. Public
availability alone is not the missing permission.

## Least-disruptive resolution plan

1. **Ask Bryon Vandiver / asterick for the precise grant.** Identify the pinned
   commit and the CPU/addressing/operations/table, bus/GPIO, EEPROM and LCD code
   listed above. Request either the intended complete compatible BSD license and
   approved copyright notice, or an explicit grant allowing redistribution and
   translation of the relevant work under GPL-3.0-only or compatible terms.
   Ask whether any of those parts were adapted from others and under which terms.
   Archive a dated public clarification or verifiable written permission, with
   exact covered revisions/files. A current license change must clearly cover
   the historical code being used; do not assume it does automatically.
2. **Ask mameise to document the intermediate chain.** Obtain names/revisions of
   the Android/Pebble/ESP32 predecessors, contributor credits and their grants;
   confirm the root GPL applies to the T7 code, SPI figure work, timer/audio and
   `hcompat.c`, to the extent owned. An assurance from mameise alone cannot grant
   rights in Bryon's or another contributor's work without authority to do so.
3. **Complete notices only after the scope is established.** Preserve the full
   GPL, exact confirmed BSD text/copyright and modification/provenance notices.
   Expand COPYRIGHT/THIRD_PARTY_LICENSES and file-level credits accurately; add
   relevant modification dates. Do not claim the entire port as exclusively
   Noctyros-authored. Include the notices with binary and corresponding-source
   archives. No notice changes were performed by this audit.
4. **If permission cannot be established, replace only affected material.** A
   verified appropriately licensed 6502 core is one option, but replacing the
   CPU alone does not resolve the inherited EEPROM/bus portions of `mod.rs`.
   Alternatively commission a genuinely independent implementation from lawful
   CPU/device specifications and documented black-box behavior, with a provenance
   record. Merely rewriting the same source with new variable names is not a
   clean-room cure. The existing author has seen the reference; separate factual
   specifications and an unexposed implementer are stronger evidence. Check any
   replacement's own license, specification rights and acquisition conditions.
5. **If an earlier publication is needed, prepare a Tama-Go-free deliverable.**
   That needs a deliberate build/source-packaging change outside this audit.
   Hiding its menu or disabling runtime selection is insufficient while the
   unresolved code remains in the distributed source or binary. Do not rewrite
   history, delete working files or alter releases automatically.

## Distribution obligations once the chain is resolved

Under [GPLv3 sections 4-6](https://www.gnu.org/licenses/gpl-3.0.html), retain
applicable copyright/license/warranty notices, identify modifications and their
relevant dates, and distribute the combined work on GPLv3 terms. When shipping
Windows object code, supply exact matching Corresponding Source through a valid
section 6 method; for the existing release process, offer the matching buildable
source archive alongside the executable, including necessary build scripts and
non-System dependencies. Do not impose additional restrictions on recipients.
A license text without corresponding source does not meet binary obligations.

For a confirmed compatible BSD predecessor, preserve the actual required notice
and disclaimer in source and binary documentation as its clauses require.
GPL compatibility does not erase that attribution. Do not add an advertising
notice or remove a clause by guessing. Do not bundle firmware to satisfy the GPL:
independent proprietary game/figure images are not newly authored source for
this emulator and RetroHatch has no grant to redistribute them.

## Scope outside Tama-Go and audit limitations

This newly identified source-chain issue is specific to the new Tama-Go port
and combined builds containing it. Read-only inspection of local `v0.8.0` finds
no `app/src/core/tamago/`, `app/src/host/figures.rs` or
`app/tools/tamago_reference_host.c` in that tag. This audit provides no reason
to retroactively label those unchanged earlier release contents as including
Tama-Go. Other backends' existing icon, S1C33/SLEIGH or upstream-notice issues
remain separate; they were not newly audited or declared cleared here.

The source can remain locally for remediation; that is not a blanket opinion
on every jurisdiction's private-use or reverse-engineering exceptions. Commit
author names do not establish assignments, employment contracts or permission
for every pre-existing snippet. Reachable public histories do not expose
private or deleted histories. The missing intermediate port and exact BSD
variant remain gaps, not invented violations. Legal review may be needed before
publication if the rights holders cannot resolve those facts.

No functionality, firmware, figure, version or existing project file was changed.
Within the public project, only this report was created; source/history evidence
was stored separately in the private local audit directory. No tests/builds were run because no program change was
made. No upstream messages, commits, pushes, tags or releases were created.
