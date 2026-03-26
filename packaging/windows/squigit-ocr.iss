[Setup]
AppName=Squigit OCR CLI
AppVersion=0.1.0
AppPublisher=a7mddra
DefaultDirName={autopf}\Squigit\ocr
DefaultGroupName=Squigit
OutputBaseFilename=setup_squigit-ocr
PrivilegesRequired=admin
ChangesEnvironment=yes
DisableProgramGroupPage=yes
DirExistsWarning=no

[Files]
Source: "..\..\target\release\binaries\paddle-ocr-x86_64-pc-windows-msvc\squigit-ocr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\binaries\paddle-ocr-x86_64-pc-windows-msvc\_internal\*"; DestDir: "{app}\_internal"; Flags: ignoreversion recursesubdirs createallsubdirs

[Code]
const
  EnvironmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';

procedure CurStepChanged(CurStep: TSetupStep);
var
  Paths: string;
begin
  if CurStep = ssPostInstall then
  begin
    if RegQueryStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', Paths) then
    begin
      if Pos(';' + ExpandConstant('{app}') + ';', ';' + Paths + ';') = 0 then
      begin
        Paths := Paths + ';' + ExpandConstant('{app}');
        RegWriteStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', Paths);
      end;
    end;
  end;
end;
