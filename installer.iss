[Setup]
AppName=Side Language
AppVersion=1.2.0
AppPublisher=Side Team
DefaultDirName={pf}\Side
DefaultGroupName=Side
UninstallDisplayIcon={app}\side.exe
Compression=lzma2
SolidCompression=yes
OutputDir=.
OutputBaseFilename=Side_Setup
SetupIconFile=assets\side.ico
WizardStyle=modern

[Files]
; Главный исполняемый файл
Source: "side.exe"; DestDir: "{app}"; Flags: ignoreversion

; Лицензия (в корень!)
Source: "LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion

; Иконки
Source: "assets\side.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "assets\sd.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "assets\spack.ico"; DestDir: "{app}\assets"; Flags: ignoreversion

; Документация
Source: "documentation\*"; DestDir: "{app}\documentation"; Flags: recursesubdirs ignoreversion

; Примеры
Source: "examples\*"; DestDir: "{app}\examples"; Flags: recursesubdirs ignoreversion

; Тесты (опционально)
Source: "tests\*"; DestDir: "{app}\tests"; Flags: recursesubdirs ignoreversion

[Registry]
; .sd файлы
Root: HKCR; Subkey: ".sd"; ValueType: string; ValueName: ""; ValueData: "SideFile"; Flags: uninsdeletevalue
Root: HKCR; Subkey: "SideFile"; ValueType: string; ValueName: ""; ValueData: "Side Script File"; Flags: uninsdeletekey
Root: HKCR; Subkey: "SideFile\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\assets\sd.ico"; Flags: uninsdeletekey
Root: HKCR; Subkey: "SideFile\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\side.exe"" ""%1"""; Flags: uninsdeletekey

; .spack файлы
Root: HKCR; Subkey: ".spack"; ValueType: string; ValueName: ""; ValueData: "SpackFile"; Flags: uninsdeletevalue
Root: HKCR; Subkey: "SpackFile"; ValueType: string; ValueName: ""; ValueData: "Spack Project File"; Flags: uninsdeletekey
Root: HKCR; Subkey: "SpackFile\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\assets\spack.ico"; Flags: uninsdeletekey
Root: HKCR; Subkey: "SpackFile\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """notepad.exe"" ""%1"""; Flags: uninsdeletekey

; Добавление в PATH
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "PATH"; ValueData: "{olddata};{app}"; Check: NeedsAddPath('{app}')

[Icons]
Name: "{group}\Side Interpreter"; Filename: "{app}\side.exe"; IconFilename: "{app}\side.exe"
Name: "{group}\Examples"; Filename: "{app}\examples"
Name: "{group}\Documentation"; Filename: "{app}\docs"
Name: "{group}\Tests"; Filename: "{app}\tests"
Name: "{group}\Uninstall Side"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Side"; Filename: "{app}\side.exe"; Tasks: desktopicon

[Tasks]
Name: desktopicon; Description: "Create desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'PATH', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;