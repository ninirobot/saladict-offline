# Smoke test: select a word in Notepad, trigger the global hotkey,
# then screenshot the main window and the popup for manual inspection.
# Keep this file ASCII-only: Windows PowerShell 5.1 misparses non-BOM UTF-8.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\smoke-ui.ps1 -Word apple
param(
    [string]$Word = 'apple',
    [string]$OutDir = $env:TEMP
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public class Native {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte b, byte s, uint f, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    public static List<IntPtr> WindowsOf(int pid) {
        var list = new List<IntPtr>();
        EnumWindows((h, l) => {
            int p; GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h)) list.Add(h);
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
'@

function Save-Shot([IntPtr]$hwnd, [string]$name) {
    $r = New-Object RECT
    [Native]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { return }
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    $path = Join-Path $OutDir $name
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    Write-Host ("saved {0} ({1}x{2})" -f $path, $w, $h)
}

$app = Get-Process saladict-desktop -ErrorAction SilentlyContinue
if (-not $app) { throw 'app not running: start saladict-desktop.exe first' }

$sample = Join-Path $env:TEMP 'saladict-sample-word.txt'
[System.IO.File]::WriteAllText($sample, $Word)
Start-Process notepad -ArgumentList $sample
Start-Sleep -Seconds 2

# Keep the app window out of the way so the click lands on Notepad.
if ($app.MainWindowHandle -ne 0) { [Native]::ShowWindow($app.MainWindowHandle, 6) | Out-Null }
Start-Sleep -Milliseconds 400

$note = Get-Process notepad | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[Native]::SetWindowPos($note.MainWindowHandle, [IntPtr](-2), 80, 80, 700, 500, 0x0040) | Out-Null
Start-Sleep -Milliseconds 500

[Native]::SetCursorPos(300, 240) | Out-Null
Start-Sleep -Milliseconds 200
[Native]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
[Native]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 400

# Win11 记事本会恢复上次的内容，先清空再输入，保证选区只有目标词
[System.Windows.Forms.SendKeys]::SendWait('^a')
Start-Sleep -Milliseconds 150
[System.Windows.Forms.SendKeys]::SendWait('{DEL}')
Start-Sleep -Milliseconds 150
[System.Windows.Forms.SendKeys]::SendWait($Word)
Start-Sleep -Milliseconds 250
[System.Windows.Forms.SendKeys]::SendWait('^a')
Start-Sleep -Milliseconds 400

# Move the pointer away so the popup does not open on top of the selection.
[Native]::SetCursorPos(140, 640) | Out-Null
Start-Sleep -Milliseconds 250

[Native]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)
[Native]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 90
[Native]::keybd_event(0x44, 0, 0, [UIntPtr]::Zero)
[Native]::keybd_event(0x44, 0, 2, [UIntPtr]::Zero)
[Native]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
[Native]::keybd_event(0x11, 0, 2, [UIntPtr]::Zero)

Start-Sleep -Milliseconds 2500

$targets = [Native]::WindowsOf($app.Id)
$sizes = $targets | ForEach-Object {
    $r = New-Object RECT
    [Native]::GetWindowRect($_, [ref]$r) | Out-Null
    [pscustomobject]@{ Hwnd = $_; W = $r.Right - $r.Left; H = $r.Bottom - $r.Top }
}

$popup = $sizes | Where-Object { $_.W -gt 200 -and $_.W -lt 700 } | Select-Object -First 1
$main = $sizes | Sort-Object W -Descending | Select-Object -First 1

if ($popup) { Save-Shot $popup.Hwnd 'saladict-popup.png' } else { Write-Host 'popup window not found' }
if ($main) { Save-Shot $main.Hwnd 'saladict-main.png' }
