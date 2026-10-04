!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\windows-cli.ps1" -Action Install -InstallDirectory "$INSTDIR"'
  Pop $0
  Pop $1
  ${If} $0 != 0
    DetailPrint "$1"
    Abort "AgentHub command registration failed. See the installation details."
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Reinstall/uninstall removes program files only; library and local data survive.
  StrCpy $DeleteAppDataCheckboxState 0
  ; A cancelled uninstall must leave command registration intact.
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\windows-cli.ps1" -Action Uninstall -InstallDirectory "$INSTDIR"'
  Pop $0
  Pop $1
  ${If} $0 != 0
    DetailPrint "$1"
    Abort "AgentHub command removal failed. See the uninstall details."
  ${EndIf}
!macroend
