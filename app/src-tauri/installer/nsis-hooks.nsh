; SPDX-License-Identifier: GPL-3.0-or-later
; SPDX-FileCopyrightText: 2026 The Disled contributors
;
; Hooks of the Windows installer, which the NSIS template of Tauri includes
; (bundle > windows > nsis > installerHooks in tauri.conf.json).

; An update from the app starts this installer with /UPDATE, then the app
; exits. Right after this hook, the template asks Restart Manager to shut
; down any process that still uses the app's executable. That can fail on
; an app still on its way out, and the update then stops with "Failed to
; kill Disled" (tauri-apps/tauri#16277, since Tauri CLI 2.12). So an update
; first waits until nothing uses the executable, for 20 s at most. Drop
; this hook once the issue is fixed.
!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
    Push $0
    Push $1
    Push $2
    Push $3
    Push $R8
    Push $R9
    StrCpy $R9 0
    disled_wait_for_exit:
      StrCpy $0 0
      !insertmacro RestartManager_StartSession $R8
      ${If} $R8 != ""
        !insertmacro RestartManager_RegisterFile $R8 "$INSTDIR\${MAINBINARYNAME}.exe"
        ${If} $0 = 0
          ; ERROR_MORE_DATA: a process still uses the executable.
          System::Call 'RSTRTMGR::RmGetList(p R8, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${EndIf}
        !insertmacro RestartManager_EndSession $R8
      ${EndIf}
      ${If} $0 = ${ERROR_MORE_DATA}
      ${AndIf} $R9 < 80
        Sleep 250
        IntOp $R9 $R9 + 1
        Goto disled_wait_for_exit
      ${EndIf}
    Pop $R9
    Pop $R8
    Pop $3
    Pop $2
    Pop $1
    Pop $0
  ${EndIf}
!macroend
