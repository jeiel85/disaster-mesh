$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root "apps/android/core-bridge/src/main/kotlin"

Push-Location $root
try {
    cargo build --locked --package mesh-ffi
    $library = if ($IsWindows) {
        "target/debug/mesh_ffi.dll"
    } elseif ($IsMacOS) {
        "target/debug/libmesh_ffi.dylib"
    } else {
        "target/debug/libmesh_ffi.so"
    }
    cargo run --locked --package mesh-ffi --features bindgen-cli --bin uniffi-bindgen -- `
        generate --no-format --library $library --language kotlin --out-dir $output

    $binding = Join-Path $output "org/disastermesh/core/mesh_ffi.kt"
    $normalized = (Get-Content $binding | ForEach-Object { $_.TrimEnd() }) -join "`n"
    $normalized = $normalized.Replace(
        '@file:Suppress("NAME_SHADOWING")',
        '@file:Suppress("NAME_SHADOWING", "UNUSED_EXPRESSION")'
    )
    [IO.File]::WriteAllText(
        $binding,
        $normalized.TrimEnd() + "`n",
        [Text.UTF8Encoding]::new($false)
    )
} finally {
    Pop-Location
}
