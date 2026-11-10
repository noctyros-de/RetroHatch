# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
# Build locally or configure the current process for a CI distribution build.
# This script never packages or uploads artifacts.
param([switch]$Offline, [switch]$ConfigureOnly)
$ErrorActionPreference = 'Stop'
$appRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$projectRoot = [IO.Path]::GetFullPath((Join-Path $appRoot '..'))
$userRoot = [Environment]::GetFolderPath('UserProfile')
$compilerRoot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate the Rust toolchain' }
$previousEncoded = $env:CARGO_ENCODED_RUSTFLAGS
if ($env:RUSTFLAGS -and -not $previousEncoded) {
    throw 'Supply custom flags through CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS; the existing environment has not been changed.'
}
$flags = @()
if ($previousEncoded) { $flags += $previousEncoded.Split([char]31) }
$flags += '-Ctarget-feature=+crt-static'
foreach ($mapping in @(@($userRoot, '/build-user'), @($projectRoot, '/retrohatch'), @($compilerRoot, '/rust-toolchain'))) {
    if (-not [string]::IsNullOrWhiteSpace($mapping[0])) {
        $flags += '--remap-path-prefix=' + $mapping[0] + '=' + $mapping[1]
        $flags += '--remap-path-prefix=' + $mapping[0].Replace('\', '/') + '=' + $mapping[1]
    }
}
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
if ($ConfigureOnly) { return }
Push-Location $appRoot
try {
    $arguments = @('build', '--release', '--locked')
    if ($Offline) { $arguments += '--offline' }
    & cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw 'The optimized distribution build failed' }
} finally {
    Pop-Location
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncoded
}
