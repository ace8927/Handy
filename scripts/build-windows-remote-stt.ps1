# Build the unmodified Handy Windows NSIS installer with only the Remote STT patch.
# Run from PowerShell on Windows x64 with Rust, Bun, MSVC C++ build tools,
# Vulkan SDK, cmake and vcpkg installed (see BUILD.md).
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
$configFile = Join-Path $root 'src-tauri\tauri.conf.json'
$configBackup = [System.IO.File]::ReadAllBytes($configFile)
try {
    foreach ($cmd in @('cargo', 'bun', 'cmake', 'vcpkg')) {
        if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
            throw "Missing $cmd. Install the Windows build prerequisites described in BUILD.md."
        }
    }
    if (-not $env:VULKAN_SDK) { throw 'VULKAN_SDK is unset. Install the Vulkan SDK and open a new shell.' }

    & vcpkg install spirv-headers:x64-windows
    if ($LASTEXITCODE -ne 0) { throw 'vcpkg could not install spirv-headers:x64-windows' }
    $vcpkgRoot = $env:VCPKG_INSTALLATION_ROOT
    if (-not $vcpkgRoot) { $vcpkgRoot = $env:VCPKG_ROOT }
    if (-not $vcpkgRoot) { throw 'Set VCPKG_ROOT (or VCPKG_INSTALLATION_ROOT) to your vcpkg installation.' }
    $prefix = Join-Path $vcpkgRoot 'installed\x64-windows'
    $env:CMAKE_PREFIX_PATH = if ($env:CMAKE_PREFIX_PATH) { "$prefix;$env:CMAKE_PREFIX_PATH" } else { $prefix }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'Visual Studio C++ Build Tools are missing.' }
    $vsRoot = & $vswhere -latest -products '*' -property installationPath
    if (-not $vsRoot) { throw 'No Visual Studio C++ installation found.' }
    $v = (Get-Content (Join-Path $vsRoot "VC\Auxiliary\Build\Microsoft.VCRedistVersion.default.txt")).Trim()
    $base = Join-Path $vsRoot ("VC\Redist\MSVC\" + $v + "\x64")
    $crt = Get-ChildItem $base -Directory -Filter 'Microsoft.VC*.CRT' | Sort-Object Name | Select-Object -Last 1
    $omp = Get-ChildItem $base -Directory -Filter 'Microsoft.VC*.OpenMP' | Sort-Object Name | Select-Object -Last 1
    if (-not $crt -or -not $omp) { throw 'VC++ Redistributable CRT / OpenMP DLLs missing.' }
    $env:HANDY_VC_REDIST_DIRS = "$($crt.FullName);$($omp.FullName)"

    # Upstream's signCommand requires a private signing account; fork is unsigned.
    $config = Get-Content $configFile -Raw | ConvertFrom-Json
    $config.bundle.windows.PSObject.Properties.Remove('signCommand')
    $config.bundle.createUpdaterArtifacts = $false
    $config.plugins.updater.endpoints = @()
    $json = $config | ConvertTo-Json -Depth 100
    [System.IO.File]::WriteAllText($configFile, $json, [System.Text.UTF8Encoding]::new($false))

    & bun install
    if ($LASTEXITCODE -ne 0) { throw 'bun install failed' }
    & bun run tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) { throw 'Windows NSIS build failed' }
    Write-Host "Installer(s): $root\src-tauri\target\release\bundle\nsis\*.exe"
} finally {
    # Restore official base config: never leave developer release settings modified.
    [System.IO.File]::WriteAllBytes($configFile, $configBackup)
    Pop-Location
}
