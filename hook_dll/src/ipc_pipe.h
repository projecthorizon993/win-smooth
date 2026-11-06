#pragma once

#include <windows.h>
#include <stdint.h>

// Shared binary schema with Rust ipc::AppMessage.
//   u32 msg_type; u32 win_id; u8 payload[56];  == 64 bytes, naturally aligned.
#pragma pack(push, 1)
typedef struct AppMessage {
    uint32_t msg_type;
    uint32_t win_id;
    uint8_t  payload[56];
} AppMessage;
#pragma pack(pop)

// Message types — must match hook_dll AppMessage.msg_type (Rust src/ipc/mod.rs).
#define MSG_TRAY_ADD              1u
#define MSG_TRAY_MODIFY           2u
#define MSG_TRAY_DELETE           3u
#define MSG_TASKBAR_BUTTON_OVERLAY 4u
#define MSG_QUIT                  5u

// Pipe name — must match Rust PIPE_NAME (\\.\pipe\win-smooth-ipc).
extern const WCHAR PIPE_NAME[];

// Initialize pipe client state. Safe to call from DllMain.
void ipc_init(void);

// Clean up pipe client state. Safe to call from DllMain (process detach).
void ipc_cleanup(void);

// Send a single 64-byte AppMessage frame. Fail-fast: returns FALSE if the
// pipe is unavailable (server not running) without blocking the caller.
BOOL ipc_send(const AppMessage* msg);
