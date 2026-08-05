# Builds aic.exe via Docker (no local Rust toolchain required) and copies
# the binary out to .\dist\aic.exe.
$ErrorActionPreference = "Stop"

docker build --target export -t aicommits-rs-build .

New-Item -ItemType Directory -Force -Path .\dist | Out-Null

$containerId = docker create aicommits-rs-build
docker cp "${containerId}:/aic.exe" .\dist\aic.exe
docker rm $containerId | Out-Null

Write-Host "Built .\dist\aic.exe"
