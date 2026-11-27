# Verifies that vendor/<crate> is exactly the original crate from crates.io plus
# vendor/patches/<crate>.patch. Any change to vendored code that is not recorded in
# the patch fails the check.
#
#   ./scripts/vendor-check.ps1            check (CI)
#   ./scripts/vendor-check.ps1 -Update    regenerate patches and dropped-file lists
#                                         from the committed vendor tree (HEAD)
#
# Originals are downloaded from static.crates.io and verified against the SHA256 in
# vendor/patches/sources.json (the crates.io index checksum).
param(
    [switch]$Update,
    [string]$PatchDirectory
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $PatchDirectory) { $PatchDirectory = Join-Path $projectRoot 'vendor/patches' }
$PatchDirectory = [IO.Path]::GetFullPath($PatchDirectory)
New-Item -ItemType Directory -Path $PatchDirectory -Force | Out-Null
# Read sources.json from the patch directory if present (a regenerated set), otherwise
# from the repository; -Update writes it to the patch directory.
$sourcesPath = Join-Path $PatchDirectory 'sources.json'
if (-not (Test-Path -LiteralPath $sourcesPath)) { $sourcesPath = Join-Path $projectRoot 'vendor/patches/sources.json' }
$sources = Get-Content -LiteralPath $sourcesPath -Raw -Encoding utf8 | ConvertFrom-Json
$tempBase = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$work = Join-Path $tempBase ("tv-vendor-check-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null

function Invoke-Git {
    # core.autocrlf=false: patches and originals are handled byte-exact.
    & git -c core.autocrlf=false -c core.quotepath=false @args
}

function Get-RelativeFiles([string]$root) {
    Get-ChildItem -LiteralPath $root -File -Recurse -Force | ForEach-Object {
        [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/')
    } | Sort-Object
}

function Expand-Original($crate, [string]$target) {
    $name = "$($crate.name)-$($crate.version)"
    $archive = Join-Path $work "$name.crate"
    if (-not (Test-Path -LiteralPath $archive)) {
        Invoke-WebRequest -Uri "https://static.crates.io/crates/$($crate.name)/$name.crate" -OutFile $archive
    }
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $crate.sha256) { throw "$name`: SHA256 mismatch ($hash)" }
    New-Item -ItemType Directory -Path $target | Out-Null
    & tar -xzf $archive -C $target --strip-components=1
    if ($LASTEXITCODE -ne 0) { throw "$name`: extraction failed" }
}

Push-Location $projectRoot
try {
    $failures = @()
    foreach ($crate in $sources.crates) {
        $name = "$($crate.name)-$($crate.version)"
        $original = Join-Path $work "$name-orig"
        Expand-Original $crate $original
        $patch = Join-Path $PatchDirectory "$name.patch"

        if ($Update) {
            # Committed vendor tree as the "after" side.
            $vendored = Join-Path $work "$name-vend"
            New-Item -ItemType Directory -Path $vendored | Out-Null
            $tarball = Join-Path $work "$name.tar"
            Invoke-Git archive --format=tar --output=$tarball HEAD "vendor/$name"
            if ($LASTEXITCODE -ne 0) { throw "$name`: git archive failed" }
            & tar -xf $tarball -C $vendored --strip-components=2
            if ($LASTEXITCODE -ne 0) { throw "$name`: extraction of vendor tree failed" }
            $vendorFiles = Get-RelativeFiles $vendored
            $dropped = @(Get-RelativeFiles $original | Where-Object { $vendorFiles -notcontains $_ })
            foreach ($file in $dropped) { Remove-Item -LiteralPath (Join-Path $original $file) -Force }
            # git diff needs relative directory names to produce a/orig/... b/vend/... paths.
            Rename-Item -LiteralPath $original -NewName 'orig'
            Rename-Item -LiteralPath $vendored -NewName 'vend'
            Push-Location $work
            try {
                Invoke-Git diff --no-index --binary --no-color --src-prefix=a/ --dst-prefix=b/ --output=$patch orig vend
                if ($LASTEXITCODE -gt 1) { throw "$name`: git diff failed" }
            } finally {
                Pop-Location
            }
            Remove-Item -LiteralPath (Join-Path $work 'orig'), (Join-Path $work 'vend') -Recurse -Force
            $crate.dropped = $dropped
            Write-Host "$name`: patch and $($dropped.Count) dropped file(s) written"
            continue
        }

        foreach ($file in $crate.dropped) { Remove-Item -LiteralPath (Join-Path $original $file) -Force }
        if ((Get-Item -LiteralPath $patch).Length -gt 0) {
            Push-Location $original
            try {
                Invoke-Git apply -p2 --whitespace=nowarn $patch
                if ($LASTEXITCODE -ne 0) { throw "$name`: patch does not apply to the original" }
            } finally {
                Pop-Location
            }
        }
        # Compare blob ids of the patched original with the committed tree. hash-object
        # --path applies the repository's attributes, so line-ending rules match.
        $expected = @{}
        foreach ($file in Get-RelativeFiles $original) {
            $expected[$file] = (Invoke-Git hash-object "--path=vendor/$name/$file" (Join-Path $original $file)).Trim()
        }
        $actual = @{}
        foreach ($line in Invoke-Git ls-tree -r HEAD "vendor/$name") {
            $meta, $path = $line -split "`t", 2
            $actual[$path.Substring("vendor/$name/".Length)] = ($meta -split ' ')[2]
        }
        $mismatch = @(@($expected.Keys) + @($actual.Keys) | Sort-Object -Unique | Where-Object { $expected[$_] -ne $actual[$_] })
        if ($mismatch.Count -gt 0) {
            $failures += "$name`: " + (($mismatch | Select-Object -First 10) -join ', ')
        } else {
            Write-Host "$name`: vendored tree matches original + patch ($($actual.Count) files)"
        }
    }

    if ($Update) {
        $outSources = Join-Path $PatchDirectory 'sources.json'
        $sources | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $outSources -Encoding utf8NoBOM
        Write-Host "Updated $outSources"
    } elseif ($failures.Count -gt 0) {
        $failures | ForEach-Object { Write-Host $_ }
        throw 'Vendored code differs from original + patch. Document the change in VENDOR_CHANGES.md and run ./scripts/vendor-check.ps1 -Update.'
    }
} finally {
    Pop-Location
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
