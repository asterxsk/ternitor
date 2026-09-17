<#
.SYNOPSIS
    Builds Ternitor-Setup.exe with Inno Setup.

.DESCRIPTION
    Ships the exe that sits in the repo root beside install.ps1, not one out of
    target\release, so the installer, the scripts and the committed binary are
    always the same bytes.

    Two things are taken off that exe instead of being kept as a second copy that
    could fall out of step: the version the shell will show in Settings > Apps,
    and the icon the setup file wears. Both are read out of the exe's own
    resources.

    Requires Inno Setup 6 (winget install JRSoftware.InnoSetup).

.PARAMETER Exe
    The binary to ship. Defaults to ternitor.exe in the repo root.

.PARAMETER Out
    Where to write the setup file. Defaults to Ternitor-Setup.exe in the repo root.

.EXAMPLE
    .\installer\build.ps1
#>
[CmdletBinding()]
param(
    [string]$Exe = (Join-Path (Split-Path -Parent $PSScriptRoot) 'Ternitor.exe'),
    [string]$Out
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
if (-not $Out) { $Out = Join-Path $root 'Ternitor-Setup.exe' }

if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw "No ternitor.exe at '$Exe'. Build one with 'cargo build --release' and copy it to the repo root."
}
$Exe = (Resolve-Path -LiteralPath $Exe).Path

$iscc = @(
    (Get-Command 'iscc' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source),
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1

if (-not $iscc) {
    throw "No ISCC.exe. Install Inno Setup 6: winget install JRSoftware.InnoSetup"
}

# The version resource, so Settings > Apps has one place it can be wrong.
$version = (Get-Item -LiteralPath $Exe).VersionInfo.FileVersion
if (-not $version) { $version = '0.0.0.0' }

# The setup file wears the app's own mark, lifted out of the exe rather than kept
# as a second asset: mark.rs already draws it, and build.rs already bakes it in.
$icon = Join-Path ([System.IO.Path]::GetTempPath()) "ternitor-setup-$PID.ico"
Add-Type -AssemblyName System.Drawing
$mark = [System.Drawing.Icon]::ExtractAssociatedIcon($Exe)
$stream = [System.IO.File]::Create($icon)
try { $mark.Save($stream) } finally { $stream.Close(); $mark.Dispose() }

Write-Host "Building Ternitor-Setup $version"
try {
    & $iscc "/DVersion=$version" "/DIconPath=$icon" `
        "/O$(Split-Path -Parent $Out)" "/F$([System.IO.Path]::GetFileNameWithoutExtension($Out))" `
        (Join-Path $PSScriptRoot 'ternitor.iss')
    if ($LASTEXITCODE -ne 0) { throw "ISCC exited $LASTEXITCODE" }
} finally {
    Remove-Item -LiteralPath $icon -Force -ErrorAction SilentlyContinue
}

$setup = Get-Item -LiteralPath $Out
Write-Host ("Wrote {0} ({1:N0} KB)" -f $setup.FullName, ($setup.Length / 1KB))
