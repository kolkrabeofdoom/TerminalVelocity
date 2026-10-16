param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build.ps1') }
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'Cargo.toml') -Raw
$version = [regex]::Match($manifest, '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $version) { throw 'Package version missing' }
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
$executable = Join-Path $targetRoot 'release/rust-terminal.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw "Release executable missing: $executable" }
$packageRoot = Join-Path $projectRoot 'dist/RustTerminal'
if (Test-Path -LiteralPath $packageRoot) { throw 'dist/RustTerminal already exists; choose a clean output directory before packaging.' }
New-Item -ItemType Directory -Path (Join-Path $packageRoot 'shells/powershell') -Force | Out-Null
Copy-Item -LiteralPath $executable -Destination (Join-Path $packageRoot 'RustTerminal.exe')
foreach ($name in @('README.md','LICENSE','VERIFICATION.md','VENDOR_CHANGES.md','THIRD_PARTY_LICENSES.txt')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $packageRoot
}
foreach ($name in @('rust-terminal.bash','powershell-source.json')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "shells/$name") -Destination (Join-Path $packageRoot "shells/$name")
}
$source = Get-Content -LiteralPath (Join-Path $projectRoot 'shells/powershell-source.json') -Raw | ConvertFrom-Json
$cacheRoot = Join-Path $projectRoot 'work/downloads'
New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
$archive = Join-Path $cacheRoot 'powershell-win-x64.zip'
Invoke-WebRequest -Uri $source.source -OutFile $archive
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $source.sha256) { throw 'PowerShell SHA256 mismatch' }
Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $packageRoot 'shells/powershell')
$zip = Join-Path $projectRoot "dist/RustTerminal-$version-Windows-x64.zip"
Compress-Archive -LiteralPath $packageRoot -DestinationPath $zip
Write-Output $zip

