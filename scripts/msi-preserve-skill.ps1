# Embedded by prepare-windows-resources.py; executes before old MSI resources retire.
# Reads only the registered installation's known legacy Skill paths, never tool roots.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$registered = Get-ItemProperty -LiteralPath "HKCU:\Software\AgentHub\MSI" -Name CliPath -ErrorAction SilentlyContinue
if (-not $registered) { exit 0 }
$directory = Split-Path -Parent ([IO.Path]::GetFullPath([string]$registered.CliPath))
if (-not (Test-Path -LiteralPath (Join-Path $directory "agenthub.exe") -PathType Leaf)) { exit 0 }
$protected = if ($env:AGENTHUB_HOME) { [IO.Path]::GetFullPath($env:AGENTHUB_HOME) } else { Join-Path $env:USERPROFILE ".agenthub" }
if ($directory.StartsWith($protected, [StringComparison]::OrdinalIgnoreCase)) { exit 0 }
$known = '__LEGACY_JSON__' | ConvertFrom-Json
foreach ($group in ($known | Group-Object path)) {
    $relative = [string]$group.Name
    if ($relative -notmatch '^skills/agenthub-manager/[a-zA-Z0-9_.\-/]+$' -or $relative.Split('/') -contains '..') { continue }
    $candidate = [IO.Path]::GetFullPath((Join-Path $directory $relative))
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { continue }
    $linked = $false
    $parent = $candidate
    while ($parent) {
        if ((Test-Path -LiteralPath $parent) -and ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { $linked = $true; break }
        if ([StringComparer]::OrdinalIgnoreCase.Equals($parent, $directory)) { break }
        $parent = Split-Path -Parent $parent
    }
    if ($linked) { continue }
    $hash = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash
    if (@($group.Group | Where-Object { $_.sha256 -ieq $hash }).Count) { continue }
    $saved = "$candidate.user-preserved"
    $number = 0
    while (Test-Path -LiteralPath $saved) { $number++; $saved = "$candidate.user-preserved.$number" }
    Copy-Item -LiteralPath $candidate -Destination $saved
}
