@echo off
cl.exe /LD src\dllmain.cpp src\hook.cpp src\ipc_pipe.cpp ^
    /Fe x64\Release\explorer_inject.dll ^
    /I src ^
    /link /OUT:x64\Release\explorer_inject.dll
if errorlevel 1 exit /b 1
echo Build OK
