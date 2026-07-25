# Build env for rtk on the maintainer's Windows ARM64 machine.
#
# Why this exists: cargo is not on PATH, and the native aarch64-pc-windows-msvc
# target cannot link (BuildTools ships no Hostarm64\arm64\link.exe and lib\arm64
# is nearly empty). The working recipe is the x86_64 HOST toolchain with explicit
# MSVC/SDK lib paths.
#
# Dot-source from PowerShell — NOT Git Bash, where coreutils `link` shadows
# MSVC's link.exe:
#
#   . .\scripts\win-dev-env.ps1
#
# Git usr\bin is appended LAST: core::stream tests spawn sh/cat/true, but it
# must not shadow System32 find/sort.

$env:RUSTUP_TOOLCHAIN   = "stable-x86_64-pc-windows-msvc"
$env:CARGO_BUILD_TARGET = "x86_64-pc-windows-msvc"

# Latest installed MSVC and Windows SDK, resolved at run time so a toolchain
# update doesn't rot a hardcoded version pin.
$msvc = (Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC" |
         Sort-Object Name | Select-Object -Last 1).FullName
$sdk  = "C:\Program Files (x86)\Windows Kits\10"
$sdkv = (Get-ChildItem "$sdk\Lib" | Sort-Object Name | Select-Object -Last 1).Name

$env:PATH = "$env:USERPROFILE\.cargo\bin;$msvc\bin\Hostarm64\x64;$env:PATH;C:\Program Files\Git\usr\bin"
$env:LIB  = "$msvc\lib\x64;$sdk\Lib\$sdkv\um\x64;$sdk\Lib\$sdkv\ucrt\x64"

Write-Host "rtk dev env: MSVC $(Split-Path $msvc -Leaf), SDK $sdkv, target $env:CARGO_BUILD_TARGET"
