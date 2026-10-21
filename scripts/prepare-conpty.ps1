$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$source = Get-Content -LiteralPath (Join-Path $projectRoot 'shells/conpty-source.json') -Raw | ConvertFrom-Json
$targetRoot = if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $projectRoot 'target' }
$cache = Join-Path $projectRoot 'work/downloads'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$archive = Join-Path $cache "conpty-$($source.version).zip"
if (-not (Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $source.sha256) {
    Invoke-WebRequest -Uri $source.source -OutFile "$archive.partial"
    if ((Get-FileHash -LiteralPath "$archive.partial" -Algorithm SHA256).Hash -ne $source.sha256) { throw 'ConPTY SHA256 mismatch' }
    Move-Item -LiteralPath "$archive.partial" -Destination $archive -Force
}
# Read the verified archive directly, avoiding trust in previously extracted files.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    foreach ($pair in @(@('runtimes/win-x64/native/conpty.dll','conpty.dll'), @('build/native/runtimes/x64/OpenConsole.exe','OpenConsole.exe'))) {
        $entry = $zip.GetEntry($pair[0])
        if (-not $entry) { throw "ConPTY runtime file missing: $($pair[0])" }
        $stream = $entry.Open()
        $memory = [IO.MemoryStream]::new()
        try { $stream.CopyTo($memory); $bytes = $memory.ToArray() } finally { $stream.Dispose(); $memory.Dispose() }
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
        foreach ($directory in @('debug','debug/deps','release','release/deps')) {
            $destination = Join-Path $targetRoot $directory
            New-Item -ItemType Directory -Path $destination -Force | Out-Null
            $file = Join-Path $destination $pair[1]
            if (-not (Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $hash) {
                [IO.File]::WriteAllBytes($file, $bytes)
            }
        }
    }
} finally { $zip.Dispose() }
