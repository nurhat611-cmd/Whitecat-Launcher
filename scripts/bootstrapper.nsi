!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "0.3.0"
!endif

!define PRODUCT_NAME "MyLauncher"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\WhitecatLauncher"

Name "${PRODUCT_NAME} ${APP_VERSION}"
OutFile "release-assets\MyLauncher-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Whitecat Launcher"
RequestExecutionLevel user
SetShellVarContext current
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show

!define MUI_ABORTWARNING
!define MUI_ICON "${__FILEDIR__}..\src-tauri\icons\icon.ico"
!define MUI_UNICON "${__FILEDIR__}..\src-tauri\icons\icon.ico"
!define MUI_WELCOMEPAGE_TITLE "Установка MyLauncher"
!define MUI_WELCOMEPAGE_TEXT "Установщик загрузит актуальную версию лаунчера с GitHub Releases.$\r$\n$\r$\nУстановка выполняется для текущего пользователя и не требует прав администратора."
!define MUI_FINISHPAGE_RUN "$INSTDIR\MyLauncher.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Запустить MyLauncher"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"

Section "Install" SEC_INSTALL
  SetOutPath "$TEMP\WhitecatLauncher-Installer"
  File /oname=install-launcher.ps1 "${__FILEDIR__}install-launcher.ps1"

  DetailPrint "Загружаем последнюю версию с GitHub Releases..."
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$TEMP\WhitecatLauncher-Installer\install-launcher.ps1" -InstallDir "$INSTDIR"' $0
  ${If} $0 != 0
    RMDir /r "$TEMP\WhitecatLauncher-Installer"
    MessageBox MB_ICONSTOP|MB_OK "Не удалось установить MyLauncher. Проверьте подключение к интернету и повторите попытку."
    Abort
  ${EndIf}
  RMDir /r "$TEMP\WhitecatLauncher-Installer"

  CreateDirectory "$SMPROGRAMS\Whitecat Launcher"
  CreateShortcut "$SMPROGRAMS\Whitecat Launcher\MyLauncher.lnk" "$INSTDIR\MyLauncher.exe" "" "$INSTDIR\MyLauncher.exe" 0 SW_SHOWNORMAL "" "Запустить MyLauncher"
  CreateShortcut "$DESKTOP\MyLauncher.lnk" "$INSTDIR\MyLauncher.exe" "" "$INSTDIR\MyLauncher.exe" 0 SW_SHOWNORMAL "" "Запустить MyLauncher"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "MyLauncher"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Whitecat Launcher"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\MyLauncher.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" "$\"$INSTDIR\Uninstall.exe$\" /S"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\MyLauncher.lnk"
  Delete "$SMPROGRAMS\Whitecat Launcher\MyLauncher.lnk"
  RMDir "$SMPROGRAMS\Whitecat Launcher"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  Delete "$INSTDIR\MyLauncher.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
