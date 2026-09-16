<#
.SYNOPSIS
    Installs Ternitor for the current user.

.DESCRIPTION
    Copies ternitor.exe to %LOCALAPPDATA%\Programs\Ternitor, adds a Start Menu
    shortcut, points the per-user Run key at it, and registers an uninstall
    entry so it can be removed from Settings > Apps like anything else.

    No administrator rights, no installer runtime, nothing written outside the
    user's own profile and HKCU -- which is also why there is no MSI here: MSI
    exists to do machine-wide installs, service registration and repair, and
    Ternitor has none of those.

.PARAMETER Source
    The ternitor.exe to install. Defaults to the one beside this script.

.PARAMETER InstallDir
    Where to put it. Defaults to %LOCALAPPDATA%\Programs\Ternitor.

.PARAMETER NoStartWithWindows
    Install without adding the Run entry, so Ternitor does not start at sign-in.

.PARAMETER NoLaunch
    Do not start Ternitor when the install finishes.

.EXAMPLE
    .\install.ps1
#>
[CmdletBinding()]
param(
    [string]$Source = (Join-Path $PSScriptRoot 'ternitor.exe'),
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\Ternitor'),
    [switch]$NoStartWithWindows,
    [switch]$NoLaunch
)

$ErrorActionPreference = 'Stop'

$AppName = 'Ternitor'

# Absolute, and stripped of any trailing separator, before a single path is quoted
# into a command line. A trailing separator escapes the closing quote of the
# -InstallDir argument the uninstall entry carries -- PowerShell then binds a path
# with a quote stuck on the end, so Settings > Apps removes the shortcut and the
# registry entries and leaves the exe on disk. A relative path resolves against
# whatever directory the shell happened to be in.
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
if ([System.IO.Path]::GetPathRoot($InstallDir) -eq $InstallDir) {
    throw "Refusing to install into a drive root: '$InstallDir'."
}
$InstallDir = $InstallDir.TrimEnd('\', '/')

$Exe = Join-Path $InstallDir "$AppName.exe"
$Uninstaller = Join-Path $InstallDir 'uninstall.ps1'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"
$Shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$AppName.lnk"
# What the PowerShell implementation installed before this one. Left behind, the
# .vbs would run a script that no longer exists at every logon.
$Legacy = Join-Path ([Environment]::GetFolderPath('Startup')) "$AppName.vbs"

function Write-Step($Text) { Write-Host "  $Text" }

if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
    throw "No ternitor.exe at '$Source'. Build one with 'cargo build --release' and copy it beside this script."
}
$Source = (Resolve-Path -LiteralPath $Source).Path

# The version the shell will show in Settings > Apps comes out of the exe's own
# version resource, so there is only one place it can be wrong.
$Version = (Get-Item -LiteralPath $Source).VersionInfo.FileVersion
if (-not $Version) { $Version = '0.0.0.0' }

Write-Host "Installing $AppName $Version"

# A running copy holds the exe open, and a second copy would fight the first over
# the tray icon and the window hook anyway -- the app enforces one instance.
Get-Process -Name $AppName -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Step "stopping running $AppName (pid $($_.Id))"
    Stop-Process -Id $_.Id -Force
    $_.WaitForExit()
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item -LiteralPath $Source -Destination $Exe -Force
Write-Step "installed $Exe"

# The uninstaller has to live where it can be found after the source folder is
# gone, because the uninstall entry points at this copy.
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall.ps1') -Destination $Uninstaller -Force
Write-Step "installed $Uninstaller"

$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($Shortcut)
$link.TargetPath = $Exe
$link.WorkingDirectory = $InstallDir
$link.Description = 'Hide the blank console windows Windows opens for other processes'
$link.Save()
Write-Step "shortcut $Shortcut"

if ($NoStartWithWindows) {
    Remove-ItemProperty -Path $RunKey -Name $AppName -ErrorAction SilentlyContinue
    Write-Step 'start with Windows: off'
} else {
    # Quoted: an unquoted path with a space would be split at the first one.
    Set-ItemProperty -Path $RunKey -Name $AppName -Value "`"$Exe`""
    Write-Step "start with Windows: on -> $Exe"
}

if (Test-Path -LiteralPath $Legacy) {
    Remove-Item -LiteralPath $Legacy -Force
    Write-Step "removed the old Startup launcher $Legacy"
}

# The uninstall entry. Add/Remove Programs reads exactly these values under HKCU,
# which is what puts Ternitor in Settings > Apps without an installer and without
# a machine-wide key.
$size = [int]((Get-Item -LiteralPath $Exe).Length / 1KB)
$uninstall = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$Uninstaller`" -InstallDir `"$InstallDir`""
New-Item -Path $UninstallKey -Force | Out-Null
$values = @{
    DisplayName          = $AppName
    DisplayVersion       = $Version
    Publisher            = 'asterxsk'
    DisplayIcon          = $Exe
    InstallLocation      = $InstallDir
    UninstallString      = $uninstall
    QuietUninstallString = "$uninstall -Quiet"
    EstimatedSize        = $size
    InstallDate          = (Get-Date -Format 'yyyyMMdd')
    NoModify             = 1
    NoRepair             = 1
}
foreach ($name in $values.Keys) {
    # Hoisted: `if` is a statement, so it cannot be passed inline as an argument.
    $kind = if ($values[$name] -is [int]) { 'DWord' } else { 'String' }
    New-ItemProperty -Path $UninstallKey -Name $name -Value $values[$name] -PropertyType $kind -Force | Out-Null
}
Write-Step "registered in Settings > Apps (version $Version)"

if (-not $NoLaunch) {
    Start-Process -FilePath $Exe
    Write-Step 'started'
}

if ($NoLaunch) {
    Write-Host "Done. Start Ternitor from the Start Menu when you want it running."
} else {
    Write-Host "Done. Ternitor is in the notification area; left-click its icon for settings."
}
