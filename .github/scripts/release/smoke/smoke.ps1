$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
Set-Location $root

cargo build --release --locked -p cli

$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("negentropy-smoke-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null

try {
    Set-Content -Path (Join-Path $tmpdir 'sample.rs') -Value 'fn main() {}'
    $out = & (Join-Path $root 'target/release/negentropy.exe') $tmpdir
    Write-Output $out
    if ($out -ne 'clean') {
        throw "smoke: expected clean, got: $out"
    }
    Write-Output 'smoke: ok'
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
