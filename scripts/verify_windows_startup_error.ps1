param(
    [Parameter(Mandatory = $true)]
    [string]$Executable
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
    throw "Windows executable was not found: $Executable"
}

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Executable))
$peOffset = [System.BitConverter]::ToInt32($bytes, 0x3c)
$optionalHeaderOffset = $peOffset + 24
$subsystem = [System.BitConverter]::ToUInt16($bytes, $optionalHeaderOffset + 68)
if ($subsystem -ne 2) {
    throw "Expected IMAGE_SUBSYSTEM_WINDOWS_GUI (2), found $subsystem"
}

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public sealed class StartupWindow
{
    public IntPtr Handle { get; set; }
    public string Title { get; set; } = "";
    public string Text { get; set; } = "";
}

public static class StartupWindowProbe
{
    private delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowTextW(IntPtr window, StringBuilder text, int maximum);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageW(IntPtr window, uint message, IntPtr maximum, StringBuilder text);

    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool PostMessageW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

    private static string WindowText(IntPtr window)
    {
        var text = new StringBuilder(4096);
        GetWindowTextW(window, text, text.Capacity);
        if (text.Length == 0)
        {
            SendMessageW(window, 0x000D, (IntPtr)text.Capacity, text);
        }
        return text.ToString();
    }

    public static StartupWindow Find(int expectedProcessId, string expectedTitle)
    {
        StartupWindow result = null;
        EnumWindows((window, parameter) =>
        {
            GetWindowThreadProcessId(window, out uint processId);
            if (processId != expectedProcessId || !IsWindowVisible(window))
            {
                return true;
            }
            string title = WindowText(window);
            if (title != expectedTitle)
            {
                return true;
            }
            var parts = new List<string> { title };
            EnumChildWindows(window, (child, childParameter) =>
            {
                string childText = WindowText(child);
                if (!String.IsNullOrWhiteSpace(childText))
                {
                    parts.Add(childText);
                }
                return true;
            }, IntPtr.Zero);
            result = new StartupWindow
            {
                Handle = window,
                Title = title,
                Text = String.Join("\n", parts)
            };
            return false;
        }, IntPtr.Zero);
        return result;
    }
}
"@

$process = $null
try {
    $process = Start-Process -FilePath $Executable -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    $window = $null
    while ([DateTime]::UtcNow -lt $deadline -and $null -eq $window) {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) {
            throw "Application exited before showing its startup error"
        }
        $window = [StartupWindowProbe]::Find($process.Id, "Streamlink Twitch GUI")
    }
    if ($null -eq $window) {
        throw "Startup error dialog did not appear within 20 seconds"
    }
    if (-not $window.Text.Contains("Twitch client ID is not configured")) {
        throw "Startup error dialog did not contain the actionable configuration error"
    }
    if (-not [StartupWindowProbe]::PostMessageW($window.Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Startup error dialog could not be closed"
    }
    if (-not $process.WaitForExit(5000)) {
        throw "Application did not exit after its startup error was dismissed"
    }
    Write-Output "Verified release GUI subsystem and visible startup error"
}
finally {
    if ($null -ne $process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
}
