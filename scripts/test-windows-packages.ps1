# Installer tests mutate only an ephemeral GitHub Actions runner's user registry.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
if ($env:GITHUB_ACTIONS -ne "true") { throw "Run installer checks only on an isolated GitHub Actions Windows runner." }

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
$testRoot = Join-Path $env:RUNNER_TEMP "agenthub-package-checks"
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null
$env:AGENTHUB_HOME = Join-Path $testRoot "library"
$powershell = Join-Path $env:SystemRoot "System32/WindowsPowerShell/v1.0/powershell.exe"
$resolver = Join-Path $repoRoot "skills/agenthub-manager/scripts/agenthub.ps1"
$environmentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey("Environment")
$options = [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
$originalPath = $environmentKey.GetValue("Path", $null, $options)
$originalKind = if ($null -ne $originalPath) { $environmentKey.GetValueKind("Path") } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
$processPath = $env:PATH
$originalOverride = $env:AGENTHUB_CLI

function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function User-Path { [string]$environmentKey.GetValue("Path", "", $options) }
function Count-Entry([string]$Directory) {
    @((User-Path).Split(';') | Where-Object { $_.Trim('"').TrimEnd('\') -ieq $Directory.TrimEnd('\') }).Count
}
function Invoke-Package([string]$Binary, [string[]]$Arguments) {
    $process = Start-Process -FilePath $Binary -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(180000)) { $process.Kill(); throw "Installer timed out" }
    Assert ($process.ExitCode -in @(0, 3010)) "Installer exited with $($process.ExitCode)"
}
function Check-Version([string]$Binary) {
    $output = & $Binary --version
    Assert ($LASTEXITCODE -eq 0 -and $output -eq "agenthub $version") "Unexpected CLI version"
}
function Check-StaleEnvironment([string]$Expected) {
    $env:PATH = Join-Path $env:SystemRoot "System32"
    $env:AGENTHUB_CLI = $null
    $resolved = & $powershell -NoProfile -ExecutionPolicy Bypass -File $resolver --resolve
    Assert ($LASTEXITCODE -eq 0 -and $resolved -ieq $Expected) "Skill failed with an old process PATH"
    $output = & $powershell -NoProfile -ExecutionPolicy Bypass -File $resolver --version
    Assert ($LASTEXITCODE -eq 0 -and $output -eq "agenthub $version") "Skill argument forwarding failed"
    & $powershell -NoProfile -ExecutionPolicy Bypass -File $resolver invalid-command 2>$null | Out-Null
    Assert ($LASTEXITCODE -eq 2) "Skill did not preserve CLI failure exit code"
    $env:AGENTHUB_CLI = Join-Path $testRoot "missing-cli.exe"
    & $powershell -NoProfile -ExecutionPolicy Bypass -File $resolver --version 2>$null | Out-Null
    Assert ($LASTEXITCODE -ne 0) "An invalid explicit CLI override was silently ignored"
    $env:AGENTHUB_CLI = $null
    $env:PATH = $processPath
}

try {
    # Preserve long, expandable, unrelated entries across installs and relocations.
    $baseline = "%USERPROFILE%\tools;;" + ((1..180 | ForEach-Object { "C:\unrelated-long-path-$_" }) -join ';')
    $environmentKey.SetValue("Path", $baseline, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    $first = Join-Path $testRoot "command space 一"
    $second = Join-Path $testRoot "command space 二"
    foreach ($directory in @($first, $second)) {
        New-Item -ItemType Directory -Force -Path $directory | Out-Null
        Copy-Item target/release/agenthub.exe $directory
    }
    & ./scripts/windows-cli.ps1 -Action Install -InstallDirectory $first
    & ./scripts/windows-cli.ps1 -Action Install -InstallDirectory $first
    Assert ((Count-Entry $first) -eq 1 -and (User-Path).StartsWith($baseline)) "PATH duplication or truncation"
    Assert ($environmentKey.GetValueKind("Path") -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) "PATH representation changed"
    Check-StaleEnvironment (Join-Path $first "agenthub.exe")
    & ./scripts/windows-cli.ps1 -Action Install -InstallDirectory $second
    & ./scripts/windows-cli.ps1 -Action Uninstall -InstallDirectory $first
    Assert ((Count-Entry $first) -eq 0 -and (Count-Entry $second) -eq 1) "Relocated upgrade was unregistered by an old uninstaller"
    & ./scripts/windows-cli.ps1 -Action Uninstall -InstallDirectory $second
    Assert ((User-Path) -ceq $baseline) "Uninstall changed unrelated PATH entries"
    $preexisting = "$baseline;`"$first\`""
    $environmentKey.SetValue("Path", $preexisting, [Microsoft.Win32.RegistryValueKind]::String)
    & ./scripts/windows-cli.ps1 -Action Install -InstallDirectory $first
    & ./scripts/windows-cli.ps1 -Action Uninstall -InstallDirectory $first
    Assert ((User-Path) -ceq $preexisting) "Uninstall removed a preexisting user PATH entry"
    $environmentKey.SetValue("Path", $baseline, [Microsoft.Win32.RegistryValueKind]::ExpandString)

    $setup = (Get-ChildItem target/release/bundle/nsis/*-setup.exe | Select-Object -First 1).FullName
    $installed = Join-Path $testRoot "installed AgentHub"
    Invoke-Package $setup @("/S", "/D=$installed")
    foreach ($relative in @("agenthub.exe", "agenthub-desktop.exe", "skills/agenthub-manager/SKILL.md", "skills/agenthub-manager/scripts/agenthub.ps1")) {
        Assert (Test-Path (Join-Path $installed $relative)) "Installer omitted $relative"
    }
    Check-Version (Join-Path $installed "agenthub.exe")
    Check-StaleEnvironment (Join-Path $installed "agenthub.exe")
    Invoke-Package $setup @("/S", "/D=$installed")
    Assert ((Count-Entry $installed) -eq 1) "Same-version upgrade duplicated PATH"
    Invoke-Package (Join-Path $installed "uninstall.exe") @("/S", "_?=$installed")
    Assert ((Count-Entry $installed) -eq 0 -and (User-Path) -ceq $baseline) "NSIS uninstall did not restore PATH"
    Assert (-not (Test-Path (Join-Path $installed "agenthub.exe"))) "NSIS uninstall left CLI installed"

    $msi = (Get-ChildItem target/release/bundle/msi/*.msi | Select-Object -First 1).FullName
    $msiDirectory = Join-Path $testRoot "MSI AgentHub"
    $msiexec = Join-Path $env:SystemRoot "System32/msiexec.exe"
    Invoke-Package $msiexec @("/i", "`"$msi`"", "/qn", "/norestart", "INSTALLDIR=`"$msiDirectory`"")
    Assert ((Count-Entry $msiDirectory) -eq 1) "MSI did not register the user command PATH"
    Check-Version (Join-Path $msiDirectory "agenthub.exe")
    Check-StaleEnvironment (Join-Path $msiDirectory "agenthub.exe")
    Invoke-Package $msiexec @("/x", "`"$msi`"", "/qn", "/norestart")
    Assert ((Count-Entry $msiDirectory) -eq 0 -and (User-Path) -ceq $baseline) "MSI uninstall changed unrelated PATH"

    $archive = (Get-ChildItem target/release/bundle/*windows-x64-portable.zip | Select-Object -First 1).FullName
    $portableRoot = Join-Path $testRoot "portable"
    Expand-Archive -Path $archive -DestinationPath $portableRoot -Force
    $portable = Join-Path $portableRoot "AgentHub"
    foreach ($relative in @("AgentHub.exe", "agenthub.exe", "agenthub.ps1", "README.md", "skills/agenthub-manager/SKILL.md")) {
        Assert (Test-Path (Join-Path $portable $relative)) "Portable ZIP omitted $relative"
    }
    $env:PATH = Join-Path $env:SystemRoot "System32"
    $output = & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $portable "agenthub.ps1") --version
    Assert ($LASTEXITCODE -eq 0 -and $output -eq "agenthub $version") "Portable CLI requires a separate install"
    Assert (-not (Test-Path $env:AGENTHUB_HOME)) "Packaging or discovery unexpectedly created a library"
    Write-Host "Windows package checks passed: long PATH, ownership, resolver, upgrade, NSIS/MSI uninstall, portable ZIP."
} finally {
    if ($null -eq $originalPath) { $environmentKey.DeleteValue("Path", $false) }
    else { $environmentKey.SetValue("Path", $originalPath, $originalKind) }
    $environmentKey.Dispose()
    $env:PATH = $processPath
    $env:AGENTHUB_CLI = $originalOverride
}
