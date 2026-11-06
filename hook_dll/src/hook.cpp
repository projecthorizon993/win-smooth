#include <windows.h>
#include <string.h>

#include "ipc_pipe.h"

// WH_CALLWNDPROC hook handle, installed into explorer.exe on DLL_PROCESS_ATTACH
// and removed on DLL_PROCESS_DETACH.
static HHOOK g_hook = NULL;

// Target window classes to monitor: the shell tray and the taskbar-switcher window.
static const WCHAR CLASS_TRAY[]      = L"Shell_TrayWnd";
static const WCHAR CLASS_TASKSW[]    = L"MSTaskSwWClass";

// Build a 64-byte AppMessage frame. win_id carries the HWND (truncated to u32).
// For WM_COPYDATA the copy-data payload (bounded to 56 bytes) is carried in payload.
static AppMessage build_message(UINT msg_type, const CWPSTRUCT* cwp) {
    AppMessage m;
    m.msg_type = msg_type;
    m.win_id   = (uint32_t)(UINT_PTR)(cwp ? cwp->hwnd : NULL);
    for (size_t i = 0; i < sizeof(m.payload); i++) m.payload[i] = 0;

    if (cwp && cwp->message == WM_COPYDATA) {
        const COPYDATASTRUCT* cds = (const COPYDATASTRUCT*)cwp->lParam;
        if (cds && cds->cbData > 0 && cds->lpData) {
            size_t copy = cds->cbData;
            if (copy > sizeof(m.payload)) copy = sizeof(m.payload); // validate buffer size
            memcpy_s(m.payload, sizeof(m.payload), cds->lpData, copy);
        }
    }
    return m;
}

static BOOL is_target_class(HWND hwnd) {
    if (!hwnd) return FALSE;
    wchar_t className[256] = {0};
    int cch = GetClassNameW(hwnd, className, 256);
    if (cch <= 0) return FALSE;
    return (lstrcmpiW(className, CLASS_TRAY) == 0) ||
           (lstrcmpiW(className, CLASS_TASKSW) == 0);
}

LRESULT CALLBACK CallWndProc(int nCode, WPARAM wParam, LPARAM lParam) {
    if (nCode == HC_ACTION && lParam) {
        const CWPSTRUCT* cwp = (const CWPSTRUCT*)lParam;
        if (cwp && is_target_class(cwp->hwnd)) {
            UINT msg_type = 0;
            switch (cwp->message) {
            case WM_COPYDATA:
                msg_type = MSG_TRAY_MODIFY;
                break;
            case WM_USER + 79:   // taskbar button overlay/badge notification
            case WM_USER + 85:   // taskbar button overlay/badge notification
                msg_type = MSG_TASKBAR_BUTTON_OVERLAY;
                break;
            default:
                break;
            }
            if (msg_type) {
                AppMessage m = build_message(msg_type, cwp);
                ipc_send(&m); // fail-fast: never blocks or crashes explorer
            }
        }
    }
    return CallNextHookEx(g_hook, nCode, wParam, lParam);
}

BOOL hook_install(HMODULE hModule) {
    if (g_hook) return TRUE; // already installed
    // Thread id 0 hooks all threads in the calling process (explorer.exe).
    g_hook = SetWindowsHookExW(WH_CALLWNDPROC, CallWndProc, hModule, 0);
    return g_hook != NULL;
}

void hook_uninstall(void) {
    if (g_hook) {
        UnhookWindowsHookEx(g_hook);
        g_hook = NULL;
    }
}
