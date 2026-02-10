# Find Visual Studio installation dynamically via vswhere
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath = & $vswhere -latest -property installationPath
if (-not $vsPath) { Write-Error "Visual Studio not found"; exit 1 }

Import-Module "$vsPath\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
Enter-VsDevShell -VsInstallPath $vsPath -SkipAutomaticLocation -Arch amd64

Set-Location $PSScriptRoot

# cmake, ninja, and nvcc are all in PATH after dev shell + CUDA install
if (-not (Test-Path build/build.ninja)) {
    if (Test-Path build) { Remove-Item -Recurse -Force build }
    cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release 2>&1
    if ($LASTEXITCODE -ne 0) { Write-Error "CMake configure failed"; exit 1 }
}

cmake --build build --config Release 2>&1
if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }

Write-Host "Build succeeded!" -ForegroundColor Green
