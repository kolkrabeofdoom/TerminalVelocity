param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    composer install --no-interaction
    if ($LASTEXITCODE -ne 0) { throw 'Composer install failed' }
    if (-not $SkipTests) {
        composer selftest
        if ($LASTEXITCODE -ne 0) { throw 'Selftest failed' }
    }
    Write-Host 'TerminalVelocity 2.0 build complete.'
} finally {
    Pop-Location
}
