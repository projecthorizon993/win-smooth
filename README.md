# win-smooth

Lightweight Windows desktop shell + DWM smoothness tuning.

Rust shell core + C/C++ hook DLL injected into explorer.exe. Runs *with* DWM (never replaces it). Custom Direct2D UI, tiling window management, DWM registry tuning, named-pipe IPC between hook and shell.

Built for Windows 11 24H2. MSVC toolchain. GitHub Actions CI on every push.

## Structure
- `src/` — Rust shell (main.rs, ipc/, shell_ui/, dwm_tuning/)
- `hook_dll/` — C/C++ DLL injected into explorer.exe (dllmain.cpp, hook.cpp, ipc_pipe.cpp)
- `.github/workflows/ci.yml` — MSVC Rust + MSVC C/C++ CI on windows-latest
- `tests/smoke_test.ps1` — CI smoke test
- `scripts/build-all.ps1` / `scripts/install.ps1` — build and install

## Ponytail rules
See `AGENTS.md`. Smallest complete change. No "for later" code.
