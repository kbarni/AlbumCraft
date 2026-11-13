<#
.SYNOPSIS
  Build and package AlbumCraft for Windows x64.

.DESCRIPTION
  Writes, in $env:DIST (default: dist\release):
    albumcraft-<version>-windows-x64.msi            installer for all users (WiX v5)
    albumcraft-<version>-windows-x64-portable.zip   portable: unpack and run albumcraft.exe

  The names follow the Craft apps' convention (<stem>-<version>-<os>-<arch>), which the update
  check (src/updates.rs, craft-update) reads to pick the file for a copy: keep them.

  The binary links the C runtime statically, so neither needs the Visual C++ redistributable.
  Needs Rust (MSVC toolchain), the Windows SDK (rc.exe, for the icon) and WiX v5:
    dotnet tool install --global wix --version 5.0.2
    wix extension add -g WixToolset.UI.wixext/5.0.2

.EXAMPLE
  pwsh packaging/windows/package.ps1
  pwsh packaging/windows/package.ps1 -SkipBuild
#>
param([switch] $SkipBuild)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version lives in one place: [workspace.package] version in the root Cargo.toml.
$Version = $env:ALBUMCRAFT_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
# The MSI ProductVersion is numeric (major.minor.build): a pre-release tag is dropped there.
$MsiVersion = ($Version -split '-')[0]

$Target = 'x86_64-pc-windows-msvc'
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
Write-Output "AlbumCraft $Version for Windows x64"

if (-not $SkipBuild) {
  # The dynamic C runtime: ONNX Runtime's prebuilt library (the AI features) is built against it
  # and can't be linked with +crt-static. Its DLLs are shipped beside the exe (below).
  # Fail the build instead of warning when the icon can't be embedded.
  $env:ALBUMCRAFT_REQUIRE_ICON = '1'
  Invoke-Native 'cargo build' { cargo build --release --locked -p albumcraft --target $Target }
}
$Exe = Join-Path $TargetDir "$Target\release\albumcraft.exe"

# PE header checks: an x64 Windows GUI program (subsystem 2: no console window opens with it).
$bytes = [System.IO.File]::ReadAllBytes($Exe)
$pe = [BitConverter]::ToInt32($bytes, 0x3C)
$machine = [BitConverter]::ToUInt16($bytes, $pe + 4)
$subsystem = [BitConverter]::ToUInt16($bytes, $pe + 0x5C)
if ($machine -ne 0x8664) { throw "albumcraft.exe is for machine 0x$('{0:X}' -f $machine), expected x64" }
if ($subsystem -ne 2) { throw "albumcraft.exe has PE subsystem $subsystem, expected 2 (GUI)" }
# The Start menu shortcut shows the icon embedded in the exe: refuse an exe without one.
Add-Type -Namespace AlbumCraft -Name Shell -MemberDefinition '[DllImport("shell32.dll", CharSet = CharSet.Unicode)] public static extern uint ExtractIconEx(string file, int index, IntPtr[] large, IntPtr[] small, uint icons);'
$icons = [AlbumCraft.Shell]::ExtractIconEx($Exe, -1, $null, $null, 0)
if ($icons -eq 0) { throw 'albumcraft.exe has no embedded icon: rebuild without -SkipBuild' }
Write-Output "ok albumcraft.exe: x64 GUI program, $icons icon(s)"

$Name = "albumcraft-$Version-windows-x64"
# the portable zip holds this one folder (its name is the zip's)
$Stage = Join-Path $TargetDir "windows-package\$Name-portable"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $Exe $Stage
foreach ($f in 'README.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE') { Copy-Item (Join-Path $Root $f) $Stage }
# The Visual C++ runtime DLLs (vcruntime140, msvcp140, ...), app-local, so no VC++ redistributable
# has to be installed; the Universal CRT is part of Windows 10 and 11.
$Vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$Vs = & $Vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $Vs) { throw 'Visual Studio with the C++ tools not found (vswhere)' }
$Crt = Get-ChildItem (Join-Path $Vs 'VC\Redist\MSVC\*\x64\Microsoft.VC*.CRT') -Directory |
  Where-Object { $_.FullName -match '\\MSVC\\\d' } | Sort-Object { [version]($_.Parent.Parent.Name) } -Descending |
  Select-Object -First 1
if (-not $Crt) { throw "no Visual C++ runtime under $Vs\VC\Redist\MSVC" }
Copy-Item (Join-Path $Crt.FullName '*.dll') $Stage
Write-Output "ok C++ runtime: $((Get-ChildItem $Stage -Filter *.dll).Name -join ', ') from $($Crt.FullName)"

# ---- MSI ---------------------------------------------------------------------------------------
$Msi = Join-Path $Dist "$Name.msi"
Invoke-Native 'wix build' {
  wix build (Join-Path $PSScriptRoot 'albumcraft.wxs') -arch x64 -ext WixToolset.UI.wixext `
    -d "Version=$MsiVersion" -d "StageDir=$Stage" -d "IconPath=$(Join-Path $Root 'assets\app-icon\albumcraft.ico')" `
    -o $Msi
}
# wix writes debug symbols (.wixpdb) next to the MSI; keep them out of the release.
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))

# ---- portable zip ------------------------------------------------------------------------------
# portable.txt beside the exe marks a portable copy: an update replaces the files in this folder
# (the MSI lists its files one by one, so it leaves this one out)
Set-Content -Path (Join-Path $Stage 'portable.txt') -Value 'This copy of AlbumCraft was unpacked from the portable zip; its updates replace the files in this folder.'
$Zip = Join-Path $Dist "$Name-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Stage -DestinationPath $Zip

Get-Item $Msi, $Zip | Format-Table Name, Length
