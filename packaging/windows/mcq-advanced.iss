; Windows installer of the two desktop programs: the Studio (teacher) and the player (student).
; Built by tools/package-windows.ps1, which passes the version: ISCC /DAppVersion=x.y.z
; Per-user install, no administrator rights. Keep this file pure ASCII.
#ifndef AppVersion
  #error Pass the version: ISCC /DAppVersion=x.y.z
#endif
[Setup]
AppId={{5B0F3C8E-2D4A-4E61-9B7A-6C1E8F2A9D43}
AppName=MCQ Advanced
AppVersion={#AppVersion}
AppPublisher=Mickael Mancini
AppPublisherURL=https://mickaelmancini.com/mcq-advanced/
AppSupportURL=https://github.com/Chmyke/mcq-advanced
DefaultDirName={localappdata}\Programs\MCQ Advanced
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
DisableReadyPage=yes
UninstallDisplayIcon={app}\mcq-studio.exe
Compression=lzma2
SolidCompression=yes
OutputDir=..\..\target\dist
OutputBaseFilename=MCQ-Advanced-Setup-{#AppVersion}
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
LicenseFile=..\..\LICENSE
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
[Files]
Source: "..\..\target\release\mcq-studio.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\mcq-player.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
[Icons]
Name: "{autoprograms}\MCQ Advanced Studio"; Filename: "{app}\mcq-studio.exe"
Name: "{autoprograms}\MCQ Advanced Player"; Filename: "{app}\mcq-player.exe"
Name: "{autodesktop}\MCQ Advanced Studio"; Filename: "{app}\mcq-studio.exe"; Tasks: desktopicon
Name: "{autodesktop}\MCQ Advanced Player"; Filename: "{app}\mcq-player.exe"; Tasks: desktopicon
