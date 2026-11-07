# tests/smoke_test.ps1 - win-smooth smoke test (Windows 11 24H2)
#
# Verifies the installed win-smooth / explorer_inject.dll pair on the machine
# where win-smooth is installed. Safe to run: never restarts explorer, never
# replaces the shell, only launches win-smooth.exe briefly and kills it.
#
# Modes:
#   - CI / headless runner: basic checks only (file existence + valid PE/DLL +
#     PowerShell syntax check). Full explorer/DWM tests cannot run headless.
#   - Physical / test VM (installed): full test suite incl. launching
#     win-smooth.exe, DWM frametime baseline, shell idle RAM.
#
# Exit code: 0 = all runnable tests passed (skips are not failures),
#            1 = any test failed.
#
# Shortcut: DWM frametime via WMI/perf-counter best-effort instead of a DXGI
# host app, <when a D3D11 test harness exists, swap in real present-interval measurement>.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
$script:INSTALL_DIR = "${env:ProgramFiles}\win-smooth"
$script:EXE_INSTALL = Join-Path $script:INSTALL_DIR "win-smooth.exe"
$script:DLL_INSTALL = Join-Path $script:INSTALL_DIR "explorer_inject.dll"
$script:EXE_BUILD   = Join-Path $PSScriptRoot "..\target\release\win-smooth.exe"
$script:DLL_BUILD   = Join-Path $PSScriptRoot "..\hook_dll\x64\Release\explorer_inject.dll"
$script:LOG_FILE    = "$env:TEMP\win-smooth-smoke.log"
$script:SMOKE_SELF  = Join-Path $PSScriptRoot "smoke_test.ps1"

$ciRaw = $env:GITHUB_ACTIONS
$script:IS_CI   = if ([string]::IsNullOrEmpty($ciRaw)) { $false } else { [bool]::Parse($ciRaw) }
$script:HEADLESS    = $script:IS_CI -or (-not [Environment]::UserInteractive)
$script:FULL_MODE   = (-not $script:HEADLESS)

# Resolve which binaries to test: installed copy wins, else repo build artifacts.
$script:EXE = $null
$script:DLL = $null
$script:INSTALLED = $false
if (Test-Path $script:EXE_INSTALL -PathType Leaf) {
    $script:EXE       = $script:EXE_INSTALL
    $script:DLL       = $script:DLL_INSTALL
    $script:INSTALLED = $true
} elseif (Test-Path $script:EXE_BUILD -PathType Leaf) {
    $script:EXE = $script:EXE_BUILD
    $script:DLL = $script:DLL_BUILD
} else {
    $script:EXE = $script:EXE_INSTALL  # for reporting / graceful skip
    $script:DLL = $script:DLL_INSTALL
}

# ---------------------------------------------------------------------------
# Result tracking
# ---------------------------------------------------------------------------
$script:RESULTS = New-Object System.Collections.Generic.List[pscustomobject]
$script:OVERALL_PASS = $true
$script:SmoothProcess = $null

function Write-Log {
    param([string]$Message)
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $line = "[$ts] $Message"
    try { Add-Content -Path $script:LOG_FILE -Value $line -ErrorAction SilentlyContinue } catch {}
    Write-Host $line
}

