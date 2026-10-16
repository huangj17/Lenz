; -*- coding: utf-8 -*-
; Standalone current-user installation. MSIX and certificate trust are separate optional setup.
Unicode true
RequestExecutionLevel user
SetCompressor /SOLID lzma
Name "Lenz ${DISPLAY_VERSION}"
OutFile "${OUTPUT_PATH}"
InstallDir "$LOCALAPPDATA\Programs\Lenz"
InstallDirRegKey HKCU "Software\LenzDesktop" "InstallDir"
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
VIProductVersion "${VERSION}"
VIAddVersionKey /LANG=1033 "ProductName" "Lenz Setup"
VIAddVersionKey /LANG=1033 "FileDescription" "Lenz desktop installer"
VIAddVersionKey /LANG=1033 "FileVersion" "${DISPLAY_VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Lenz contributors"

!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"

Var PowerShell
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\LenzDesktop"
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TEXT "$(WelcomeText)"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PAYLOAD_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_TEXT "$(FinishedText)"
!define MUI_FINISHPAGE_RUN "$INSTDIR\lenz.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "--show-main"
!define MUI_FINISHPAGE_RUN_TEXT "$(LaunchText)"
!define MUI_FINISHPAGE_LINK "$(SystemOcrLink)"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/huangj17/Lenz/releases"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

LangString WelcomeText ${LANG_ENGLISH} "Install Lenz for your Windows account. You do not need to import a certificate.$\r$\n$\r$\nAfter installation, add Tencent in Settings > Services > OCR and set it as the default. Local OCR language packs are not required.$\r$\n$\r$\nIf WebView2 is missing, setup downloads and installs the official Microsoft runtime."
LangString WelcomeText ${LANG_SIMPCHINESE} "为当前 Windows 用户安装 Lenz，无需手动导入证书。$\r$\n$\r$\n安装后进入「设置 → 服务 → OCR」添加腾讯云并设为默认即可，无需安装本地 OCR 语言包。$\r$\n$\r$\n若缺少 WebView2，向导将下载并安装微软官方运行环境。"
LangString FinishedText ${LANG_ENGLISH} "Lenz is installed. Configure Tencent or another OCR service in Settings > Services > OCR.$\r$\n$\r$\nWindows system OCR is optional. If you need it later, switch to the MSIX version and complete its certificate and language setup. Importing a certificate alone does not enable system OCR in this EXE version."
LangString FinishedText ${LANG_SIMPCHINESE} "Lenz 已安装，可在「设置 → 服务 → OCR」配置并选择腾讯云等服务。$\r$\n$\r$\nWindows 系统 OCR 为可选功能。以后需要时，再切换到 MSIX 版本并完成证书和语言包设置。仅导入证书不能让此 EXE 版本启用系统 OCR。"
LangString LaunchText ${LANG_ENGLISH} "Open Lenz"
LangString LaunchText ${LANG_SIMPCHINESE} "打开 Lenz"
LangString SystemOcrLink ${LANG_ENGLISH} "Optional: download the MSIX version for Windows system OCR"
LangString SystemOcrLink ${LANG_SIMPCHINESE} "以后需要系统 OCR 时，可选下载 MSIX 版本"
LangString InstallFailed ${LANG_ENGLISH} "Installation did not finish. Check the details above, close Lenz if it is running, and retry. No certificate import or OCR setup is required for this installer."
LangString InstallFailed ${LANG_SIMPCHINESE} "安装未完成，请查看上方详情；如果 Lenz 正在运行，请退出后重试。此安装包无需导入证书或配置本地 OCR。"
LangString CloseLenz ${LANG_ENGLISH} "Please quit Lenz from its tray menu before continuing."
LangString CloseLenz ${LANG_SIMPCHINESE} "请先从托盘菜单退出正在运行的 Lenz，再继续。"
LangString UnsupportedSystem ${LANG_ENGLISH} "This Lenz installer requires Windows 10 1809 or later with a compatible 64-bit CPU."
LangString UnsupportedSystem ${LANG_SIMPCHINESE} "此 Lenz 安装包需要 Windows 10 1809 或更高版本，以及兼容的 64 位处理器。"

