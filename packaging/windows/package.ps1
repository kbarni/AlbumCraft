<#
.SYNOPSIS
  Build and package AlbumCraft for Windows x64.

.DESCRIPTION
  Writes, in $env:DIST (default: dist\release):
    albumcraft-<version>-windows-x64.msi            installer for all users (WiX v5)
    albumcraft-<version>-windows-x64-portable.zip   portable: unpack and run albumcraft.exe

  The names follow the Craft apps' convention (<stem>-<version>-<os>-<arch>), which the update
  check (src/updates.rs, craft-update) reads to pick the file for a copy: keep them.

  The Visual C++ runtime DLLs go beside the exe, so neither needs the Visual C++ redistributable.
  Needs Rust (MSVC toolchain), the Windows SDK (rc.exe, for the icon) and WiX v5:
    dotnet tool install --global wix --version 5.0.2
    wix extension add -g WixToolset.UI.wixext/5.0.2

  Videos (the `video` feature) need the FFmpeg libraries of packaging/ffmpeg/build.sh, built in
  an MSYS2 UCRT64 shell (or unpacked from its --archive) into .tools\ffmpeg\windows-x86_64, or
  the folder named by $env:ALBUMCRAFT_FFMPEG; and libclang for the bindings (LLVM, found in
  C:\Program Files\LLVM or through $env:LIBCLANG_PATH). Their DLLs and licence texts go beside
  the exe. -NoVideo builds without videos.

.EXAMPLE
  pwsh packaging/windows/package.ps1
  pwsh packaging/windows/package.ps1 -SkipBuild
  pwsh packaging/windows/package.ps1 -NoVideo
#>
param([switch] $SkipBuild, [switch] $NoVideo)
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

# The FFmpeg libraries (packaging/ffmpeg/build.sh): bin\ the DLLs, lib\ the import libraries,
# include\ the headers, share\licenses\ their licence texts.
$FFmpeg = $null
if (-not $NoVideo) {
  $FFmpeg = if ($env:ALBUMCRAFT_FFMPEG) { $env:ALBUMCRAFT_FFMPEG } else { Join-Path $Root '.tools\ffmpeg\windows-x86_64' }
  foreach ($f in 'lib\avcodec.lib', 'include\libavcodec\avcodec.h', 'share\licenses\FFmpeg-COPYING.LGPLv2.1') {
    if (-not (Test-Path (Join-Path $FFmpeg $f))) {
      throw "no FFmpeg build in $FFmpeg ($f is missing): run packaging/ffmpeg/build.sh in an MSYS2 UCRT64 shell, or package with -NoVideo"
    }
  }
  $FFmpegDlls = Get-ChildItem (Join-Path $FFmpeg 'bin') -Filter *.dll
  if (-not $FFmpegDlls) { throw "no FFmpeg DLLs in $FFmpeg\bin" }
  Write-Output "FFmpeg libraries from $FFmpeg"
}

if (-not $SkipBuild) {
  # The dynamic C runtime: ONNX Runtime's prebuilt library (the AI features) is built against it
  # and can't be linked with +crt-static. Its DLLs are shipped beside the exe (below).
  # Fail the build instead of warning when the icon can't be embedded.
  $env:ALBUMCRAFT_REQUIRE_ICON = '1'
  if ($NoVideo) {
    Invoke-Native 'cargo build (without videos)' { cargo build --release --locked -p albumcraft --target $Target --no-default-features --features onnx }
  } else {
    # ffmpeg-sys-next takes the headers and import libraries from FFMPEG_DIR (with the MSVC
    # toolchain it would otherwise ask vcpkg)
    $env:FFMPEG_DIR = $FFmpeg
    $Llvm = Join-Path $env:ProgramFiles 'LLVM\bin'
    if (-not $env:LIBCLANG_PATH -and (Test-Path (Join-Path $Llvm 'libclang.dll'))) { $env:LIBCLANG_PATH = $Llvm }
    Invoke-Native 'cargo build' { cargo build --release --locked -p albumcraft --target $Target }
  }
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

# The FFmpeg DLLs beside the exe, where Windows looks for them first; they need only Windows' own
# DLLs (build.sh checks). Their licence texts (LGPL 2.1, dav1d's BSD) go with them.
if ($FFmpeg) {
  $FFmpegDlls | Copy-Item -Destination $Stage
  Copy-Item (Join-Path $FFmpeg 'share\licenses\*') $Stage
  Write-Output "ok FFmpeg: $($FFmpegDlls.Name -join ', ')"
  # the exe must ask for the DLLs that were copied (same FFmpeg versions as it was built with)
  $text = [Text.Encoding]::ASCII.GetString($bytes)
  foreach ($dll in 'avcodec-', 'avformat-', 'avutil-', 'swscale-', 'swresample-') {
    $m = [regex]::Match($text, "$dll\d+\.dll", 'IgnoreCase')
    if (-not $m.Success) { throw "albumcraft.exe doesn't use $dll*.dll: was it built with the video feature?" }
    if (-not (Test-Path (Join-Path $Stage $m.Value))) { throw "albumcraft.exe needs $($m.Value), which $FFmpeg\bin doesn't have" }
  }
  Write-Output 'ok albumcraft.exe uses those FFmpeg DLLs'
}

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