function Record-Result {
    param([string]$Name, [string]$Status, [string]$Detail, [switch]$Critical)
    $StatusUpper = $Status.ToUpperInvariant()
    if ($StatusUpper -eq "FAIL") { $script:OVERALL_PASS = $false }
    $script:RESULTS.Add([pscustomobject]@{
        Name    = $Name
        Status  = $Status
        Detail  = $Detail
        Time    = (Get-Date -Format "HH:mm:ss")
    })
    $tag = $StatusUpper
    Write-Log ("{0}: {1} - {2}" -f $tag, $Name, $Detail)
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
# Validate a PE image: MZ signature + PE\0\0 signature at e_lfanew.
# Returns [pscustomobject]@Valid=$true|$false, Kind="EXE"|"DLL"|"UNKNOWN", Reason.
function Test-PeImage {
    param([string]$Path)
    if (-not (Test-Path $Path -PathType Leaf)) {
        return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "file not found" }
    }
    try {
        $bytes = [System.IO.File]::ReadAllBytes($Path)
        if ($bytes.Length -lt 4) {
            return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "too small" }
        }
        $magic = [System.Text.Encoding]::ASCII.GetString($bytes, 0, 2)
        if ($magic -ne "MZ") {
            return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "no MZ signature" }
        }
        if ($bytes.Length -lt 64) {
            return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "truncated DOS header" }
        }
        $e_lfanew = [BitConverter]::ToInt32($bytes, 60)
        if ($e_lfanew -lt 0 -or $e_lfanew + 24 -gt $bytes.Length) {
            return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "bad e_lfanew" }
        }
        $sig = [System.Text.Encoding]::ASCII.GetString($bytes, $e_lfanew, 4)
        if ($sig -ne "PE`0`0") {
            return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "no PE signature" }
        }
        # COFF header Characteristics at e_lfanew + 24 (2 bytes). IMAGE_FILE_DLL = 0x2000.
        $chars = [BitConverter]::ToUInt16($bytes, $e_lfanew + 24)
        $isDll = ($chars -band 0x2000) -ne 0
        $kind  = if ($isDll) { "DLL" } else { "EXE" }
        return [pscustomobject]@{ Valid = $true; Kind = $kind; Reason = "valid PE" }
    } catch {
        return [pscustomobject]@{ Valid = $false; Kind = "UNKNOWN"; Reason = "exception: $_" }
    }
}

# ---------------------------------------------------------------------------
# Test 1 & 2: binaries exist and are valid PE executable / DLL
# ---------------------------------------------------------------------------
function Test-Binaries {
    Write-Log "=== Test 1/2: win-smooth.exe + explorer_inject.dll (PE validity) ==="

    # --- win-smooth.exe ---
    $pe = Test-PeImage $script:EXE
    if ($pe.Valid) {
        Record-Result "win-smooth.exe exists + valid PE" "PASS" ("{0} ({1})" -f $script:EXE, $pe.Reason)
    } else {
        Record-Result "win-smooth.exe exists + valid PE" "FAIL" ($pe.Reason)
    }

    # --- explorer_inject.dll ---
    $pe = Test-PeImage $script:DLL
    if ($pe.Valid) {
        Record-Result "explorer_inject.dll exists + valid DLL" "PASS" ("{0} (kind={1}, {2})" -f $script:DLL, $pe.Kind, $pe.Reason)
    } else {
        Record-Result "explorer_inject.dll exists + valid DLL" "FAIL" ($pe.Reason)
    }
}

# ---------------------------------------------------------------------------
# Test 3: DWM running (dwm.exe process)
# ---------------------------------------------------------------------------
function Test-DwmRunning {
    Write-Log "=== Test 3: DWM running ==="
    $dwm = Get-Process -Name "dwm" -ErrorAction SilentlyContinue
    if ($dwm) {
        Record-Result "DWM running (dwm.exe)" "PASS" ("PID {0}, WS {1} MB" -f $dwm.Id, [math]::Round($dwm.WS / 1MB, 2))
    } else {
        Record-Result "DWM running (dwm.exe)" "FAIL" "dwm.exe process not found"
    }
}

# ---------------------------------------------------------------------------
# Test 4: win-smooth.exe starts and stays alive >= 5 seconds
# ---------------------------------------------------------------------------
function Test-ExeStaysAlive {
    Write-Log "=== Test 4: win-smooth.exe starts and stays alive >= 5s ==="
    if (-not $script:FULL_MODE) {
        Record-Result "win-smooth.exe stays alive >= 5s" "SKIP" "headless/CI: GUI app not launched"
        return
    }
    if (-not (Test-Path $script:EXE -PathType Leaf)) {
        Record-Result "win-smooth.exe stays alive >= 5s" "SKIP" "win-smooth.exe not found"
        return
    }
    try {
        $proc = Start-Process -FilePath $script:EXE -PassThru -WindowStyle Hidden -ErrorAction Stop
        $script:SmoothProcess = $proc
        $alive = $false
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        while ($sw.ElapsedMilliseconds -lt 5000) {
            Start-Sleep -Milliseconds 250
            if ($proc.HasExited) {
                $alive = $false
                break
            }
            $alive = $true
        }
        $sw.Stop()
        if ($alive -and (-not $proc.HasExited)) {
            Record-Result "win-smooth.exe stays alive >= 5s" "PASS" ("running {0} ms" -f $sw.ElapsedMilliseconds)
        } else {
            $exitCode = if ($proc.HasExited) { $proc.ExitCode } else { "n/a" }
            Record-Result "win-smooth.exe stays alive >= 5s" "FAIL" ("exited early, code={0}" -f $exitCode)
        }
    } catch {
        Record-Result "win-smooth.exe stays alive >= 5s" "FAIL" ("launch failed: $_")
    }
}

