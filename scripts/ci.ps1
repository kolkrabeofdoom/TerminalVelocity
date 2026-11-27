$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    # Fail explicitly instead of silently skipping the offline integration tests.
    Get-Command pwsh.exe -ErrorAction Stop | Out-Null
    Get-Command ssh.exe -ErrorAction Stop | Out-Null
    & (Join-Path $PSScriptRoot 'prepare-conpty.ps1')

    & cargo fmt -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
    & cargo clippy --locked --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    # Release builds have no test-tools feature; they must be warning-free too.
    & cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy without features failed' }
    & cargo test --locked -- --include-ignored
    if ($LASTEXITCODE -ne 0) { throw 'Offline tests failed' }
    & cargo test --locked -p vte --lib
    if ($LASTEXITCODE -ne 0) { throw 'Parser regression tests failed' }
    & cargo test --locked -p egui_term --lib
    if ($LASTEXITCODE -ne 0) { throw 'egui_term tests failed' }
    # PTY test modes exist only with the test-tools feature.
    & (Join-Path $PSScriptRoot 'build.ps1') -SkipTests -Features test-tools

    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
    $executable = Join-Path $targetRoot 'release/TerminalVelocity.exe'
    foreach ($mode in @('--directory-test', '--restart-test', '--profile-test', '--search-test')) {
        Write-Host "Running $mode"
        $process = Start-Process -FilePath $executable -ArgumentList $mode -WindowStyle Hidden -PassThru
        if (-not $process.WaitForExit(120000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            throw "$mode exceeded two minutes"
        }
        $process.Refresh()
        # Show the test report (e.g. directory-test.txt) so failures name the shell and cause.
        $report = Join-Path (Split-Path -Parent $executable) ($mode.TrimStart('-') + '.txt')
        if (Test-Path -LiteralPath $report) { Get-Content -LiteralPath $report -Encoding utf8 | Write-Host }
        if ($process.ExitCode -ne 0) { throw "$mode failed with exit code $($process.ExitCode)" }
    }

    # Rebuild as shipped (no test-tools) and make sure the EXE refuses test modes.
    & (Join-Path $PSScriptRoot 'build.ps1') -SkipTests
    $probe = Start-Process -FilePath $executable -ArgumentList '--directory-test' -WindowStyle Hidden -PassThru
    if (-not $probe.WaitForExit(30000)) {
        Stop-Process -Id $probe.Id -Force -ErrorAction SilentlyContinue
        throw 'Release executable contains test modes (probe timed out)'
    }
    $probe.Refresh()
    if ($probe.ExitCode -ne 2) { throw "Release executable contains test modes (exit code $($probe.ExitCode), expected 2)" }
    Write-Host 'Release build refuses test modes (exit code 2)'
} finally {
    Pop-Location
}
