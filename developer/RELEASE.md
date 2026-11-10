<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Release and redistribution checklist

The repository remains private. Local build and packaging commands never push,
publish or change repository visibility. The tag-triggered workflow described
below **does publish a GitHub Release** when an authorized maintainer pushes a
matching `v*` version tag. Resolve the known LCD artwork provenance and
preferred-source gaps before authorizing distribution; automation is not legal
clearance.

## Gates before any public source or binary distribution

- Resolve rights to every derived LCD symbol, or replace affected material with
  verified original/permitted assets through a separate functional change. Keep
  the preferred editable source and its exact license available with the result.
- Review [THIRD_PARTY_LICENSES.md](../THIRD_PARTY_LICENSES.md), every mandatory
  compound font/data license and the complete Cargo/toolchain notices. Fonts
  keep their own licenses; do not label them exclusively GPL-3.0-only.
- Inspect the exact intended Git index, branch/tag history and other refs.
  .gitignore does not remove old tracked material. Do not mirror private tool
  refs or publish a `.git` directory. Any cleanup needs separate owner approval;
  no cleanup or history rewrite is performed by this preparation.
- Verify all required source, synthetic fixtures, Cargo.toml, Cargo.lock, public
  docs and notices are in the intended source snapshot. Local uncommitted files
  are not automatically present in an archive of HEAD.
- Build/check/test a clean source snapshot without `references/`, `docs/`, ROMs,
  saves or user configuration. Record the exact source revision, toolchain,
  target, features, build command and artifact hashes. Re-run the license
  inventory when Cargo.lock, features, target or toolchain changes.
- Obtain the owner's release authorization after reviewing the final artifacts
  and unresolved-risk report, before pushing the version tag. A tag push starts
  publication automatically; local packaging does not.

The staged tamaemu translation adds a separate release gate: clarify the
upstream README's no-charge instruction against its GPLv3 declaration, and
review the CPU's credited SLEIGH derivation. Retain
`app/LICENSES/TamaEmu-GPL-3.0.txt` in source/binary packages. Passing synthetic
S1C33 tests must never be presented as verified support for the target devices.

## GPL-3.0-only binaries and matching source

GPL version 3 permits commercial redistribution; do not add a noncommercial or
no-fork restriction. Preserve notices, the license and warranty disclaimer;
mark changes appropriately. When conveying object code, satisfy an applicable
GPLv3 section-6 method for its complete Corresponding Source. For a normal web
download, provide equivalent access to the exact matching source at the same
place without an additional charge, with build scripts/configuration and any
required installation information. A private GitHub URL inaccessible to binary
recipients, a link to a different revision, or Cargo.lock alone is insufficient.
If a source offer is used instead, review its precise applicability and duration
rather than using a generic promise. No source offer is fabricated here.

