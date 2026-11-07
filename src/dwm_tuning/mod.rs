// DWM registry tuning + process priority
// shortcut: only reads/writes keys proven to affect smoothness on Win 11 24H2;
// upgrade when Windows changes DWM internals.

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;
use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::Foundation::{DWORD, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemServices::SetPriorityClass;
use windows::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, CloseHandle, OpenProcess, PROCESS_QUERY_INFORMATION,
    PROCESS_SET_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_SETCOMBOBOXANIMATION, SPI_SETANIMATION, SPI_SETDRAGFULLWINDOWS,
    SPI_SETGRADIENTCAPTIONS, SPI_SETSELECTIONFADE, SPI_SETUIEFFECTS,
    SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
};

/// DWM registry key path
const DWM_REG_KEY: &str = r"SOFTWARE\Microsoft\Windows\DWM";

/// Desktop animation registry key path
const DESKTOP_REG_KEY: &str = r"Control Panel\Desktop";

/// Apply all DWM smoothness tunings.
pub fn apply_dwm_tuning() -> Result<()> {
    // 1. DWM registry tweaks
    apply_dwm_registry()?;

    // 2. Desktop animation tweaks
    apply_desktop_animations()?;

    // 3. Set dwm.exe process priority
    set_dwm_priority()?;

    Ok(())
}

/// Tweak DWM registry keys: disable blur/acrylic, force flip model.
fn apply_dwm_registry() -> Result<()> {
    let hkml = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (dwm_key, _) = hkml.create_subkey(DWM_REG_KEY).context("create DWM reg key")?;

    // DisableBlurBehindWindow: 0 = disable blur behind windows (reduces compositing cost)
    dwm_key
        .set_value("DisableBlurBehindWindow", &0u32)
        .context("set DisableBlurBehindWindow")?;

    // ForceUseCompatAltTab: not relevant for smoothness, skip

    // MultiplaneOverlay (MPO): 0 = disable MPO on some GPU configs that cause stutter
    dwm_key
        .set_value("OverlayTestMode", &0u32)
        .context("set OverlayTestMode")?;

    Ok(())
}

/// Disable desktop animations for snappier UI.
fn apply_desktop_animations() -> Result<()> {
    unsafe {
        // SPI_SETANIMATION: minimize/restore animation
        SystemParametersInfoW(SPI_SETANIMATION, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
        // SPI_SETCOMBOBOXANIMATION
        SystemParametersInfoW(SPI_SETCOMBOBOXANIMATION, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
        // SPI_SETDRAGFULLWINDOWS: drag full windows (not outline)
        SystemParametersInfoW(SPI_SETDRAGFULLWINDOWS, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
        // SPI_SETGRADIENTCAPTIONS: gradient title bars
        SystemParametersInfoW(SPI_SETGRADIENTCAPTIONS, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
        // SPI_SETSELECTIONFADE: selection fade effect
        SystemParametersInfoW(SPI_SETSELECTIONFADE, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
        // SPI_SETUIEFFECTS: UI effects (shadows, etc.)
        SystemParametersInfoW(SPI_SETUIEFFECTS, 0, std::ptr::null_mut(), SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()?;
    }
    Ok(())
}

/// Find dwm.exe PID and set its priority class to ABOVE_NORMAL.
fn set_dwm_priority() -> Result<()> {
    let snapshot = unsafe {
        CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .context("CreateToolhelp32Snapshot")?
    };

    unsafe {
        let mut entry: PROCESSENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry).is_ok() {
            loop {
                let exe_name = String::from_utf16_lossy(
                    entry.szExeFile.iter().take_while(|&&c| c != 0),
                );
                if exe_name.eq_ignore_ascii_case("dwm.exe") {
                    let pid = entry.th32ProcessID;

                    let hproc = OpenProcess(
                        PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION,
                        false,
                        pid,
                    )?;

                    SetPriorityClass(hproc, ABOVE_NORMAL_PRIORITY_CLASS)
                        .ok()
                        .context("SetPriorityClass on dwm.exe")?;

                    CloseHandle(hproc).ok();
                    break;
                }

                if !Process32Next(snapshot, &mut entry).is_ok() {
                    break;
                }
            }
        }
    }

    CloseHandle(snapshot).ok();
    Ok(())
}
