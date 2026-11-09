<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros -->

# Release and redistribution checklist

This is a preparation checklist, not permission to publish. The repository
remains private. Do not push, create a release, change visibility or alter owner
settings as part of running local checks. Known LCD artwork provenance and
preferred-source gaps currently block public distribution.

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
- Obtain the owner's explicit release authorization after reviewing the final
  artifacts and unresolved-risk report. This task grants no such authorization.

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
No public release automation or GitHub setting changes are included.


## Build-path privacy

A default Rust release build can embed absolute source/dependency paths in panic
metadata even without PDB files. The preparation audit found personal build-home
prefixes in the ordinary executable. For a future distribution candidate, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File app/tools/build_release.ps1
```

This local-only wrapper uses dynamically discovered path-prefix remapping for
the user home, project and Rust toolchain. It restores the process environment
after building, respects an existing CARGO_TARGET_DIR, and never uploads or
authorizes distribution. Custom flags use CARGO_ENCODED_RUSTFLAGS to avoid
whitespace/quoting errors. Re-scan the actual binary for personal paths after
changing the toolchain or build location. Remapping addresses a privacy issue,
not the outstanding asset/licensing release gates. See the
[rustc path-remapping documentation](https://doc.rust-lang.org/rustc/command-line-arguments.html#--remap-path-prefix-remap-source-names-in-output).
