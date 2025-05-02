param (
    [Parameter(Mandatory = $false, ValueFromRemainingArguments = $true)]
    [string[]] $KlintArgs
)

$toolchain = "1.85"
$sysroot = (& rustup run $toolchain rustc --print sysroot).Trim()
if (-not (Test-Path $sysroot)) {
    Write-Error "Rust toolchain '$toolchain' not found. Install it with `rustup install $toolchain`."
    exit 1
}

$dllDir = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\lib"
if (-not (Test-Path $dllDir)) {
    Write-Error "DLL directory not found: $dllDir"
    exit 1
}
$env:PATH = "$dllDir;$env:PATH"

$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
$klintExe = Join-Path $cargoHome "bin\klint.exe"

if (-not (Test-Path $klintExe)) {
    Write-Error "Could not locate klint.exe. Checked Cargo bin: $cargoBin."
    exit 1
}

& $klintExe @KlintArgs
exit $LASTEXITCODE
