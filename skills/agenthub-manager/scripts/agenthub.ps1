# Forward all CLI arguments, including flags, without interpreting them in PowerShell.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$forward = @($args)
$resolveOnly = $forward.Count -eq 1 -and $forward[0] -eq "--resolve"
$candidates = @()
if ($env:AGENTHUB_CLI) {
    # An explicit portable/CLI override is authoritative; never silently fall back.
    $candidates = @($env:AGENTHUB_CLI)
} else {
    $candidates += Join-Path $PSScriptRoot "agenthub.exe"
    $command = Get-Command agenthub.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { $candidates += $command.Source }
    foreach ($key in @("HKCU:\Software\AgentHub\CLI", "HKCU:\Software\AgentHub\MSI")) {
        $value = Get-ItemProperty -LiteralPath $key -Name CliPath -ErrorAction SilentlyContinue
        if ($value) { $candidates += $value.CliPath }
    }
    $installed = Get-ItemProperty -LiteralPath "HKCU:\Software\AgentHub\AgentHub" -ErrorAction SilentlyContinue
    if ($installed) {
        foreach ($name in @("InstallDir", "(default)")) {
            if ($installed.PSObject.Properties[$name] -and $installed.$name) {
                $candidates += Join-Path $installed.$name "agenthub.exe"
            }
        }
    }
    if ($env:LOCALAPPDATA) { $candidates += Join-Path $env:LOCALAPPDATA "AgentHub\agenthub.exe" }
    if ($env:ProgramFiles) { $candidates += Join-Path $env:ProgramFiles "AgentHub\agenthub.exe" }
}
foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        $binary = (Resolve-Path -LiteralPath $candidate).ProviderPath
        if ($resolveOnly) { Write-Output $binary; exit 0 }
        & $binary @forward
        exit $LASTEXITCODE
    }
}
throw "AgentHub CLI was not found. Install the complete AgentHub package, or set AGENTHUB_CLI to the agenthub.exe in your portable folder."
