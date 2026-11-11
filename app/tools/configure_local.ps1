# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
# Optional developer setup only; never run by the public release workflow.
param([string]$LocalRoot)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
if (-not $LocalRoot) { $LocalRoot = Join-Path (Split-Path $projectRoot) 'local' }
$LocalRoot = [IO.Path]::GetFullPath($LocalRoot)
if ($LocalRoot.StartsWith($projectRoot + [IO.Path]::DirectorySeparatorChar) -or $LocalRoot -eq $projectRoot) {
    throw 'LocalRoot must remain outside the source repository'
}
$config = Join-Path $projectRoot '.cargo/config.toml'
$buildPath = (Join-Path $LocalRoot 'builds/cargo').Replace('\', '/')
$localPath = $LocalRoot.Replace('\', '/')
$content = "# Local-only paths. Do not commit this configuration.`n[build]`ntarget-dir = '$buildPath'`n`n[env]`nRETROHATCH_LOCAL_DIR = '$localPath'`n"
if (Test-Path -LiteralPath $config) {
    if ([IO.File]::ReadAllText($config) -ne $content) { throw 'Existing Cargo configuration preserved; merge the local target-dir manually' }
} else {
    foreach ($folder in @('builds/cargo', 'releases', 'roms', 'references', 'resources', 'test-data', 'backups')) {
        New-Item -ItemType Directory -Force -Path (Join-Path $LocalRoot $folder) | Out-Null
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $config) | Out-Null
    [IO.File]::WriteAllText($config, $content, (New-Object Text.UTF8Encoding($false)))
}
Write-Output 'Local Cargo output configured. Public builds do not need this step.'
