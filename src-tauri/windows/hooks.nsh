!macro NSIS_HOOK_PREINSTALL
  ; Tauri currently bundles the main exe but may miss WebView2Loader.dll.
  ; Copy it into the install directory so the app can start on clean machines.
  File "/oname=WebView2Loader.dll" "..\..\WebView2Loader.dll"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\WebView2Loader.dll"
!macroend
