#define MyAppName "GitSearcher"
#define MyAppVersion "0.1.1"
#define MyAppPublisher "Parresia"
#define MyAppURL "https://github.com/elguala9/GitSearcher"
#define MyAppExeName "gitsearcher-ui.exe"
#define MyCliExeName "GitSearcher.exe"

[Setup]
AppId={{B4F2E3A1-7C8D-4F5E-9A2B-1D3C6E8F0A4B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\installer-output
OutputBaseFilename=GitSearcher-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
ChangesEnvironment=yes

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "addtopath"; Description: "Aggiungi GitSearcher al PATH (per uso da terminale)"; GroupDescription: "Opzioni:"; Flags: unchecked

[Files]
Source: "..\publish\win-x64\{#MyCliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\ui\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "default-config.toml"; DestDir: "{app}"; DestName: "gitsearcher-ui.toml"; Flags: ignoreversion onlyifdoesntexist

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Disinstalla {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Crea un'icona sul Desktop"; GroupDescription: "Icone aggiuntive:"; Flags: unchecked

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; \
  ValueData: "{olddata};{app}"; \
  Check: NeedsAddPath(ExpandConstant('{app}')); Tasks: addtopath

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;
