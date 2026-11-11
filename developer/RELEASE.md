<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->

# Release and redistribution checklist

RetroHatch is a public project at https://github.com/noctyros-de/RetroHatch.
Local build/packaging commands never publish or change repository settings.
The workflow publishes automatically after an authorized push to main changes
the Cargo version. No manual tag, release description or archive upload is
needed. Public source, local development
builds and published binary releases are distinct. See the current
[audit](OPEN_SOURCE_AUDIT.md) for observed release assets and unresolved rights
questions; an existing public release is not evidence of complete clearance.

## Redistribution requirements and open review items

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
- Build/check/test a clean source snapshot without `references/`, internal docs, ROMs,
  saves or user configuration. Record the exact source revision, toolchain,
  target, features, build command and artifact hashes. Re-run the license
  inventory when Cargo.lock, features, target or toolchain changes.
- Obtain the owner's release authorization after reviewing the final artifacts
  and unresolved-risk report, before pushing a version change to main. That push starts
  publication automatically; a local version bump/package does not.

The incorporated tamaemu translation has a specific open question: clarify the
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

## Automatic Windows releases

Push the reviewed application version and synchronized Cargo.lock/license
inventories to main. Version detection reads app/Cargo.toml and the preceding
push commit. Unchanged versions and already published releases skip the build.
An optional main-branch workflow_dispatch retries an unreleased version; normal
releases need no manual action beyond the authorized version commit/push.

The Windows job checks formatting, locked Cargo check/tests, an optimized
statically linked/path-remapped EXE, complete license evidence and regression
checks. It packages the versioned Windows ZIP, notices and exact matching source
with vendored dependencies/hashes. An independent extracted offline source build
must succeed before any tag or draft is created. The final helper rechecks
checksums, required licenses/build identity and absence of firmware/user-state
files in the Windows package.

Only then does the repository GITHUB_TOKEN (contents: write) create the version
tag at the verified SHA, create a draft, upload checked files and publish it.
No personal access token or manual GitHub release configuration is required.
Every version is published as a normal GitHub release with prerelease: false,
including versions below 1.0.0 and suffixed versions. This permanent policy does
not change the application version or move existing tags. Notes group actual
Git commit subjects since the last reachable
published release and include .github/release-limitations.md automatically.
Keep that standing limitations document accurate when functionality changes.

Tags are never moved, releases never duplicated and published files never
replaced. Each version has serialized publication. A matching auto-created draft
can resume missing uploads; a foreign draft, conflicting tag or mismatched
existing asset stops safely for review. Failed validation never publishes.
Rerun a failed Actions run instead of recreating v0.2.0 or any other old tag.
Any token/authentication/ruleset denial is an explicit workflow failure. This
source configuration does not alter repository visibility, remotes or settings.

License notice hashes use canonical LF text, so Windows CRLF checkouts preserve
the same evidence; substantive changes still invalidate checks. Before a version
commit run the existing audit generator, its --check mode and
app/tools/test_audit_licenses.py. CI verifies the committed evidence strictly;
it never silently rewrites/accepts an unaudited inventory.

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
GitHub links use the verified public address https://github.com/noctyros-de/RetroHatch.
The local origin remote may still show the former account; this audit does not
modify Git configuration. Third-party authors and license texts remain intact.

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

The packager reads Cargo and requires the matching version label; the actual
tag is created only after the workflow validates both archives/source build.
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
The earlier private/old-URL check was historical. On 11 November 2026 the
GitHub API confirms the new public address. The published `RetroHatch` tag
currently has only a RetroHatch.exe asset; its tagged Cargo version is 0.1.0.
No Windows ZIP, separate matching-source bundle, notices bundle or BUILD_INFO
asset is attached. GitHub tag source archives exist, but the exact EXE/source
match and preferred-source completeness were not established by this audit.
The current packager is designed to provide those artifacts for future releases;
no archive, tag, commit, push, release or visibility change is performed here.
