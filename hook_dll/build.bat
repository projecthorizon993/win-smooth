@echo off
if not exist x64\Release\ mkdir x64\Release\
cl.exe /LD src\dllmain.cpp src\hook.cpp src\ipc_pipe.cpp ^
    /Fe x64\Release\explorer_inject.dll ^
    /I src
if errorlevel 1 exit /b 1
echo Build OK
