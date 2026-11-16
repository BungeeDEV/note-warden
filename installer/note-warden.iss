; Inno Setup 6. Build: iscc /DAppVersion=1.1.0 installer\note-warden.iss  (after `cargo build --release`)
; Per-user installation, without administrator rights. User data is stored in
; %LOCALAPPDATA%\Note Warden and is never touched during uninstall.
#ifndef AppVersion
  #define AppVersion "1.1.0"
#endif
#ifndef AppBinary
  #define AppBinary "..\target\release\note-warden.exe"
#endif
#ifndef AppRuntime
  #define AppRuntime "..\target\release\vcruntime140.dll"
#endif

[Setup]
AppId={{6F0B8C52-4E0B-4D5B-9C58-2D7A3B1E9A10}
AppName=Note Warden
AppPublisher=Note Warden
AppComments=Notizen und Updates, lokal und ohne Konto.
DefaultDirName={localappdata}\Programs\Note Warden
DefaultGroupName=Note Warden
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\target\installer
OutputBaseFilename=NoteWarden-{#AppVersion}-setup
SetupIconFile=..\branding\app.ico
UninstallDisplayIcon={app}\note-warden.exe
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no
AppMutex=Local\NoteWarden.SingleInstance
UsePreviousAppDir=yes
UsePreviousTasks=yes
VersionInfoVersion={#AppVersion}
VersionInfoProductVersion={#AppVersion}
AppVersion={#AppVersion}
WizardStyle=modern

[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Messages]
german.FinishedLabel=Note Warden ist installiert. Deine Daten bleiben auch bei einer Deinstallation erhalten.

[Tasks]
Name: "desktopicon"; Description: "Verknüpfung auf dem Desktop anlegen"; Flags: unchecked

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AppRuntime}"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE-LUCIDE.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Note Warden"; Filename: "{app}\note-warden.exe"
Name: "{autodesktop}\Note Warden"; Filename: "{app}\note-warden.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\note-warden.exe"; Description: "Note Warden starten"; Flags: nowait postinstall skipifsilent

; Only an explicit in-app update restarts silently. Interactive installs keep the
; normal checkbox, and ordinary unattended installs never launch the app.
Filename: "{app}\note-warden.exe"; Flags: nowait skipifnotsilent; Check: IsNoteWardenUpdate

[Code]
function IsNoteWardenUpdate: Boolean;
begin
  Result := ExpandConstant('{param:NOTEWARDENUPDATE|0}') = '1';
end;
