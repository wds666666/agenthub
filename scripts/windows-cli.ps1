[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Install", "Uninstall")]
    [string]$Action,
    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory
)

# Called by the installer in a hidden process; never opens the library or hosts.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Normalize-PathEntry([string]$Entry) {
    [Environment]::ExpandEnvironmentVariables($Entry.Trim().Trim('"')).TrimEnd('\', '/')
}

function Test-PathEntry([string]$Entry, [string]$Directory) {
    [StringComparer]::OrdinalIgnoreCase.Equals((Normalize-PathEntry $Entry), (Normalize-PathEntry $Directory))
}

# Stale resources are removed only when an exact previous package hash proves
# ownership. Unknown files, edited files, libraries and reparse points survive.
function Update-OwnedResources([string]$Directory) {
    $manifestPath = Join-Path $Directory "windows-resources.json"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) { return }
    $current = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($current.product -ne "AgentHub" -or $current.schemaVersion -ne 1) { throw "Invalid resource ownership manifest." }
    $receiptPath = Join-Path $Directory ".agenthub-installed-files.json"
    $previous = @($current.legacy)
    if (Test-Path -LiteralPath $receiptPath -PathType Leaf) {
        if ((Get-Item -LiteralPath $receiptPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Resource ownership receipt cannot be a link." }
        $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($receipt.product -ne "AgentHub" -or $receipt.schemaVersion -ne 1) { throw "Invalid previous resource ownership receipt." }
        $previous = @($receipt.files) + $previous
    }
    $protectedRoot = if ($env:AGENTHUB_HOME) { [IO.Path]::GetFullPath($env:AGENTHUB_HOME).TrimEnd('\', '/') } else { Join-Path $env:USERPROFILE ".agenthub" }
    $currentPaths = @($current.files | ForEach-Object { $_.path })
    foreach ($entry in $previous) {
        if ($entry.path -in $currentPaths) { continue }
        $relative = [string]$entry.path
        if ($relative -notmatch '^skills/agenthub-manager/[a-zA-Z0-9_.\-/]+$' -or $relative.Split('/') -contains '..' -or $relative.Contains('//')) { continue }
        if ([string]$entry.sha256 -notmatch '^[a-fA-F0-9]{64}$') { continue }
        $candidate = [IO.Path]::GetFullPath((Join-Path $Directory $relative))
        if ($candidate.StartsWith($protectedRoot + '\', [StringComparison]::OrdinalIgnoreCase) -or (Test-PathEntry $candidate $protectedRoot)) { continue }
        $linked = $false
        $ancestor = $candidate
        while ($ancestor -and -not (Test-PathEntry $ancestor $Directory)) {
            if (Test-Path -LiteralPath $ancestor) {
                if ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { $linked = $true; break }
            }
            $ancestor = Split-Path -Parent $ancestor
        }
        if (-not $linked -and (Test-Path -LiteralPath $candidate -PathType Leaf) -and (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash -ieq $entry.sha256) {
            Remove-Item -LiteralPath $candidate -Force
        }
    }
    # Installed files are recorded from the build manifest, never by scanning.
    $receipt = @{ product = "AgentHub"; schemaVersion = 1; files = @($current.files) }
    $temporaryReceipt = "$receiptPath.$([Guid]::NewGuid().ToString('N')).tmp"
    $receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $temporaryReceipt -Encoding UTF8
    Move-Item -LiteralPath $temporaryReceipt -Destination $receiptPath -Force
}

$directory = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\', '/')
if ($directory.Contains(';')) { throw "The installation directory cannot contain a semicolon for command discovery." }
$cliPath = Join-Path $directory "agenthub.exe"
if ($Action -eq "Install" -and -not (Test-Path -LiteralPath $cliPath -PathType Leaf)) {
    throw "The bundled agenthub.exe was not installed."
}
if ($Action -eq "Install") { Update-OwnedResources $directory }
if ($Action -eq "Uninstall") {
    # A retired installation owns its receipt even when another directory now
    # owns command registration. Preserve unknown/edited non-receipt files.
    $receiptPath = Join-Path $directory ".agenthub-installed-files.json"
    if ((Test-Path -LiteralPath $receiptPath -PathType Leaf) -and -not ((Get-Item -LiteralPath $receiptPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        $receipt = $null
        try { $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json } catch { }
        if ($receipt -and $receipt.PSObject.Properties.Name -contains "product" -and $receipt.PSObject.Properties.Name -contains "schemaVersion" -and $receipt.product -eq "AgentHub" -and $receipt.schemaVersion -eq 1) {
            Remove-Item -LiteralPath $receiptPath -Force
        }
    }
}

$userRegistry = [Microsoft.Win32.Registry]::CurrentUser
$environmentKey = $userRegistry.CreateSubKey("Environment")
$registration = $userRegistry.CreateSubKey("Software\AgentHub\CLI")
try {
    $options = [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
    $oldPath = $environmentKey.GetValue("Path", $null, $options)
    $pathKind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    if ($null -ne $oldPath) {
        $pathKind = $environmentKey.GetValueKind("Path")
        if ($pathKind -notin @([Microsoft.Win32.RegistryValueKind]::String, [Microsoft.Win32.RegistryValueKind]::ExpandString)) {
            throw "The user PATH has an unsupported registry type; it was preserved."
        }
    }
    $oldCli = $registration.GetValue("CliPath", $null, $options)
    $oldOwned = $registration.GetValue("OwnedPath", $null, $options)
    $entries = @()
    if ($null -ne $oldPath -and $oldPath -ne "") { $entries = @(([string]$oldPath).Split(';')) }

    if ($Action -eq "Install") {
        # A relocated upgrade removes only an entry a prior AgentHub install added.
        if ($oldOwned -and -not (Test-PathEntry $oldOwned $directory)) {
            $entries = @($entries | Where-Object { -not (Test-PathEntry $_ $oldOwned) })
        }
        $present = @($entries | Where-Object { Test-PathEntry $_ $directory }).Count -gt 0
        $owned = $null
        if (-not $present) {
            $entries += $directory
            $owned = $directory
        } elseif ($oldOwned -and (Test-PathEntry $oldOwned $directory)) {
            $owned = $directory
        }
    } else {
        # An old uninstaller must not unregister a newer installation elsewhere.
        if (-not $oldCli -or -not (Test-PathEntry $oldCli $cliPath)) { return }
        if ($oldOwned) {
            $entries = @($entries | Where-Object { -not (Test-PathEntry $_ $oldOwned) })
        }
    }

    try {
        $newPath = $entries -join ';'
        if ($newPath -cne [string]$oldPath) { $environmentKey.SetValue("Path", $newPath, $pathKind) }
        if ($Action -eq "Install") {
            $registration.SetValue("CliPath", $cliPath, [Microsoft.Win32.RegistryValueKind]::String)
            if ($owned) { $registration.SetValue("OwnedPath", $owned) }
            else { $registration.DeleteValue("OwnedPath", $false) }
        } else {
            $registration.DeleteValue("CliPath", $false)
            $registration.DeleteValue("OwnedPath", $false)
        }
    } catch {
        # Preserve the original registry representation and ownership on failure.
        if ($null -eq $oldPath) { $environmentKey.DeleteValue("Path", $false) }
        else { $environmentKey.SetValue("Path", $oldPath, $pathKind) }
        foreach ($value in @(@("CliPath", $oldCli), @("OwnedPath", $oldOwned))) {
            if ($null -eq $value[1]) { $registration.DeleteValue($value[0], $false) }
            else { $registration.SetValue($value[0], $value[1]) }
        }
        throw
    }
} finally {
    $environmentKey.Dispose()
    $registration.Dispose()
}

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AgentHubEnvironment {
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message,
        UIntPtr wParam, string lParam, uint flags, uint timeout, out UIntPtr result);
}
'@
$result = [UIntPtr]::Zero
[void][AgentHubEnvironment]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, "Environment", 2, 2000, [ref]$result)
