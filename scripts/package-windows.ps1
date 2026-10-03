[CmdletBinding()]
param(
    [switch]$SkipInstall,
    [ValidateSet("all", "nsis", "msi")]
    [string]$Bundle = "all"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "Windows packages must be built from a Windows host."
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

foreach ($command in @("git", "cargo", "pnpm")) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required command is unavailable: $command"
    }
}
cargo tauri --version | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Tauri CLI is unavailable; install tauri-cli 2.12.0" }

if (-not $SkipInstall) {
    pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw "pnpm install failed" }
}

pnpm lint
if ($LASTEXITCODE -ne 0) { throw "frontend lint failed" }
pnpm test
if ($LASTEXITCODE -ne 0) { throw "frontend tests failed" }
cargo test -p agenthub-core -p agenthub-cli
if ($LASTEXITCODE -ne 0) { throw "Rust tests failed" }

cargo build --release -p agenthub-cli
if ($LASTEXITCODE -ne 0) { throw "AgentHub CLI build failed" }

$sidecarDir = Join-Path $repoRoot "src-tauri/binaries"
New-Item -ItemType Directory -Force -Path $sidecarDir | Out-Null
$cliBinary = Join-Path $repoRoot "target/release/agenthub.exe"
$sidecarBinary = Join-Path $sidecarDir "agenthub-x86_64-pc-windows-msvc.exe"
Copy-Item $cliBinary $sidecarBinary -Force

$bundleArgs = if ($Bundle -eq "all") { "nsis,msi" } else { $Bundle }
pnpm tauri build --bundles $bundleArgs
if ($LASTEXITCODE -ne 0) { throw "Tauri Windows package build failed" }

function Get-PESubsystem([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
        throw "Not a valid PE executable: $Path"
    }
    $peOffset = [System.BitConverter]::ToInt32($bytes, 0x3c)
    if ($peOffset -lt 0 -or ($peOffset + 94) -ge $bytes.Length) {
        throw "Invalid PE header: $Path"
    }
    return [System.BitConverter]::ToUInt16($bytes, $peOffset + 24 + 68)
}

$desktopBinary = Join-Path $repoRoot "target/release/agenthub-desktop.exe"
$builtCliBinary = Join-Path $repoRoot "target/release/agenthub.exe"
if ((Get-PESubsystem $desktopBinary) -ne 2) {
    throw "Desktop executable is not a Windows GUI subsystem binary; it would open a console window"
}
if ((Get-PESubsystem $builtCliBinary) -ne 3) {
    throw "CLI executable is not a Windows console subsystem binary"
}

$version = (Get-Content (Join-Path $repoRoot "package.json") -Raw | ConvertFrom-Json).version
$portableDirectory = Join-Path $repoRoot "target/release/portable/AgentHub"
if (Test-Path $portableDirectory) { Remove-Item $portableDirectory -Recurse -Force }
New-Item -ItemType Directory -Force -Path $portableDirectory | Out-Null
Copy-Item $desktopBinary (Join-Path $portableDirectory "agenthub-desktop.exe")
Copy-Item $builtCliBinary (Join-Path $portableDirectory "agenthub.exe")
if ((Get-PESubsystem (Join-Path $portableDirectory "agenthub-desktop.exe")) -ne 2 -or
    (Get-PESubsystem (Join-Path $portableDirectory "agenthub.exe")) -ne 3) {
    throw "Portable desktop and CLI must remain separate executables with the correct subsystem"
}
Copy-Item "skills/agenthub-manager/scripts/agenthub.ps1" $portableDirectory
New-Item -ItemType Directory -Path (Join-Path $portableDirectory "skills") | Out-Null
Copy-Item "skills/agenthub-manager" (Join-Path $portableDirectory "skills") -Recurse
Copy-Item "docs/portable-windows.md" (Join-Path $portableDirectory "README.md")
Copy-Item "LICENSE" $portableDirectory
$portableArchive = Join-Path $repoRoot "target/release/bundle/AgentHub_${version}_windows-x64-portable.zip"
Compress-Archive -Path $portableDirectory -DestinationPath $portableArchive -Force

$artifacts = @(
    Get-ChildItem "target/release/bundle/nsis/*-setup.exe" -ErrorAction SilentlyContinue
    Get-ChildItem "target/release/bundle/msi/*.msi" -ErrorAction SilentlyContinue
    Get-Item $builtCliBinary
    Get-Item $portableArchive
)

$checksumPath = Join-Path $repoRoot "target/release/bundle/SHA256SUMS.windows.txt"
$artifacts | ForEach-Object {
    $hash = (Get-FileHash -Algorithm SHA256 $_.FullName).Hash.ToLowerInvariant()
    "$hash  $($_.Name)"
} | Set-Content -Encoding ascii $checksumPath

Write-Host "Windows artifacts:"
$artifacts.FullName
Write-Host $checksumPath
