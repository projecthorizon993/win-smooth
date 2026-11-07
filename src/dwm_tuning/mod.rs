// DWM registry tuning + process priority
// shortcut: only reads/writes keys proven to affect smoothness on Win 11 24H2;
// upgrade when Windows changes DWM internals.

use anyhow::{Context, Result};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
    SetPriorityClass,
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

    // MultiplaneOverlay (MPO): 0 = disable MPO on some GPU configs that cause stutter
    dwm_key
        .set_value("OverlayTestMode", &0u32)
        .context("set OverlayTestMode")?;

    Ok(())
}

/// Disable desktop animations for snappier UI.
fn apply_desktop_animations() -> Result<()> {
    unsafe {
        SystemParametersInfoW(SPI_SETANIMATION, 0, None, SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()
            .context("SPI_SETANIMATION")?;
        SystemParametersInfoW(
            SPI_SETCOMBOBOXANIMATION,
            0,
            None,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        )
        .ok()
        .context("SPI_SETCOMBOBOXANIMATION")?;
        SystemParametersInfoW(
            SPI_SETDRAGFULLWINDOWS,
            0,
            None,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        )
        .ok()
        .context("SPI_SETDRAGFULLWINDOWS")?;
        SystemParametersInfoW(
            SPI_SETGRADIENTCAPTIONS,
            0,
            None,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        )
        .ok()
        .context("SPI_SETGRADIENTCAPTIONS")?;
        SystemParametersInfoW(
            SPI_SETSELECTIONFADE,
            0,
            None,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        )
        .ok()
        .context("SPI_SETSELECTIONFADE")?;
        SystemParametersInfoW(SPI_SETUIEFFECTS, 0, None, SPIF_SENDCHANGE | SPIF_UPDATEINIFILE)
            .ok()
            .context("SPI_SETUIEFFECTS")?;
    }
    Ok(())
}

/// Find dwm.exe PID and set its priority class to ABOVE_NORMAL.
fn set_dwm_priority() -> Result<()> {
    let snapshot = unsafe {
        CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .context("CreateToolhelp32Snapshot")?
    };

    let result = unsafe {
        let mut entry: PROCESSENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry).is_ok() {
            loop {
                let exe_name: Vec<u8> = entry
                    .szExeFile
                    .iter()
                    .take_while(|&&c| c != 0)
                    .map(|&c| c as u8)
                    .collect();
                let exe_name = String::from_utf8_lossy(&exe_name);
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
                    break true;
                }

                if !Process32Next(snapshot, &mut entry).is_ok() {
                    break false;
                }
            }
        } else {
            false
        }
    };

    if result {
        Ok(())
    } else {
        Err(anyhow::anyhow!("dwm.exe process not found"))
    }
}
