#include "ipc_pipe.h"

static_assert(sizeof(AppMessage) == 64, "AppMessage must be exactly 64 bytes");

const WCHAR PIPE_NAME[] = L"\\\\.\\pipe\\win-smooth-ipc";

static HANDLE g_pipe = INVALID_HANDLE_VALUE;
static CRITICAL_SECTION g_pipe_cs;
static BOOL g_pipe_initialized = FALSE;

// Open the pipe for writing. Returns INVALID_HANDLE_VALUE when the server is
// not running — callers treat this as "send dropped", never fatal.
static HANDLE open_pipe(void) {
    return CreateFileW(
        PIPE_NAME,
        GENERIC_WRITE,
        0,                // no sharing
        NULL,             // default security
        OPEN_EXISTING,
        0,                // synchronous I/O
        NULL);            // no template file
}

void ipc_init(void) {
    if (g_pipe_initialized) return;
    InitializeCriticalSection(&g_pipe_cs);
    g_pipe = INVALID_HANDLE_VALUE;
    g_pipe_initialized = TRUE;
}

void ipc_cleanup(void) {
    if (!g_pipe_initialized) return;
    EnterCriticalSection(&g_pipe_cs);
    if (g_pipe != INVALID_HANDLE_VALUE) {
        CloseHandle(g_pipe);
        g_pipe = INVALID_HANDLE_VALUE;
    }
    LeaveCriticalSection(&g_pipe_cs);
    DeleteCriticalSection(&g_pipe_cs);
    g_pipe_initialized = FALSE;
}

BOOL ipc_send(const AppMessage* msg) {
    if (!msg) return FALSE;
    if (!g_pipe_initialized) return FALSE;

    EnterCriticalSection(&g_pipe_cs);

    // Reopen if handle closed (e.g. server restarted). Fail fast if unavailable.
    if (g_pipe == INVALID_HANDLE_VALUE) {
        g_pipe = open_pipe();
        if (g_pipe == INVALID_HANDLE_VALUE) {
            LeaveCriticalSection(&g_pipe_cs);
            return FALSE; // server not running — drop, do not block explorer
        }
    }

    DWORD written = 0;
    BOOL ok = WriteFile(
        g_pipe,
        msg,
        (DWORD)sizeof(AppMessage),
        &written,
        NULL); // synchronous, no overlapped

    if (!ok || written != (DWORD)sizeof(AppMessage)) {
        // Pipe broken / server gone: drop the handle so next send retries.
        if (g_pipe != INVALID_HANDLE_VALUE) {
            CloseHandle(g_pipe);
            g_pipe = INVALID_HANDLE_VALUE;
        }
        LeaveCriticalSection(&g_pipe_cs);
        return FALSE;
    }

    LeaveCriticalSection(&g_pipe_cs);
    return TRUE;
}
