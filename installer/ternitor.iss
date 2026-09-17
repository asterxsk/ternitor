; Ternitor's installer: the same per-user install as install.ps1, in one
; double-clickable file.
;
; Build it with installer\build.ps1, which passes the version and the icon it
; reads off the built exe -- both come from the app itself, so neither can drift
; from what this ships. Compiling by hand works too:
;
;   iscc /DVersion=1.0.0 installer\ternitor.iss

#ifndef Version
  #define Version "1.0.0"
#endif

; The app to ship, relative to this script: the build that sits in the repo root
; beside install.ps1, not one out of target\release, so the installer and the
; scripts always install the same bytes.
#ifndef AppExe
  #define AppExe "..\ternitor.exe"
#endif

[Setup]
; Per-user, like install.ps1: nothing here needs an administrator, and nothing
; is written outside the user's own profile.
AppId={{7E3E7A6E-2B4C-4C8E-9C6A-9E0B7B0F6D31}
AppName=Ternitor
AppVersion={#Version}
AppVerName=Ternitor {#Version}
AppPublisher=asterxsk
DefaultDirName={localappdata}\Programs\Ternitor
UninstallDisplayName=Ternitor
UninstallDisplayIcon={app}\Ternitor.exe
PrivilegesRequired=lowest
; The install directory is fixed and the shortcut goes straight to the Start
; Menu: there is nothing here for anyone to choose.
DisableDirPage=yes
DisableProgramGroupPage=yes
; The running copy is killed before the copy (see [Code]). Restart Manager would
; only put a "these programs need to close" dialog in front of the thing this app
; exists to keep off the screen.
CloseApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..
OutputBaseFilename=Ternitor-Setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
#ifdef IconPath
SetupIconFile={#IconPath}
#endif

[Tasks]
Name: autostart; Description: "Start Ternitor when Windows starts"; GroupDescription: "Startup:"

[Files]
Source: "{#AppExe}"; DestDir: "{app}"; DestName: "Ternitor.exe"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Ternitor"; Filename: "{app}\Ternitor.exe"; WorkingDir: "{app}"; Comment: "Hide the blank console windows Windows opens for other processes"

[Registry]
; The value the app's own "Start with Windows" switch writes, quoted for the same
; reason: an unquoted path with a space is split at the first one. Declining the
; task takes away a value an earlier install left, rather than leaving a startup
; entry for an app the user just chose not to start.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Ternitor"; ValueData: """{app}\Ternitor.exe"""; Flags: uninsdeletevalue; Tasks: autostart
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Ternitor"; ValueData: ""; Flags: deletevalue; Tasks: not autostart

[Run]
Filename: "{app}\Ternitor.exe"; Description: "Start Ternitor now"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; The log lives beside the exe and is the app's own history. Uninstalling takes
; it with everything else rather than leaving a folder behind.
Type: filesandordirs; Name: "{app}"

[Code]
const
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  ScriptUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Ternitor';

{ A running copy holds the exe open, and a second copy would fight the first over
  the tray icon and the window hook anyway -- the app enforces one instance. }
procedure StopTernitor;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/IM Ternitor.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ What install.ps1 leaves behind: its own entry in Settings > Apps, its
  uninstaller beside the exe, and the Startup .vbs an earlier version used. Left
  alone they would show up as a second Ternitor in that list, or run a script
  that no longer exists at every logon. }
procedure RemoveScriptInstall;
begin
  RegDeleteKeyIncludingSubkeys(HKCU, ScriptUninstallKey);
  DeleteFile(ExpandConstant('{app}\uninstall.ps1'));
  DeleteFile(ExpandConstant('{userstartup}\Ternitor.vbs'));
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
  begin
    StopTernitor;
    RemoveScriptInstall;
  end;
end;

function InitializeUninstall(): Boolean;
begin
  StopTernitor;
  { The app's own switch writes this value after an install that skipped the
    task, so it is taken away here and not only by the entry in [Registry]. }
  RegDeleteValue(HKCU, RunKey, 'Ternitor');
  Result := True;
end;
