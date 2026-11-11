# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Optional private workspace paths; public builds never require local data."""
import os
from pathlib import Path
import subprocess
import json

ROOT = Path(__file__).resolve().parents[2]


def local_dir():
    return Path(os.environ.get("RETROHATCH_LOCAL_DIR", ROOT.parent / "local")).resolve()


def references_dir():
    return Path(os.environ.get("RETROHATCH_REFERENCES_DIR", local_dir() / "references")).resolve()


def target_dir():
    # Cargo resolves configuration-relative paths and CARGO_TARGET_DIR itself.
    metadata = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--locked", "--offline", "--format-version", "1"],
        cwd=ROOT / "app", text=True, encoding="utf-8")
    return Path(json.loads(metadata)["target_directory"])


def release_dir():
    # GitHub Actions and ordinary public checkouts retain the dist/ convention.
    local = local_dir()
    return local / "releases" if local.is_dir() else ROOT / "dist"
