; SDRCraft v0.1.0 - current-user installer (no admin, no UAC).
; Built with NSIS 3 (makensis installer/installer.nsi, CWD = repo root).
; Installs the egui desktop app + the CLI/MCP server side by side.

!include "MUI2.nsh"

Name "SDRCraft 0.1.0"
OutFile "..\target\release\bundle\SDRCraft-0.1.0-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\sdrcraft"
RequestExecutionLevel user

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Binaries" SecBinaries
  SetOutPath "$INSTDIR"
  File "..\target\release\sdrcraft.exe"
  File "..\target\release\sdrcraft-mcp.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft" \
    "DisplayName" "SDRCraft 0.1.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft" \
    "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft" \
    "DisplayIcon" "$INSTDIR\sdrcraft.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft" \
    "DisplayVersion" "0.1.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft" \
    "Publisher" "sandraschi"
SectionEnd

Section "Start Menu shortcuts" SecShortcuts
  CreateDirectory "$SMPROGRAMS\SDRCraft"
  CreateShortcut "$SMPROGRAMS\SDRCraft\SDRCraft.lnk" "$INSTDIR\sdrcraft.exe"
  CreateShortcut "$SMPROGRAMS\SDRCraft\Uninstall.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section "uninstall"
  Delete "$INSTDIR\sdrcraft.exe"
  Delete "$INSTDIR\sdrcraft-mcp.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\SDRCraft\SDRCraft.lnk"
  Delete "$SMPROGRAMS\SDRCraft\Uninstall.lnk"
  RMDir "$SMPROGRAMS\SDRCraft"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\sdrcraft"
SectionEnd
