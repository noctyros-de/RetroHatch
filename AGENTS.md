# RetroHatch project language policy

This permanent policy applies to all RetroHatch project content and future changes.

- Write all code, code comments, API documentation, Markdown documentation,
  developer guides, UI labels, user-facing messages, test descriptions, and Git
  commit messages in professional, internationally understandable English.
- Keep explanations, progress reports, and completion reports addressed to the
  user in German, unless the user explicitly requests another language.
- Preserve established technical terminology, instruction mnemonics, file paths,
  identifiers, data formats, and externally defined names. Do not rename them
  solely for translation.
- Language cleanup must preserve algorithms, CPU emulation behavior, tests, and
  application functionality. Do not introduce features or refactor code as part
  of a translation task.
- Do not modify original third-party projects under the optional sibling
  `../local/references/`, including
  their existing language. These projects are outside the repository and excluded from language cleanup.

Run the relevant checks from `app/` after changes:

```text
cargo fmt --check
cargo check --locked
cargo test --locked
```

## Attribution and version management

- The creator is Finn Lülsdorf; the developer alias is Noctyros. Preserve
  third-party authorship and upstream license notices. External account names
  and URLs are identifiers: do not rename them merely to match the alias.
- app/Cargo.toml is the single source of truth for the application version.
  Use Semantic Versioning (MAJOR.MINOR.PATCH). Derive the About dialog,
  Windows executable metadata and release filenames from that version.
- After each completed development task with actual application changes,
  increment the version automatically once: PATCH for bug fixes; MINOR for
  new functionality, resetting PATCH to zero. For mixed tasks use the higher
  applicable increment, regardless of the number of changed files.
- Keep MAJOR at 0 during development unless explicitly instructed otherwise.
  Breaking changes require explicit approval before choosing a new version.
- Do not increment for analysis-only or documentation-only tasks, incomplete
  implementations, or merely introducing this version-management policy and
  correcting metadata. Do not count pre-existing work as a new completed task.
- Always update app/Cargo.toml when incrementing, synchronize app/Cargo.lock
  through Cargo, and ensure all application components use the same version.
  Include the previous and new version in the final development report.
- The executable remains RetroHatch.exe. Windows ZIPs are named
  RetroHatch-vX.Y.Z-Windows.zip using the Cargo version, and are generated only
  when explicitly requested or preparing an authorized release, not for every
  development build.
- A version increment alone does not authorize committing or pushing. An
  explicitly authorized push of a new Cargo version to main runs the automatic
  verified release workflow, including its tag and publication; no separate
  manual tag or release description is required. Never change visibility,
  rename repositories or modify remotes without explicit authorization.

## Public repository and owner-managed gallery

- The public project URL is https://github.com/noctyros-de/RetroHatch. Keep local
  ROMs, saves, resources and private research outside the source repository. Public status does not
  authorize tags, commits, pushes, releases or repository-setting changes.
- Treat README Screenshots and Gameplay Videos sections, their identifiable
  USER-MANAGED markers and docs/screenshots/ content as owner-managed.
- Inspect the latest gallery before editing. Preserve every existing image
  reference, video link, caption, description and the owner's chosen ordering.
- Never delete, rename or overwrite media files, references, links or captions
  without explicit user authorization. Add entries only when explicitly asked.
- Preserve gallery sections when changing other README content; use targeted
  edits and never regenerate the entire README for an unrelated update.
- Do not redesign the gallery during routine development. Preserve manual edits.
- If local and remote README differ, inspect the differences and preserve both
  sets of user-managed content. Never blindly overwrite either version.
- Never automatically copy or upload Pictures captures, recordings, ROM-derived
  artwork or resources/ into the public gallery. Public gallery files require
  deliberate owner selection and appropriate publication rights.

## Automatic release workflow

- Preserve the Semantic Versioning rules above. Cargo.toml is authoritative;
  synchronize Cargo.lock through Cargo and regenerate/verify license evidence
  before committing a version change:
  `python -B app/tools/audit_licenses.py`, then `--check`, followed by the audit,
  packaging and automatic-release regression tests.
- An authorized push to main with a changed application version automatically
  builds, validates licenses, packages Windows/matching vendored source, verifies
  an independent offline source build, then creates the immutable vX.Y.Z tag
  and publishes the verified assets with commit-derived notes.
- Ordinary commits without a version change do not release. Already published
  versions are skipped. Always publish normal GitHub releases with
  prerelease: false, regardless of major version or version suffix. Never
  move/force-push version tags or replace public assets.
- No manual tags, release descriptions, ZIP uploads or release configuration
  are needed. Keep known limitations in .github/release-limitations.md accurate
  when functionality changes; never invent features in release notes.
- Failed runs stop before publication. Retry the original Actions run (or the
  main workflow_dispatch when no different commit owns its tag). Foreign drafts
  or mismatched existing draft assets require review, never silent replacement.
- This workflow is permanently authorized as part of an authorized version
  push; it does not authorize Codex to commit/push unrelated work on its own.

## Workspace layout and safe maintenance

- The Git repository is project/ inside the developer workspace. Its sibling
  local/ contains private references, ROMs, branding originals, test diagnostics,
  build output, distribution drafts and backups. Never copy that private data
  into source packages or the Git index. AGENTS.md contains public instructions.
- Use app/tools/configure_local.ps1 for optional external Cargo output. See
  developer/WORKSPACE.md for path overrides and clean-checkout behavior.
- Preserve synthetic regression fixtures and their generators; no proprietary
  firmware is required for Cargo tests. Do not remove cases to reduce file size.
- Layout/documentation changes alone do not require an application version bump.
- Do not commit, push, tag or release a workspace migration without authorization.

## Compact device selection and action feedback

Device selection menus show category headings, model names and selection only.
Keep implementation notes, verification status and technical limitations in
project documentation. Show technical warnings in the application only when
directly relevant to the user's current action. Temporary status feedback belongs
in the fixed footer, never in the menu bar or over the LCD/device controls.
