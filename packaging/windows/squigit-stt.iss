[Setup]
AppName=Squigit STT CLI
AppVersion=0.1.0
AppPublisher=a7mddra
DefaultDirName={autopf}\Squigit\stt
DefaultGroupName=Squigit
OutputBaseFilename=setup_squigit-stt
PrivilegesRequired=admin
ChangesEnvironment=yes
DisableProgramGroupPage=yes
DirExistsWarning=no

[Files]
Source: "..\..\target\release\binaries\whisper-stt-x86_64-pc-windows-msvc\squigit-stt.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\sidecars\whisper-stt\models\*"; DestDir: "{app}\models"; Flags: ignoreversion recursesubdirs createallsubdirs

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
