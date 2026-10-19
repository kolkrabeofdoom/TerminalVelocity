$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    # Fail explicitly instead of silently skipping the offline integration tests.
    Get-Command pwsh.exe -ErrorAction Stop | Out-Null
    Get-Command ssh.exe -ErrorAction Stop | Out-Null

    & cargo fmt -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
    & cargo test --locked -- --include-ignored
    if ($LASTEXITCODE -ne 0) { throw 'Offline tests failed' }
    & (Join-Path $PSScriptRoot 'build.ps1') -SkipTests

    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
    $executable = Join-Path $targetRoot 'release/rust-terminal.exe'
    foreach ($mode in @('--directory-test', '--restart-test', '--profile-test', '--search-test')) {
        Write-Host "Running $mode"
        $process = Start-Process -FilePath $executable -ArgumentList $mode -WindowStyle Hidden -PassThru
        if (-not $process.WaitForExit(120000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            throw "$mode exceeded two minutes"
        }
        $process.Refresh()
        if ($process.ExitCode -ne 0) { throw "$mode failed with exit code $($process.ExitCode)" }
    }
} finally {
    Pop-Location
}
