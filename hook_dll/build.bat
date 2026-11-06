@echo off
if not exist x64\Release\ mkdir x64\Release\
cl.exe /LD src\dllmain.cpp src\hook.cpp src\ipc_pipe.cpp ^
    /Feexplorer_inject.dll ^
    /I src
if errorlevel 1 exit /b 1
move explorer_inject.dll x64\Release\explorer_inject.dll
echo Build OK