See the [official GNU license](https://www.gnu.org/licenses/gpl-3.0.html) and
[GNU FAQ](https://www.gnu.org/licenses/gpl-faq.html). GPLv3 does not require
making a privately used checkout public merely because it is modified. This
does not remove obligations if covered binaries/source are actually conveyed.
Permissive dependencies and separately licensed fonts retain their own terms.
Review font embedding/source conditions, system-library exclusions and any
compiler/runtime redistributables for the actual distribution method.

## Local package contents

Keep any staging directory private and clearly marked as a non-release draft
until the gates above are resolved. A future Windows binary package must carry:

```text
RetroHatch.exe
LICENSE                         complete, unmodified GPL version 3
COPYRIGHT.md
THIRD_PARTY_LICENSES.md
LICENSES/                       complete Cargo, font and runtime notices
app/LICENSES/BrickEmuPy-CC0-1.0.txt
README.md
SOURCE.md                       exact matching source/build information
```

The corresponding source package must additionally contain all required app
source, generated-data preferred sources once cleared, fixtures, manifests,
lockfile, tools and build instructions. Include third-party source required by
the selected GPL method; retaining exact vendored crate sources is a robust
option when preparing a final source package. No vendoring or downloading of
firmware is authorized. Standard Windows system DLLs and build tools should not
be copied from a personal installation into a package; review any separately
needed redistributable's own terms.

Refresh the Windows inventory from a built/fetched locked checkout:

```powershell
python -B app/tools/audit_licenses.py
python -B app/tools/audit_licenses.py --check
```

This reads cached dependency sources and writes only public notice evidence.
It is not a legal approval, security vulnerability scan or all-platform audit.
GitHub settings and repository visibility are never changed by these tools.


## Build-path privacy

A default Rust release build can embed absolute source/dependency paths in panic
metadata even without PDB files. The preparation audit found personal build-home
prefixes in the ordinary executable. For a future distribution candidate, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File app/tools/build_release.ps1
```

This build wrapper statically links the Microsoft C runtime and uses dynamically
discovered path-prefix remapping for the user home, project and Rust toolchain. It restores the process environment
after building, respects an existing CARGO_TARGET_DIR, and never uploads or
authorizes distribution. Custom flags use CARGO_ENCODED_RUSTFLAGS to avoid
whitespace/quoting errors. Re-scan the actual binary for personal paths after
changing the toolchain or build location. Remapping addresses a privacy issue,
not the outstanding asset/licensing release gates. See the
[rustc path-remapping documentation](https://doc.rust-lang.org/rustc/command-line-arguments.html#--remap-path-prefix-remap-source-names-in-output).

## Windows tag workflow

`.github/workflows/windows-release.yml` runs only when a `v*` tag is pushed.
The tag must equal `v` plus the current version in `app/Cargo.toml`.
It uses Windows Server 2025, the audited Rust 1.99.1 MSVC toolchain, formatting,
locked check/test/build commands, license-inventory checks and packaging tests.
The actual build command is `cargo build --release --locked` from `app/`, with
the wrapper's path-remapping and static-C-runtime flags configured first.

The packager uses an explicit source allowlist, creates a matching source ZIP
with `cargo vendor --locked` dependencies and an offline Cargo configuration,
and checks that the Windows EXE has no non-system DLL imports. An independent
offline release build of the extracted source must succeed before publication.
The binary ZIP includes the EXE, README, GPL text, copyright and third-party
notices, public developer guides and `SOURCE.md` with the exact revision and
build information. Fonts and LCD masks are embedded in the executable; external
ROMs, settings and saves are not needed to launch it and are never packaged.

The final step uses the runner's GitHub CLI with the workflow token's
`contents: write` permission. It verifies that the tag already exists, uploads
both ZIPs and `SHA256SUMS.txt`, then publishes the release. A version containing
`-` is marked as a prerelease. Published releases are not overwritten. If an
upload fails, inspect any remaining draft before retrying the workflow. No
personal access token is required. Actions must be enabled and repository or
organization policy must permit the workflow token to write release contents.

Private repository releases remain access-controlled. This workflow neither
makes the repository public nor grants access to additional people. Do not push
a release tag until the rights/provenance gates above are resolved. There is no
separate approval prompt after a matching tag has been pushed.

## Local package verification

From the root, after building and committing the intended source in your normal
maintainer workflow:

```powershell
python -B app/tools/test_package_release.py
$metadata = cargo metadata --manifest-path app/Cargo.toml --no-deps --format-version 1 --locked | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'retrohatch').version
python -B app/tools/package_release.py --tag "v$version" --output dist
```

Packaging normally refuses tracked changes or untracked publication files.
For an uncommitted local verification only, add `--allow-dirty`; the manifest
then explicitly labels the result as a local draft, not an exact tagged build.
`--offline` uses only cached crates. Existing output archives are never replaced.
The scripts do not create tags, commits or releases. They do not read original
ROMs, AppData libraries, private references or saves. Review the resulting ZIPs
and hashes; do not upload the temporary staging directory or `.git` metadata.

Extract the Windows ZIP to a separate directory and launch `RetroHatch.exe`
with no source checkout beside it. Use a temporary `RETROHATCH_DATA_DIR` during
smoke tests to avoid touching personal libraries and saves. Test an actual
Windows desktop with a suitable OpenGL driver; hosted CI cannot establish every
GPU, clean-machine or gameplay compatibility claim. The portable package does
not require Rust, Visual Studio or a separate MSVC runtime installer. SmartScreen
may warn about unsigned downloads; these builds are not code-signed.

## Central version and developer attribution

The creator is Finn Lülsdorf and the developer alias is Noctyros. Existing
GitHub URLs deliberately retain the actual account identifier `Noktyros`, as
recorded in the unchanged origin remote. An alias correction does not rename
an external account. Upstream notices remain unchanged; only RetroHatch's own
MIT integration-addition attribution in tg18-core is corrected.

`app/Cargo.toml` is the sole application version source. `app/Cargo.lock`
records the same version; About uses `env!("CARGO_PKG_VERSION")`. The permanent
local AGENTS.md policy defines one SemVer increment per completed application
change: PATCH for fixes, MINOR for functionality, no increment for incomplete,
analysis-only or documentation-only tasks. Breaking versions require approval;
MAJOR remains zero unless explicitly instructed otherwise. Establishing the
policy and correcting this metadata leaves the existing version unchanged.

On Windows, `app/build.rs` always builds a VERSIONINFO resource with Windows
SDK rc.exe, independently of optional private ICO branding. CompanyName is
Finn Lülsdorf / Noctyros; ProductName is RetroHatch; FileDescription is
RetroHatch Tamagotchi Emulator; OriginalFilename is RetroHatch.exe. FileVersion
and ProductVersion strings retain the Cargo SemVer, while fixed numerical
fields map MAJOR.MINOR.PATCH to MAJOR,MINOR,PATCH,0 (16-bit components).
Non-Windows targets skip RC compilation. No new dependency is required.

The packager reads the Cargo manifest and requires a matching version tag.
Windows archives are named `RetroHatch-vX.Y.Z-Windows.zip`; the executable
inside remains `RetroHatch.exe`. Source archives keep the matching version.
The workflow upload path and README use the same naming. Packaging only runs
when explicitly requested or preparing an authorized release, not as part of
ordinary development builds. No archive, tag or release is created by this
metadata update. Existing publication/provenance gates still apply.

Metadata-update verification: formatting, locked Cargo check, the full Rust
suite (177 passed, one ignored), two build-resource tests, nine packaging tests
and the optimized Windows build pass. Windows FileVersionInfo confirms the
Cargo string version, fixed four-part version and Unicode author. An isolated
Windows build without resources/ also retains correct version/author fields.
The license inventory verifies 106 packages. Upstream license texts and the
Cargo lockfile are unchanged. Only copyright comments change in emulator cores.
An unauthenticated request to the unchanged GitHub repository URL returns 404;
private-repository availability cannot be verified publicly. No archive, tag,
commit, push, release, remote or visibility change is performed.
