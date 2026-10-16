param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $projectRoot 'Cargo.toml'
if (-not $SkipTests) {
    & cargo test --locked --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
}
& cargo build --release --locked --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }

