<#
.SYNOPSIS
    Removes Ternitor.

.DESCRIPTION
    Stops the app and takes away everything the install put down: the exe and
    its folder, the Start Menu shortcut, the Run entry (only if it points at this
    install), and the uninstall entry itself -- so an uninstalled Ternitor stops
    appearing in Settings > Apps and stops starting at sign-in.

    The log is the one thing that is not obviously the installer's to delete, so
    it asks. -Quiet answers for it, which is what Settings > Apps uses.

    Nothing here needs administrator rights.

.PARAMETER InstallDir
    The folder to remove. Defaults to the folder this script is running from,
    which is where the installer puts it.

.PARAMETER Quiet
    No prompts: remove the log too.

.PARAMETER KeepLog
    No prompts: keep the log.

.EXAMPLE
    .\uninstall.ps1
#>
[CmdletBinding()]
param(
    [string]$InstallDir = $PSScriptRoot,
    [switch]$Quiet,
    [switch]$KeepLog
)

$ErrorActionPreference = 'Stop'

$AppName = 'Ternitor'
$Exe = Join-Path $InstallDir "$AppName.exe"
$Log = Join-Path $InstallDir "$AppName.log"
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"
$Shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$AppName.lnk"

function Write-Step($Text) { Write-Host "  $Text" }

# The next act is a recursive force-delete of $InstallDir, and the default is the
# folder this script is running from -- which, in a clone of the repository, is
# the repository. So it refuses to touch anything that is not a real Ternitor
# install. No Ternitor.exe, no deletion: the cost of refusing wrongly is that
# someone deletes a folder by hand.
if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw "Refusing to uninstall: there is no Ternitor.exe in '$InstallDir'. Pass -InstallDir with the folder install.ps1 created, or delete the folder yourself."
}

Write-Host "Removing $AppName"

Get-Process -Name $AppName -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Step "stopping $AppName (pid $($_.Id))"
    Stop-Process -Id $_.Id -Force
    $_.WaitForExit()
}

# Only the entry that points here. A Run value aimed at a development build is
# that build's autostart, and removing it would silently break it.
$run = (Get-ItemProperty -Path $RunKey -Name $AppName -ErrorAction SilentlyContinue).$AppName
if ($run) {
    if ($run.Trim('"') -ieq $Exe) {
        Remove-ItemProperty -Path $RunKey -Name $AppName
        Write-Step 'removed the start-with-Windows entry'
    } else {
        Write-Step "left the start-with-Windows entry alone: it points at $run"
    }
}

if (Test-Path -LiteralPath $Shortcut) {
    Remove-Item -LiteralPath $Shortcut -Force
    Write-Step 'removed the Start Menu shortcut'
}

if (Test-Path -LiteralPath $UninstallKey) {
    Remove-Item -Path $UninstallKey -Recurse -Force
    Write-Step 'removed the Settings > Apps entry'
}

$dropLog = $false
if (Test-Path -LiteralPath $Log) {
    if ($KeepLog) {
        Write-Step "kept $Log"
    } elseif ($Quiet) {
        $dropLog = $true
    } else {
        $answer = Read-Host "  Delete the log at $Log too? (y/N)"
        $dropLog = $answer -match '^(y|yes)$'
        if (-not $dropLog) { Write-Step "kept $Log" }
    }
}

# Everything else goes. This script is running from the folder it is deleting,
# which works because PowerShell has already read it.
Get-ChildItem -LiteralPath $InstallDir -Force -ErrorAction SilentlyContinue | ForEach-Object {
    if ($_.Name -eq "$AppName.log" -and -not $dropLog) { return }
    Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
}

# The folder only goes if the loop actually emptied it. Remove-Item cannot delete
# a directory that still holds a file -- it throws a null reference rather than
# reporting the reason -- so with -KeepLog, or with something still open, the
# attempt is skipped and what remains is named instead of crashing the summary.
$held = @(
    Get-ChildItem -LiteralPath $InstallDir -Force -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Name
)
if ($held.Count -eq 0) {
    Remove-Item -LiteralPath $InstallDir -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $InstallDir) {
    Write-Host "Left behind in ${InstallDir}: $($held -join ', ')"
    Write-Host 'Close anything using those files and delete the folder.'
} else {
    Write-Step "removed $InstallDir"
}

Write-Host "Done. $AppName is uninstalled."