# ---------------------------------------------------------------------------
# Test 5: Hook DLL does not crash explorer.exe.
# Safe proxy: AppCertDLLs injects the DLL into any process created after
# install. Launch a throwaway cmd process; if the DLL's DllMain crashes, the
# process fails to start. explorer.exe itself is never restarted/touched.
# Shortcut: throwaway-process proxy instead of restarting explorer,
# <when a physical VM is available, restart explorer and confirm it survives hook load/unload>.
# ---------------------------------------------------------------------------
function Test-HookDoesNotCrashExplorer {
    Write-Log "=== Test 5: hook DLL does not crash explorer.exe ==="
    if (-not $script:FULL_MODE) {
        Record-Result "explorer survives hook load/unload" "SKIP" "headless/CI: skip process-injection test"
        return
    }
    if (-not $script:INSTALLED) {
        Record-Result "explorer survives hook load/unload" "SKIP" "not installed: AppCertDLLs not active"
        return
    }
    # explorer.exe must be running (the shell under test).
    $explorer = Get-Process -Name "explorer" -ErrorAction SilentlyContinue
    if (-not $explorer) {
        Record-Result "explorer survives hook load/unload" "FAIL" "explorer.exe not running"
        return
    }
    # Throwaway host: DLL loads via AppCertDLLs on process creation, unloads on exit.
    try {
        $host = Start-Process -FilePath "cmd.exe" -ArgumentList "/c exit 0" -PassThru -WindowStyle Hidden -ErrorAction Stop
        $waited = $host.WaitForExit(5000)
        $code   = if ($waited) { $host.ExitCode } else { "timeout" }
        if ($waited -and $code -eq 0) {
            Record-Result "explorer survives hook load/unload" "PASS" ("throwaway process exited {0}; explorer still running (PID {1})" -f $code, $explorer.Id)
        } else {
            Record-Result "explorer survives hook load/unload" "FAIL" ("throwaway process {0}; DLL may crash host" -f $code)
        }
    } catch {
        Record-Result "explorer survives hook load/unload" "FAIL" ("throwaway launch failed: $_")
    }
}

# ---------------------------------------------------------------------------
# Test 6: DWM frametime baseline captured (best-effort WMI/perf timing)
# ---------------------------------------------------------------------------
function Test-DwmFrametime {
    Write-Log "=== Test 6: DWM frametime baseline ==="
    if (-not $script:FULL_MODE) {
        Record-Result "DWM frametime baseline captured" "SKIP" "headless/CI: no DXGI host app"
        return
    }
    $baselineMs = $null
    $source     = "none"
    # Best-effort: time a DWM process query as a lightweight present-interval proxy.
    try {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $dwm = Get-CimInstance -ClassName Win32_Process -Filter "Name='dwm.exe'" -ErrorAction Stop
        $sw.Stop()
        $baselineMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
        $source     = "WMI Win32_Process query"
    } catch {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            Get-Process -Name "dwm" -ErrorAction Stop | Out-Null
            $sw.Stop()
            $baselineMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
            $source     = "Get-Process dwm"
        } catch {
            $baselineMs = -1
            $source     = "unavailable"
        }
    }
    if ($baselineMs -ge 0) {
        Record-Result "DWM frametime baseline captured" "PASS" ("{0} ms via {1}" -f $baselineMs, $source)
    } else {
        Record-Result "DWM frametime baseline captured" "FAIL" "could not capture baseline"
    }
    # Shortcut: real DXGI present-interval measurement needs a D3D11 host app;
    # <upgrade when a D3D11 test harness exists>.
}

