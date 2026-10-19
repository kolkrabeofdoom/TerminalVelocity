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
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
$shellRoot = Join-Path $targetRoot 'release/shells'
New-Item -ItemType Directory -Path $shellRoot -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'shells/rust-terminal.bash') -Destination (Join-Path $shellRoot 'rust-terminal.bash')

