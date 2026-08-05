# Builds aic.exe via Docker (no local Rust toolchain required) and copies
# it, plus the libstdc++-6.dll it needs alongside it, to .\dist\.
$ErrorActionPreference = "Stop"

docker build --target export -t aicommits-rs-build .

New-Item -ItemType Directory -Force -Path .\dist | Out-Null

$containerId = docker create aicommits-rs-build
docker cp "${containerId}:/aic.exe" .\dist\aic.exe
docker cp "${containerId}:/libstdc++-6.dll" .\dist\libstdc++-6.dll
docker rm $containerId | Out-Null

Write-Host "Built .\dist\aic.exe (+ libstdc++-6.dll, keep both files together)"
