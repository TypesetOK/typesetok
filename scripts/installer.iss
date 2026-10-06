; =========================================================================
; Inno Setup Script for TypesetOK (TOK) Desktop Publishing System
; Bilingual Support: Hebrew (עברית) and English
; Warm Book Paper / Canvas Theme (#F4F3EF) - Windows 11 Fluent, No Retro Bevels
; =========================================================================

#define MyAppName "TypesetOK"
; build-installer.mjs passes /DMyAppVersion=<package.json version>
#ifndef MyAppVersion
  #define MyAppVersion "0.9.8"
#endif
#define MyAppPublisher "TypesetOK Team"
#define MyAppURL "https://github.com/TypesetOK/typesetok"
#define MyAppExeName "TypesetOK.exe"
#define SourceDir "..\dist\TypesetOK-v" + MyAppVersion + "-windows-x64"

#ifndef CompressionLevel
  #define CompressionLevel "lzma2/max"
#endif

[Setup]
AppId={{E1B385C9-5D8A-4A73-98FB-364F36AA8C80}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppMutex=TypesetOK_Desktop_App_Mutex

; Destination Directory selection enabled
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE.md
OutputDir=..\dist
OutputBaseFilename=TypesetOK-v{#MyAppVersion}-Setup-x64
Compression={#CompressionLevel}
SolidCompression=yes

; Authentic Warm Paper / Canvas Styling (#F4F3EF - צהבהב/קרם עדין כמו התוכנה)
WizardStyle=modern windows11 hidebevels includetitlebar
WizardBackColor=#F4F3EF
WizardImageBackColor=#F4F3EF
WizardSmallImageBackColor=#F4F3EF
WizardSizePercent=115,115
WizardImageFile=..\assets\installer-sidebar.bmp
WizardSmallImageFile=..\assets\installer-header.bmp
SetupIconFile=..\assets\icon.ico

ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "hebrew"; MessagesFile: "compiler:Languages\Hebrew.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
; Hebrew translations
hebrew.LaunchProgram=הפעל את TypesetOK כעת
hebrew.ShortcutsGroup=קיצורי דרך וסרגל המשימות
hebrew.CreateDesktopIcon=צור קיצור דרך בשולחן העבודה
hebrew.CreateTaskbarIcon=צור קיצור דרך בתפריט התחל והצמד לסרגל המשימות
hebrew.AssociateTok=שייך קובצי מסמך (.tok) ל-TypesetOK
hebrew.AssociateTokBook=שייך קובצי ספר (.tokbook) ל-TypesetOK
hebrew.FileAssociations=שיוך סוגי קבצים
hebrew.ProjectsDirTitle=בחירת תיקיית פרויקטים
hebrew.ProjectsDirSubtitle=בחר את התיקייה שבה ייפתחו ויישמרו כל פרויקטי העימוד שלך כברירת מחדל.
hebrew.ProjectsDirPrompt=תיקיית פרויקטים של TypesetOK:

; English translations
english.LaunchProgram=Launch TypesetOK now
english.ShortcutsGroup=Shortcuts and Taskbar
english.CreateDesktopIcon=Create a desktop shortcut
english.CreateTaskbarIcon=Create Start Menu and Taskbar shortcut
english.AssociateTok=Associate TypesetOK document files (.tok)
english.AssociateTokBook=Associate TypesetOK book files (.tokbook)
english.FileAssociations=File Associations
english.ProjectsDirTitle=Select Projects Directory
english.ProjectsDirSubtitle=Choose the directory where all your typesetting projects will be saved and opened by default.
english.ProjectsDirPrompt=TypesetOK Projects Directory:

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:ShortcutsGroup}"; Flags: unchecked
Name: "taskbaricon"; Description: "{cm:CreateTaskbarIcon}"; GroupDescription: "{cm:ShortcutsGroup}"
Name: "associatetok"; Description: "{cm:AssociateTok}"; GroupDescription: "{cm:FileAssociations}"
Name: "associatetokbook"; Description: "{cm:AssociateTokBook}"; GroupDescription: "{cm:FileAssociations}"

[Files]
; Core application files (always installed)
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

; All Professional Hebrew Fonts (always installed directly to Windows Fonts)
Source: "..\assets\fonts\DavidLibre-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "David Libre"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\FrankRuhlLibre-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Frank Ruhl Libre Regular"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\FrankRuhlLibre-Bold.ttf"; DestDir: "{autofonts}"; FontInstall: "Frank Ruhl Libre Bold"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoRashiHebrew-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Rashi Hebrew Regular"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoRashiHebrew-Bold.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Rashi Hebrew Bold"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoSerifHebrew-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Serif Hebrew Regular"; Flags: onlyifdoesntexist uninsneveruninstall

[Registry]
; Associate .tok document
Root: HKA; Subkey: "Software\Classes\.tok"; ValueType: string; ValueName: ""; ValueData: "TypesetOK.Document"; Flags: uninsdeletevalue; Tasks: associatetok
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document"; ValueType: string; ValueName: ""; ValueData: "TypesetOK Document"; Flags: uninsdeletekey; Tasks: associatetok
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\resources\icon.ico,0"; Tasks: associatetok
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associatetok

; Associate .tokbook multi-document book
Root: HKA; Subkey: "Software\Classes\.tokbook"; ValueType: string; ValueName: ""; ValueData: "TypesetOK.Book"; Flags: uninsdeletevalue; Tasks: associatetokbook
Root: HKA; Subkey: "Software\Classes\TypesetOK.Book"; ValueType: string; ValueName: ""; ValueData: "TypesetOK Multi-Document Book"; Flags: uninsdeletekey; Tasks: associatetokbook
Root: HKA; Subkey: "Software\Classes\TypesetOK.Book\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\resources\icon.ico,0"; Tasks: associatetokbook
Root: HKA; Subkey: "Software\Classes\TypesetOK.Book\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associatetokbook

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Tasks: taskbaricon
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Tasks: taskbaricon
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ProjectsDirPage: TInputDirWizardPage;

procedure InitializeWizard;
begin
  // Set bold modern typography for welcome and finish screens
  WizardForm.WelcomeLabel1.Font.Size := 13;
  WizardForm.WelcomeLabel1.Font.Style := [fsBold];
  WizardForm.FinishedHeadingLabel.Font.Size := 13;
  WizardForm.FinishedHeadingLabel.Font.Style := [fsBold];

  // Projects Directory Selection Page
  ProjectsDirPage := CreateInputDirPage(
    wpSelectDir,
    ExpandConstant('{cm:ProjectsDirTitle}'),
    ExpandConstant('{cm:ProjectsDirSubtitle}'),
    ExpandConstant('{cm:ProjectsDirPrompt}'),
    False,
    ''
  );
  ProjectsDirPage.Add('');
  ProjectsDirPage.Values[0] := ExpandConstant('{userdocs}\TypesetOK Projects');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  SelectedProjectsDir: string;
begin
  if CurStep = ssPostInstall then
  begin
    SelectedProjectsDir := ProjectsDirPage.Values[0];
    if SelectedProjectsDir <> '' then
    begin
      ForceDirectories(SelectedProjectsDir);
      RegWriteStringValue(HKEY_CURRENT_USER, 'Software\TypesetOK', 'ProjectsDir', SelectedProjectsDir);
    end;
    if ActiveLanguage = 'hebrew' then
      RegWriteStringValue(HKEY_CURRENT_USER, 'Software\TypesetOK', 'Language', 'he')
    else
      RegWriteStringValue(HKEY_CURRENT_USER, 'Software\TypesetOK', 'Language', 'en');
  end;
end;
