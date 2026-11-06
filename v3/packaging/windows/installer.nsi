; Установщик Z.UST для Windows. Сборка:
;   makensis -INPUTCHARSET UTF8 -DVERSION=3.0.0 -DSRC=<папка с Z.UST.exe, zapret, tor, frp> -DOUT=Z.UST-setup.exe installer.nsi
; Ставит в Program Files, создаёт ярлыки, тихо удаляет Zust 2 (настройки остаются).

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel admin

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!define APP "Z.UST"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Z.UST"
; ключ удаления Zust 2 (установщик Tauri, режим "для текущего пользователя")
!define V2_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Zust"

Name "${APP}"
OutFile "${OUT}"
InstallDir "$PROGRAMFILES64\${APP}"
InstallDirRegKey HKLM "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP} ${VERSION}"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "Установка ${APP}"
VIAddVersionKey "CompanyName" "f1zzen"
VIAddVersionKey "LegalCopyright" "f1zzen"

!define MUI_ICON "${SRC}\icon.ico"
!define MUI_UNICON "${SRC}\icon.ico"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_NODESC
!define MUI_FINISHPAGE_RUN "$INSTDIR\Z.UST.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Запустить ${APP}"

!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"

!macro CloseApp
  nsExec::Exec 'taskkill /F /T /IM Z.UST.exe'
  nsExec::Exec 'taskkill /F /T /IM Zust.exe'
  Sleep 500
!macroend

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "${APP} работает только на 64-битной Windows."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

; Тихо удалить Zust 2: его uninstall.exe с /S не трогает настройки,
; а они у Z.UST общие (%APPDATA%\com.f1zzen.zust).
Function RemoveV2
  ReadRegStr $0 HKCU "${V2_KEY}" "UninstallString"
  ${If} $0 == ""
    ReadRegStr $0 HKLM "${V2_KEY}" "UninstallString"
  ${EndIf}
  ${If} $0 == ""
    Return
  ${EndIf}
  ; "C:\...\uninstall.exe" -> без кавычек
  StrCpy $1 $0 1
  ${If} $1 == '"'
    StrCpy $0 $0 "" 1
    StrCpy $0 $0 -1
  ${EndIf}
  ${GetParent} $0 $1
  DetailPrint "Удаление Zust 2 из $1"
  ${If} ${FileExists} $0
    ; _?= - ждать завершения, а не копию во временной папке
    ExecWait '"$0" /S _?=$1'
    Delete $0
    RMDir $1
  ${EndIf}
  DeleteRegKey HKCU "${V2_KEY}"
  ; старый автозапуск Zust 2 указывал на удалённый exe
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "zust"
FunctionEnd

Section "${APP}" SecApp
  SectionIn RO
  !insertmacro CloseApp
  Call RemoveV2

  SetOutPath "$INSTDIR"
  File "${SRC}\Z.UST.exe"
  File "${SRC}\icon.ico"
  ; ресурсы обновляем целиком, чтобы не оставались удалённые стратегии
  RMDir /r "$INSTDIR\zapret"
  RMDir /r "$INSTDIR\tor"
  RMDir /r "$INSTDIR\frp"
  File /r "${SRC}\zapret"
  File /r "${SRC}\tor"
  File /r "${SRC}\frp"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  ; ссылки zust://, в том числе поверх записи Zust 2
  WriteRegStr HKCU "Software\Classes\zust" "" "URL:Z.UST Protocol"
  WriteRegStr HKCU "Software\Classes\zust" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\zust\DefaultIcon" "" '"$INSTDIR\Z.UST.exe",0'
  WriteRegStr HKCU "Software\Classes\zust\shell\open\command" "" '"$INSTDIR\Z.UST.exe" "%1"'
  SetShellVarContext all
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\Z.UST.exe" "" "$INSTDIR\icon.ico"

  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "f1zzen"
  WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "https://github.com/f1zzen/zust"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" $0
SectionEnd

Section "Ярлык на рабочем столе" SecDesktop
  SetShellVarContext current
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\Z.UST.exe" "" "$INSTDIR\icon.ico"
SectionEnd

Section "Uninstall"
  !insertmacro CloseApp
  ; zapret работает службой и без окна Z.UST - останавливаем
  nsExec::Exec 'net stop zapret'
  nsExec::Exec 'sc delete zapret'
  nsExec::Exec 'taskkill /F /IM winws.exe'
  nsExec::Exec 'net stop WinDivert'
  nsExec::Exec 'sc delete WinDivert'
  nsExec::Exec 'schtasks /Delete /TN Zust /F'
  DeleteRegKey HKCR "zust"
  DeleteRegKey HKCU "Software\Classes\zust"

  SetShellVarContext all
  Delete "$SMPROGRAMS\${APP}.lnk"
  SetShellVarContext current
  Delete "$DESKTOP\${APP}.lnk"

  RMDir /r "$INSTDIR\zapret"
  RMDir /r "$INSTDIR\tor"
  RMDir /r "$INSTDIR\frp"
  Delete "$INSTDIR\Z.UST.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINST_KEY}"

  IfSilent done
  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "Удалить и настройки Z.UST (стратегии, списки, лог)?" IDNO done
  SetShellVarContext current
  RMDir /r "$APPDATA\com.f1zzen.zust"
  done:
SectionEnd