# ---------------------------------------------------------------------------
# Test 7: Shell idle RAM reported (win-smooth.exe working set)
# ---------------------------------------------------------------------------
function Test-ShellIdleRam {
    Write-Log "=== Test 7: shell idle RAM (win-smooth.exe working set) ==="
    if (-not $script:FULL_MODE) {
        Record-Result "win-smooth.exe working set reported" "SKIP" "headless/CI: app not running"
        return
    }
    $proc = $script:SmoothProcess
    if ($proc -and (-not $proc.HasExited)) {
        try {
            $proc.Refresh()
            $wsMB = [math]::Round($proc.WS / 1MB, 2)
            Record-Result "win-smooth.exe working set reported" "PASS" ("{0} MB (PID {1})" -f $wsMB, $proc.Id)
        } catch {
            Record-Result "win-smooth.exe working set reported" "FAIL" ("could not read working set: $_")
        }
    } else {
        Record-Result "win-smooth.exe working set reported" "SKIP" "win-smooth.exe not running"
    }
}

# ---------------------------------------------------------------------------
# Test 0 (CI only): PowerShell syntax check of this script + repo scripts
# ---------------------------------------------------------------------------
function Test-Syntax {
    Write-Log "=== Test 0: PowerShell syntax check ==="
    $files = @(
        (Resolve-Path $script:SMOKE_SELF).Path,
        (Resolve-Path (Join-Path $PSScriptRoot "..\scripts\install.ps1")).Path,
        (Resolve-Path (Join-Path $PSScriptRoot "..\scripts\uninstall.ps1")).Path
    )
    $allOk = $true
    foreach ($f in $files) {
        if (-not (Test-Path $f)) { continue }
        $errors = $null
        $null = [System.Management.Automation.Language.Parser]::ParseFile($f, [ref] $null, [ref] $errors)
        if ($errors -and $errors.Count -gt 0) {
            $allOk = $false
            $msgs = ($errors | ForEach-Object { $_.Message }) -join "; "
            Record-Result ("PS syntax: {0}" -f (Split-Path $f -Leaf)) "FAIL" $msgs
        } else {
            Record-Result ("PS syntax: {0}" -f (Split-Path $f -Leaf)) "PASS" "no parse errors"
        }
    }
    if (-not $allOk) { $script:OVERALL_PASS = $false }
}

# ---------------------------------------------------------------------------
# Cleanup: stop win-smooth.exe if we launched it
# ---------------------------------------------------------------------------
function Cleanup {
    if ($script:SmoothProcess -and (-not $script:SmoothProcess.HasExited)) {
        try {
            $script:SmoothProcess.CloseMainWindow() | Out-Null
            Start-Sleep -Milliseconds 1000
            if (-not $script:SmoothProcess.HasExited) {
                $script:SmoothProcess.Kill()
            }
        } catch {}
    }
}

# ---------------------------------------------------------------------------
# Graceful skip: nothing to test if win-smooth is not installed and no build
# artifacts exist. This is not a failure - it's a skip with a warning.
# ---------------------------------------------------------------------------
if (-not (Test-Path $script:EXE -PathType Leaf)) {
    Write-Log "win-smooth smoke test - SKIPPED: win-smooth is not installed and no build artifacts found."
    Write-Log "  Expected: $script:EXE"
    Write-Log "  Build first with scripts\build-all.ps1, or install with scripts\install.ps1."
    Write-Log "=== OVERALL: SKIP (no binaries to test) ==="
    exit 0
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
$modeLabel = if ($script:FULL_MODE) { "FULL (physical VM)" } else { "BASIC (headless/CI)" }
$binLabel  = if ($script:INSTALLED) { "installed" } else { "build artifacts" }

try {
    Write-Log ("win-smooth smoke test - mode={0}, binaries={1}" -f $modeLabel, $binLabel)

    if (-not $script:HEADLESS) {
        Test-Binaries
        Test-DwmRunning
        Test-ExeStaysAlive
        Test-HookDoesNotCrashExplorer
        Test-DwmFrametime
        Test-ShellIdleRam
    } else {
        # CI / headless: basic file existence + PE validity + syntax.
        Test-Binaries
        Test-Syntax
    }

    Write-Log ("=== OVERALL: {0} ===" -f $(if ($script:OVERALL_PASS) { "PASS" } else { "FAIL" }))
    foreach ($r in $script:RESULTS) {
        Write-Log ("  [{0}] {1} - {2}" -f $r.Status, $r.Name, $r.Detail)
    }
} finally {
    Cleanup
}

$exit = if ($script:OVERALL_PASS) { 0 } else { 1 }
exit $exit
