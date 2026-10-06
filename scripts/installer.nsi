; NSIS Modern User Interface
; Multilingual TypesetOK Installer (Hebrew & English)

!include "MUI2.nsh"
!include "FileFunc.nsh"

; build-installer.mjs passes /DVERSION=<package.json version>
!ifndef VERSION
  !define VERSION "0.9.8"
!endif

Name "TypesetOK"
OutFile "..\dist\TypesetOK-v${VERSION}-NSIS-Setup.exe"
InstallDir "$PROGRAMFILES64\TypesetOK"
InstallDirRegKey HKLM "Software\TypesetOK" "Install_Dir"
RequestExecutionLevel admin

; Modern Interface & Branding Settings
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"
!define MUI_WELCOMEFINISHPAGE_BITMAP "..\assets\installer-sidebar.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "..\assets\installer-sidebar.bmp"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "..\assets\installer-header.bmp"
!define MUI_HEADERIMAGE_RIGHT

; Language Selection Dialog
!define MUI_LANGDLL_REGISTRY_ROOT "HKLM"
!define MUI_LANGDLL_REGISTRY_KEY "Software\TypesetOK"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE.md"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\TypesetOK.exe"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Languages (Hebrew and English)
!insertmacro MUI_LANGUAGE "Hebrew"
!insertmacro MUI_LANGUAGE "English"

; Setup functions
Function .onInit
  !insertmacro MUI_LANGDLL_DISPLAY
FunctionEnd

Section "TypesetOK Application (Required)" SecApp
  SectionIn RO
  SetOutPath "$INSTDIR"
  File /r "..\dist\TypesetOK-v${VERSION}-windows-x64\*.*"

  ; Create shortcuts
  CreateDirectory "$SMPROGRAMS\TypesetOK"
  CreateShortcut "$SMPROGRAMS\TypesetOK\TypesetOK.lnk" "$INSTDIR\TypesetOK.exe"
  CreateShortcut "$SMPROGRAMS\TypesetOK\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\TypesetOK.lnk" "$INSTDIR\TypesetOK.exe"

  ; File Associations (.tok and .tokbook)
  WriteRegStr HKCR ".tok" "" "TypesetOK.Document"
  WriteRegStr HKCR "TypesetOK.Document" "" "TypesetOK Document"
  WriteRegStr HKCR "TypesetOK.Document\shell\open\command" "" '"$INSTDIR\TypesetOK.exe" "%1"'
  
  WriteRegStr HKCR ".tokbook" "" "TypesetOK.Book"
  WriteRegStr HKCR "TypesetOK.Book" "" "TypesetOK Multi-Document Book"
  WriteRegStr HKCR "TypesetOK.Book\shell\open\command" "" '"$INSTDIR\TypesetOK.exe" "%1"'

  ; Write Uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK" "DisplayName" "TypesetOK (TOK)"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK" "DisplayIcon" '"$INSTDIR\TypesetOK.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK" "Publisher" "TypesetOK Team"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK" "DisplayVersion" "${VERSION}"
SectionEnd

Section "Uninstall"
  RMDir /r "$INSTDIR"
  Delete "$DESKTOP\TypesetOK.lnk"
  RMDir /r "$SMPROGRAMS\TypesetOK"
  DeleteRegKey HKCR ".tok"
  DeleteRegKey HKCR "TypesetOK.Document"
  DeleteRegKey HKCR ".tokbook"
  DeleteRegKey HKCR "TypesetOK.Book"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\TypesetOK"
SectionEnd
