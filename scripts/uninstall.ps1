# uninstall.ps1 — win-smooth uninstaller (Windows 11 24H2)
#
# Requires: administrator elevation
# Restores everything changed by install.ps1:
#   - Winlogon\Shell restored from backup (or removed if none existed)
#   - %ProgramFiles%\win-smooth\ files removed
#   - AppCertDLLs hook injection removed
#   - Uninstaller entry removed
# Shortcut: uninstall.ps1 removes win-smooth completely; re-run install.ps1 to reinstall.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:LOG_FILE = "$env:TEMP\win-smooth-uninstall.log"
$script:BACKUP_FILE = "$env:TEMP\win-smooth-shell-backup.reg"
$script:INSTALL_DIR = "${env:ProgramFiles}\win-smooth"
$script:SHELL_KEY = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon"
$script:SHELL_VALUE = "Shell"
$script:UNINSTALL_KEY = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\win-smooth"
$script:ACD_KEY = "HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager"

function Write-Log {
    param([string]$Message)
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $line = "[$ts] $Message"
    Add-Content -Path $script:LOG_FILE -Value $line -ErrorAction SilentlyContinue
    Write-Host $line
}

function Throw-Admin {
    if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Log "ERROR: This script must be run as administrator."
        exit 1
    }
}

# --- Restore explorer.exe shell from backup ---
function Restore-Shell {
    Write-Log "Restoring Winlogon Shell from backup..."
    if (-not (Test-Path $script:BACKUP_FILE)) {
        Write-Log "No backup file found at $script:BACKUP_FILE. Setting Shell to explorer.exe."
        if (Test-Path $script:SHELL_KEY) {
            Set-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -Value "explorer.exe" -ErrorAction Stop
        }
        return
    }
    $value = Get-Content -Path $script:BACKUP_FILE -TotalCount 1 -ErrorAction SilentlyContinue
    if (-not $value -or $value -eq "none") {
        if (Test-Path $script:SHELL_KEY) {
            Remove-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -ErrorAction SilentlyContinue
            Write-Log "Shell value removed (was not set before install)."
        }
        return
    }
    if (Test-Path $script:SHELL_KEY) {
        Set-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -Value $value -ErrorAction Stop
        Write-Log "Shell restored to $value"
    }
}

# --- Remove hook DLL injection (AppCertDLLs) ---
function Remove-HookDll {
    Write-Log "Removing hook DLL injection from AppCertDLLs..."
    if (Test-Path $script:ACD_KEY) {
        if (Get-ItemProperty -Path $script:ACD_KEY -Name "AppCertDLLs" -ErrorAction SilentlyContinue) {
            Remove-ItemProperty -Path $script:ACD_KEY -Name "AppCertDLLs" -ErrorAction Stop
            Write-Log "AppCertDLLs removed."
        } else {
            Write-Log "AppCertDLLs not present; nothing to remove."
        }
    }
}

# --- Remove win-smooth files ---
function Remove-Files {
    Write-Log "Removing win-smooth files from $script:INSTALL_DIR..."
    if (Test-Path $script:INSTALL_DIR) {
        Remove-Item -Path $script:INSTALL_DIR -Recurse -Force -ErrorAction Stop
        Write-Log "Install directory removed."
    } else {
        Write-Log "Install directory not present; nothing to remove."
    }
}

# --- Remove uninstaller entry ---
function Remove-UninstallerEntry {
    Write-Log "Removing uninstaller entry at $script:UNINSTALL_KEY..."
    if (Test-Path $script:UNINSTALL_KEY) {
        Remove-Item -Path $script:UNINSTALL_KEY -Recurse -Force -ErrorAction Stop
        Write-Log "Uninstaller entry removed."
    } else {
        Write-Log "Uninstaller entry not present; nothing to remove."
    }
}

# --- Main ---
try {
    Throw-Admin
    Restore-Shell
    Remove-HookDll
    Remove-Files
    Remove-UninstallerEntry
    Write-Log "win-smooth uninstalled successfully. A reboot is required for the shell to fully revert."
    exit 0
} catch {
    Write-Log "ERROR: $_"
    Write-Log "Uninstallation failed partway. Review $script:LOG_FILE and restore Shell manually from $script:BACKUP_FILE."
    exit 1
}
