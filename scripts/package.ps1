param([switch]$SkipBuild, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'dist' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'Cargo.toml') -Raw
$version = [regex]::Match($manifest, '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $version) { throw 'Package version missing' }
$packageRoot = Join-Path $OutputDirectory 'TerminalVelocity'
$zip = Join-Path $OutputDirectory "TerminalVelocity-$version-Windows-x64.zip"
if ((Test-Path -LiteralPath $packageRoot) -or (Test-Path -LiteralPath $zip)) { throw 'Output already exists. Choose a fresh -OutputDirectory.' }
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'build.ps1') }
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
$executable = Join-Path $targetRoot 'release/TerminalVelocity.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw "Release executable missing: $executable" }
$source = Get-Content -LiteralPath (Join-Path $projectRoot 'shells/powershell-source.json') -Raw | ConvertFrom-Json
$cacheRoot = Join-Path $projectRoot 'work/downloads'
New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
$archive = Join-Path $cacheRoot "powershell-$($source.version)-win-x64.zip"
$validCache = (Test-Path -LiteralPath $archive) -and ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -eq $source.sha256)
if (-not $validCache) {
    $download = "$archive.partial"
    Invoke-WebRequest -Uri $source.source -OutFile $download
    if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ne $source.sha256) { throw 'PowerShell SHA256 mismatch' }
    Move-Item -LiteralPath $download -Destination $archive -Force
}
New-Item -ItemType Directory -Path (Join-Path $packageRoot 'shells/powershell') -Force | Out-Null
Copy-Item -LiteralPath $executable -Destination (Join-Path $packageRoot 'TerminalVelocity.exe')
& (Join-Path $PSScriptRoot 'prepare-conpty.ps1')
foreach ($name in @('conpty.dll','OpenConsole.exe')) {
    Copy-Item -LiteralPath (Join-Path $targetRoot "release/$name") -Destination $packageRoot
}
foreach ($name in @('README.md','RELEASE_NOTES.md','LICENSE','VERIFICATION.md','VENDOR_CHANGES.md','THIRD_PARTY_LICENSES.txt','CHANGELOG.md')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $packageRoot
}
foreach ($name in @('terminalvelocity.bash','sixel-demo.ps1','powershell-source.json','conpty-source.json','conpty-LICENSE.txt')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "shells/$name") -Destination (Join-Path $packageRoot "shells/$name")
}
Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $packageRoot 'shells/powershell')
$revision = (& git -C $projectRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine source revision' }
$dirty = [bool](& git -C $projectRoot status --porcelain --untracked-files=normal)
$files = [ordered]@{}
Get-ChildItem -LiteralPath $packageRoot -File -Recurse | Sort-Object FullName | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($packageRoot, $_.FullName).Replace('\','/')
    $files[$relative] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
}
[ordered]@{
    version = $version
    source_commit = $revision
    source_dirty = $dirty
    rustc = (& rustc -V).Trim()
    cargo = (& cargo -V).Trim()
    cargo_lock_sha256 = (Get-FileHash -LiteralPath (Join-Path $projectRoot 'Cargo.lock') -Algorithm SHA256).Hash.ToLowerInvariant()
    powershell_version = $source.version
    conpty_version = (Get-Content -LiteralPath (Join-Path $projectRoot 'shells/conpty-source.json') -Raw | ConvertFrom-Json).version
    files = $files
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $packageRoot 'BUILD-INFO.json') -Encoding utf8NoBOM
# Stable ordering and timestamps produce identical ZIPs for identical input files/tooling.
Add-Type -AssemblyName System.IO.Compression
$stream = [IO.File]::Open($zip, [IO.FileMode]::CreateNew)
try {
    $zipArchive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $true)
    try {
        Get-ChildItem -LiteralPath $packageRoot -File -Recurse | Sort-Object FullName | ForEach-Object {
            $entryName = 'TerminalVelocity/' + [IO.Path]::GetRelativePath($packageRoot, $_.FullName).Replace('\','/')
            $entry = $zipArchive.CreateEntry($entryName, [IO.Compression.CompressionLevel]::Optimal)
            $entry.LastWriteTime = [DateTimeOffset]::new(2000,1,1,0,0,0,[TimeSpan]::Zero)
            $destination = $entry.Open()
            $inputFile = [IO.File]::OpenRead($_.FullName)
            try { $inputFile.CopyTo($destination) } finally { $inputFile.Dispose(); $destination.Dispose() }
        }
    } finally { $zipArchive.Dispose() }
} finally { $stream.Dispose() }
$hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $([IO.Path]::GetFileName($zip))" | Set-Content -LiteralPath "$zip.sha256" -Encoding ascii
Write-Output $zip
