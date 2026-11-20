; =========================================================================
; Inno Setup Script for TypesetOK (TOK) Desktop Publishing System
; Bilingual Support: Hebrew (עברית) and English
; Warm Book Paper / Canvas Theme (#F4F3EF) - Windows 11 Fluent, No Retro Bevels
; =========================================================================

#define MyAppName "TypesetOK"
; build-installer.mjs passes /DMyAppVersion=<package.json version>
#ifndef MyAppVersion
  #define MyAppVersion "0.35.0"
#endif
#define MyAppPublisher "TypesetOK Team"
#define MyAppURL "https://github.com/TypesetOK/typesetok"
#define MyAppExeName "TypesetOK.exe"
#define SourceDir "..\dist\TypesetOK-v" + MyAppVersion + "-windows-x64"

; RuntimeMode: "electron" (legacy) or "tauri" (Stage 4+ Tauri v2 shell)
#ifndef RuntimeMode
  #define RuntimeMode "electron"
#endif

; OfflineWebView2: "0" (standard ~10-20 MB installer) or "1" (bundles MicrosoftEdgeWebView2RuntimeInstallerX64.exe)
#ifndef OfflineWebView2
  #define OfflineWebView2 "0"
#endif

#ifndef WebView2InstallerPath
  #define WebView2InstallerPath "..\assets\redist\MicrosoftEdgeWebView2RuntimeInstallerX64.exe"
#endif

; For Tauri (~17 MB binary), lzma2 produces a ~8-10 MB installer quickly.
; For Electron (168 MB binary), zip extracts about twice as fast as lzma2.
#ifndef CompressionLevel
  #if RuntimeMode == "tauri"
    #define CompressionLevel "lzma2"
  #else
    #define CompressionLevel "zip"
  #endif
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
#if OfflineWebView2 == "1"
  OutputBaseFilename=TypesetOK-v{#MyAppVersion}-Setup-Offline-x64
#else
  OutputBaseFilename=TypesetOK-v{#MyAppVersion}-Setup-x64
#endif
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
hebrew.ProjectsDirTitle=בחירת תיקיית פרויקטים
hebrew.ProjectsDirSubtitle=בחר את התיקייה שבה ייפתחו ויישמרו כל פרויקטי העימוד שלך כברירת מחדל.
hebrew.ProjectsDirPrompt=תיקיית פרויקטים של TypesetOK:
hebrew.WebView2MissingMessage=רכיב Microsoft Edge WebView2 Runtime אינו מותקן במחשב זה.%n%nתוכנת TypesetOK זקוקה לרכיב זה כדי לפעול.%nבמחשבים מנותקי אינטרנט מומלץ להשתמש במתקין האופליין המלא (TypesetOK-v{#MyAppVersion}-Setup-Offline-x64.exe).%n%nהאם להמשיך בהתקנה בכל זאת?
hebrew.InstallingWebView2=מתקין רכיב Microsoft Edge WebView2 Runtime (אופליין)...

; English translations
english.LaunchProgram=Launch TypesetOK now
english.ShortcutsGroup=Shortcuts and Taskbar
english.CreateDesktopIcon=Create a desktop shortcut
english.CreateTaskbarIcon=Create Start Menu and Taskbar shortcut
english.ProjectsDirTitle=Select Projects Directory
english.ProjectsDirSubtitle=Choose the directory where all your typesetting projects will be saved and opened by default.
english.ProjectsDirPrompt=TypesetOK Projects Directory:
english.WebView2MissingMessage=Microsoft Edge WebView2 Runtime is not installed on this computer.%n%nTypesetOK requires this component to run.%nOn offline computers, please use the full Offline Installer (TypesetOK-v{#MyAppVersion}-Setup-Offline-x64.exe).%n%nContinue installation anyway?
english.InstallingWebView2=Installing Microsoft Edge WebView2 Runtime (offline)...

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:ShortcutsGroup}"; Flags: unchecked
Name: "taskbaricon"; Description: "{cm:CreateTaskbarIcon}"; GroupDescription: "{cm:ShortcutsGroup}"

[Files]
; Core application files (always installed)
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

#if OfflineWebView2 == "1"
; Offline Microsoft Edge WebView2 Evergreen Standalone Installer (runs only if missing)
Source: "{#WebView2InstallerPath}"; DestDir: "{tmp}"; DestName: "MicrosoftEdgeWebView2RuntimeInstallerX64.exe"; Flags: deleteafterinstall; Check: NeedsWebView2Install
#endif

; All Professional Hebrew Fonts (always installed directly to Windows Fonts)
Source: "..\assets\fonts\DavidLibre-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "David Libre"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\FrankRuhlLibre-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Frank Ruhl Libre Regular"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\FrankRuhlLibre-Bold.ttf"; DestDir: "{autofonts}"; FontInstall: "Frank Ruhl Libre Bold"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoRashiHebrew-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Rashi Hebrew Regular"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoRashiHebrew-Bold.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Rashi Hebrew Bold"; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\assets\fonts\NotoSerifHebrew-Regular.ttf"; DestDir: "{autofonts}"; FontInstall: "Noto Serif Hebrew Regular"; Flags: onlyifdoesntexist uninsneveruninstall

[Registry]
; Associate .tok document unconditionally (TypesetOK is the native document editor)
Root: HKA; Subkey: "Software\Classes\.tok"; ValueType: string; ValueName: ""; ValueData: "TypesetOK.Document"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document"; ValueType: string; ValueName: ""; ValueData: "TypesetOK Document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\resources\icon.ico,0"
Root: HKA; Subkey: "Software\Classes\TypesetOK.Document\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Tasks: taskbaricon
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Tasks: taskbaricon
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\resources\icon.ico"; Check: DesktopIconCheck

[Run]
#if OfflineWebView2 == "1"
Filename: "{tmp}\MicrosoftEdgeWebView2RuntimeInstallerX64.exe"; Parameters: "/silent /install"; StatusMsg: "{cm:InstallingWebView2}"; Flags: waituntilterminated; Check: NeedsWebView2Install
#endif
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ProjectsDirPage: TInputDirWizardPage;
  IsUpgradeInstall: Boolean;

function IsWebView2Installed: Boolean;
var
  Ver: string;
begin
  Result := False;
  if RegQueryStringValue(HKEY_LOCAL_MACHINE, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Ver) and (Ver <> '') then
    Result := True
  else if RegQueryStringValue(HKEY_LOCAL_MACHINE, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Ver) and (Ver <> '') then
    Result := True
  else if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Ver) and (Ver <> '') then
    Result := True;
end;

function NeedsWebView2Install: Boolean;
begin
  Result := not IsWebView2Installed;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = wpWelcome) and ('{#RuntimeMode}' = 'tauri') and ('{#OfflineWebView2}' <> '1') then
  begin
    if (not IsWebView2Installed) and (not WizardSilent) then
    begin
      if MsgBox(ExpandConstant('{cm:WebView2MissingMessage}'), mbConfirmation, MB_YESNO) = IDNO then
        Result := False;
    end;
  end;
end;

function DetectPreviousInstallation: Boolean;
var
  PrevVer: string;
begin
  Result := False;
  if RegQueryStringValue(HKEY_LOCAL_MACHINE, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting("AppId")}_is1', 'DisplayVersion', PrevVer) then
    Result := True
  else if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting("AppId")}_is1', 'DisplayVersion', PrevVer) then
    Result := True
  else if RegKeyExists(HKEY_LOCAL_MACHINE, 'Software\TypesetOK') or RegKeyExists(HKEY_CURRENT_USER, 'Software\TypesetOK') then
    Result := True
  else if FileExists(ExpandConstant('{autopf}\{#MyAppName}\{#MyAppExeName}')) then
    Result := True;
end;

function DesktopIconCheck: Boolean;
begin
  if IsUpgradeInstall then
    Result := FileExists(ExpandConstant('{autodesktop}\{#MyAppName}.lnk')) or FileExists(ExpandConstant('{userdesktop}\{#MyAppName}.lnk'))
  else
    Result := WizardIsTaskSelected('desktopicon');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
var
  ExistingProjectsDir: string;
begin
  Result := False;
  if IsUpgradeInstall then
  begin
    // In upgrade mode: skip shortcut selection (relies on user choice from initial install)
    if PageID = wpSelectTasks then
      Result := True
    // Skip projects directory selection if already configured in registry
    else if (ProjectsDirPage <> nil) and (PageID = ProjectsDirPage.ID) then
    begin
      if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\TypesetOK', 'ProjectsDir', ExistingProjectsDir) and (ExistingProjectsDir <> '') then
        Result := True;
    end;
  end;
end;

procedure InitializeWizard;
begin
  IsUpgradeInstall := DetectPreviousInstallation;

  // Ensure the license agreement memo is strictly read-only and cannot be typed into
  WizardForm.LicenseMemo.ReadOnly := True;

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
