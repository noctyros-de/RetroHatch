# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noktyros
# Local build only. This script never packages, uploads or authorizes a release.
param([switch]$Offline)
$ErrorActionPreference = 'Stop'
$appRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$projectRoot = [IO.Path]::GetFullPath((Join-Path $appRoot '..'))
$userRoot = [Environment]::GetFolderPath('UserProfile')
$compilerRoot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate the Rust toolchain' }
$previousEncoded = $env:CARGO_ENCODED_RUSTFLAGS
if ($env:RUSTFLAGS -and -not $previousEncoded) {
    throw 'For this privacy build, supply custom flags through CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS; the existing environment has not been changed.'
}
$flags = @()
if ($previousEncoded) { $flags += $previousEncoded.Split([char]31) }
foreach ($mapping in @(@($userRoot, '/build-user'), @($projectRoot, '/retrohatch'), @($compilerRoot, '/rust-toolchain'))) {
    if (-not [string]::IsNullOrWhiteSpace($mapping[0])) {
        $flags += '--remap-path-prefix=' + $mapping[0] + '=' + $mapping[1]
        $flags += '--remap-path-prefix=' + $mapping[0].Replace('\', '/') + '=' + $mapping[1]
    }
}
try {
    $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
    $arguments = @('build', '--manifest-path', (Join-Path $appRoot 'Cargo.toml'), '--release', '--locked')
    if ($Offline) { $arguments += '--offline' }
    & cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw 'The optimized privacy build failed' }
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncoded
}
