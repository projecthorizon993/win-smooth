#include <windows.h>

#include "ipc_pipe.h"

// hook.cpp, compiled into the same DLL.
BOOL hook_install(HMODULE hModule);
void hook_uninstall(void);

static HMODULE g_module = NULL;

BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved) {
    switch (ul_reason_for_call) {
    case DLL_PROCESS_ATTACH: {
        g_module = hModule;
        // Avoid thread attach/detach notifications; we handle process lifetime only.
        DisableThreadLibraryCalls(hModule);
        ipc_init();
        // Install the WH_CALLWNDPROC hook into explorer.exe. All threads (id 0).
        if (!hook_install(hModule)) {
            // Non-fatal: shell still works, just no tray/taskbar events.
            hook_uninstall();
        }
        break;
    }
    case DLL_PROCESS_DETACH: {
        // Clean unload: remove hook first, then release pipe resources.
        hook_uninstall();
        ipc_cleanup();
        g_module = NULL;
        break;
    }
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
        break;
    }
    return TRUE;
}