Function .onInit
  ${IfNot} ${RunningX64}
  ${OrIfNot} ${AtLeastWin10}
  ${OrIfNot} ${AtLeastBuild} 17763
    MessageBox MB_OK|MB_ICONSTOP "$(UnsupportedSystem)"
    SetErrorLevel 2
    Quit
  ${EndIf}
  !if "${ARCHITECTURE}" == "arm64"
    ${IfNot} ${IsNativeARM64}
      MessageBox MB_OK|MB_ICONSTOP "$(UnsupportedSystem)"
      SetErrorLevel 2
      Quit
    ${EndIf}
  !endif
  SetRegView 64
  SetShellVarContext current
  StrCpy $PowerShell "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
FunctionEnd

!macro RequireClosedApp
  check_app:
  IfFileExists "$INSTDIR\lenz.exe" 0 app_closed
  ClearErrors
  FileOpen $0 "$INSTDIR\lenz.exe" a
  ${If} ${Errors}
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "$(CloseLenz)" IDRETRY check_app
    SetErrorLevel 1
    Abort
  ${EndIf}
  FileClose $0
  app_closed:
!macroend

Section "Install Lenz"
  SetDetailsView show
  !insertmacro RequireClosedApp
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "${PAYLOAD_DIR}\Install-WebView2.ps1"
  File "${PAYLOAD_DIR}\MsixTools.ps1"
  ; Only the shared WebView2 helper is invoked. No MSIX deployment, certificate or OCR step.
  nsExec::ExecToLog '"$PowerShell" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\Install-WebView2.ps1"'
  Pop $0
  ${If} $0 != 0
    DetailPrint "$(InstallFailed)"
    SetErrorLevel 1
    Abort
  ${EndIf}
  SetOutPath "$INSTDIR"
  ClearErrors
  SetOverwrite on
  File "${PAYLOAD_DIR}\lenz.exe"
  File "${PAYLOAD_DIR}\LICENSE"
  !ifdef HAS_LOADER
    File "${PAYLOAD_DIR}\WebView2Loader.dll"
  !endif
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  ${If} ${Errors}
    DetailPrint "$(InstallFailed)"
    SetErrorLevel 1
    Abort
  ${EndIf}
  CreateShortcut "$SMPROGRAMS\Lenz.lnk" "$INSTDIR\lenz.exe" "--show-main"
  WriteRegStr HKCU "Software\LenzDesktop" "InstallDir" "$INSTDIR"
  WriteRegDWORD HKCU "Software\LenzDesktop" "InstallerLanguage" $LANGUAGE
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "Lenz (EXE)"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${DISPLAY_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Lenz contributors"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\lenz.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${If} ${Errors}
    DetailPrint "$(InstallFailed)"
    SetErrorLevel 1
    Abort
  ${EndIf}
SectionEnd

Function un.onInit
  SetRegView 64
  SetShellVarContext current
  ClearErrors
  ReadRegDWORD $0 HKCU "Software\LenzDesktop" "InstallerLanguage"
  ${IfNot} ${Errors}
    StrCpy $LANGUAGE $0
  ${EndIf}
FunctionEnd

Section "Uninstall"
  !insertmacro RequireClosedApp
  ClearErrors
  Delete "$INSTDIR\lenz.exe"
  ${If} ${Errors}
    SetErrorLevel 1
    Abort
  ${EndIf}
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$SMPROGRAMS\Lenz.lnk"
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Lenz"
  ${If} $0 == '"$INSTDIR\lenz.exe"'
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Lenz"
  ${EndIf}
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "Software\LenzDesktop"
  ; Remove only files owned by the installer; preserve user configuration and unrelated files.
  SetOutPath "$TEMP"
  RMDir "$INSTDIR"
SectionEnd
