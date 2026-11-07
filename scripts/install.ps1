# install.ps1 — win-smooth installer (Windows 11 24H2)
#
# Requires: administrator elevation, win-smooth.exe built at target/release/win-smooth.exe,
#           explorer_inject.dll built at hook_dll\x64\Release\explorer_inject.dll
# Shortcut: install.ps1 installs once per machine; upgrade by re-running install.ps1 over the existing install.
#
# Places changed:
#   - HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell (backup then rewrite)
#   - %ProgramFiles%\win-smooth\  (win-smooth.exe + explorer_inject.dll)
#   - HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\win-smooth (uninstaller entry)
#   - System restore point created before any change

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:LOG_FILE = "$env:TEMP\win-smooth-install.log"
$script:BACKUP_FILE = "$env:TEMP\win-smooth-shell-backup.reg"
$script:INSTALL_DIR = "${env:ProgramFiles}\win-smooth"
$script:SHELL_KEY = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon"
$script:SHELL_VALUE = "Shell"
$script:UNINSTALL_KEY = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\win-smooth"

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

# --- Restore point (SystemRestore) ---
function New-RestorePoint {
    Write-Log "Creating system restore point before changes..."
    $result = & wmic.exe /Namespace:\\root\default Path SystemRestore Call CreateRestorePoint "win-smooth install", 102, 7 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Log "WARNING: Could not create restore point (wmic exit $LASTEXITCODE). Continuing anyway."
    } else {
        Write-Log "Restore point created."
    }
}

# --- Backup existing Shell registry value ---
function Backup-ShellKey {
    Write-Log "Backing up Winlogon Shell value to $script:BACKUP_FILE"
    if (Test-Path $script:SHELL_KEY) {
        $current = Get-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -ErrorAction SilentlyContinue
        if ($current -and $current.PSObject.Properties.Name -contains $script:SHELL_VALUE) {
            $current.$script:SHELL_VALUE | Out-File -FilePath $script:BACKUP_FILE -Encoding utf8 -ErrorAction Stop
            Write-Log "Backed up Shell = $($current.$script:SHELL_VALUE)"
        } else {
            "none" | Out-File -FilePath $script:BACKUP_FILE -Encoding utf8 -ErrorAction Stop
            Write-Log "No existing Shell value found; backed up 'none'."
        }
    } else {
        "none" | Out-File -FilePath $script:BACKUP_FILE -Encoding utf8 -ErrorAction Stop
        Write-Log "Winlogon key does not exist; backed up 'none'."
    }
}

function Restore-ShellFromBackup {
    param([string]$BackupFile)
    if (-not (Test-Path $BackupFile)) { return }
    $value = Get-Content -Path $BackupFile -TotalCount 1 -ErrorAction SilentlyContinue
    if (-not $value -or $value -eq "none") {
        if (Test-Path $script:SHELL_KEY) {
            Remove-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -ErrorAction SilentlyContinue
            Write-Log "Removed Shell value (restored to none)."
        }
        return
    }
    if (Test-Path $script:SHELL_KEY) {
        Set-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -Value $value -ErrorAction Stop
        Write-Log "Restored Shell = $value"
    }
}

# --- Copy binaries ---
function Copy-Binaries {
    Write-Log "Installing binaries to $script:INSTALL_DIR"
    New-Item -ItemType Directory -Path $script:INSTALL_DIR -Force | Out-Null

    $srcExe = Join-Path $PSScriptRoot "..\target\release\win-smooth.exe"
    $srcDll = Join-Path $PSScriptRoot "..\hook_dll\x64\Release\explorer_inject.dll"
    $dstExe = Join-Path $script:INSTALL_DIR "win-smooth.exe"
    $dstDll = Join-Path $script:INSTALL_DIR "explorer_inject.dll"

    if (-not (Test-Path $srcExe)) { throw "Source not found: $srcExe (build first: scripts\build-all.ps1)" }
    if (-not (Test-Path $srcDll)) { throw "Source not found: $srcDll (build first: scripts\build-all.ps1)" }

    Copy-Item -Path $srcExe -Destination $dstExe -Force
    Copy-Item -Path $srcDll -Destination $dstDll -Force
    Write-Log "Copied win-smooth.exe and explorer_inject.dll."
}

# --- Register shell replacement via Winlogon\Shell ---
function Register-Shell {
    Write-Log "Registering shell replacement in Winlogon\Shell"
    $shellPath = Join-Path $script:INSTALL_DIR "win-smooth.exe"
    if (Test-Path $script:SHELL_KEY) {
        Set-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -Value $shellPath -ErrorAction Stop
    } else {
        New-Item -Path $script:SHELL_KEY -Force | Out-Null
        Set-ItemProperty -Path $script:SHELL_KEY -Name $script:SHELL_VALUE -Value $shellPath -ErrorAction Stop
    }
    Write-Log "Shell set to $shellPath"
}

# --- Install hook DLL injection mechanism ---
function Install-HookDll {
    Write-Log "Installing hook DLL injection mechanism"
    # AppCertDLLs is the mechanism: DLL loads into every process that calls
    # CreateProcess/WinExec/etc., including explorer.exe. The DLL's DllMain installs
    # the WH_CALLWNDPROC hook internally and must not crash explorer.exe.
    $acdKey = "HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager"
    $dllPath = Join-Path $script:INSTALL_DIR "explorer_inject.dll"
    if (Test-Path $acdKey) {
        Set-ItemProperty -Path $acdKey -Name "AppCertDLLs" -Value $dllPath -ErrorAction Stop
    } else {
        New-Item -Path $acdKey -Force | Out-Null
        Set-ItemProperty -Path $acdKey -Name "AppCertDLLs" -Value $dllPath -ErrorAction Stop
    }
    Write-Log "AppCertDLLs set to $dllPath"
}

# --- Uninstaller entry ---
function Install-UninstallerEntry {
    Write-Log "Creating uninstaller entry at $script:UNINSTALL_KEY"
    if (Test-Path $script:UNINSTALL_KEY) { Remove-Item -Path $script:UNINSTALL_KEY -Recurse -Force | Out-Null }
    New-Item -Path $script:UNINSTALL_KEY -Force | Out-Null
    $uninstallExe = Join-Path $PSScriptRoot "uninstall.ps1"
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "DisplayName" -Value "win-smooth" -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "UninstallString" -Value "powershell.exe -ExecutionPolicy Bypass -File `"$uninstallExe`"" -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "DisplayVersion" -Value "0.1.0" -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "Publisher" -Value "projecthorizon993" -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "InstallLocation" -Value $script:INSTALL_DIR -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "NoModify" -Value 1 -ErrorAction Stop
    Set-ItemProperty -Path $script:UNINSTALL_KEY -Name "NoRepair" -Value 1 -ErrorAction Stop
    Write-Log "Uninstaller entry created."
}

# --- Main ---
try {
    Throw-Admin
    New-RestorePoint
    Backup-ShellKey
    Copy-Binaries
    Register-Shell
    Install-HookDll
    Install-UninstallerEntry
    Write-Log "win-smooth installed successfully. A reboot is required for the shell replacement to take effect."
    exit 0
} catch {
    Write-Log "ERROR: $_"
    Write-Log "Installation failed. Roll back manually: restore Shell from $script:BACKUP_FILE."
    exit 1
}
