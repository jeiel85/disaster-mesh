[CmdletBinding()]
param(
    [string]$Serial = "emulator-5554",
    [string]$PackageName = "org.disastermesh.android.dev",
    [string]$EvidenceDirectory = "reports\evidence\emulator-api36",
    [switch]$SkipBuild,
    [switch]$PreserveAppData
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$LASTEXITCODE = 0

$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$sourceCommitAtStart = (& git -c "safe.directory=$($root.Replace('\', '/'))" -C $root rev-parse HEAD).Trim()
$sourceDirtyAtStart = [bool]((& git -c "safe.directory=$($root.Replace('\', '/'))" -C $root status --porcelain) -join "")

function Find-AndroidSdk {
    $candidates = @(
        $env:ANDROID_SDK_ROOT,
        $env:ANDROID_HOME,
        $(if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "Android\Sdk" })
    ) | Where-Object { $_ }
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath (Join-Path $candidate "platform-tools\adb.exe")) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    throw "Android SDK not found. Set ANDROID_SDK_ROOT or ANDROID_HOME."
}

function Find-JavaHome {
    if ($env:JAVA_HOME -and (Test-Path -LiteralPath (Join-Path $env:JAVA_HOME "bin\java.exe"))) {
        return $env:JAVA_HOME
    }
    $local = Join-Path $HOME ".local"
    if (Test-Path -LiteralPath $local) {
        $java = Get-ChildItem -LiteralPath $local -Filter java.exe -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "temurin|jdk" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($java) { return $java.Directory.Parent.FullName }
    }
    throw "JDK 17 not found. Set JAVA_HOME before running this script."
}

$sdk = Find-AndroidSdk
$adb = Join-Path $sdk "platform-tools\adb.exe"
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_HOME = $sdk
$env:JAVA_HOME = Find-JavaHome
$env:PATH = "$(Join-Path $env:JAVA_HOME 'bin');$env:PATH"
if (Test-Path -LiteralPath "C:\msys64\mingw64\bin") {
    $env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
}
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"

$state = ((& $adb -s $Serial get-state 2>$null) -join "").Trim()
if ($state -ne "device") {
    throw "$Serial is not online. Run tools/setup_android_emulator.ps1 -Action Start first."
}
$booted = ((& $adb -s $Serial shell getprop sys.boot_completed) -join "").Trim()
if ($booted -ne "1") { throw "$Serial has not completed boot." }

$evidence = if ([IO.Path]::IsPathRooted($EvidenceDirectory)) {
    $EvidenceDirectory
} else {
    Join-Path $root $EvidenceDirectory
}
New-Item -ItemType Directory -Force -Path $evidence | Out-Null

function Get-UiDocument {
    for ($attempt = 0; $attempt -lt 10; $attempt++) {
        & $adb -s $Serial shell uiautomator dump /sdcard/disastermesh-smoke-window.xml | Out-Null
        $raw = (& $adb -s $Serial shell cat /sdcard/disastermesh-smoke-window.xml) -join "`n"
        try { return [xml]$raw } catch { Start-Sleep -Milliseconds 400 }
    }
    throw "Unable to read the current UI hierarchy."
}

function Find-UiNode([xml]$Document, [string]$Text, [switch]$Wildcard) {
    $nodes = $Document.SelectNodes("//node")
    if ($Wildcard) {
        return $nodes | Where-Object { ([string]$_.text) -like $Text } | Select-Object -First 1
    }
    return $nodes | Where-Object { ([string]$_.text) -ceq $Text } | Select-Object -First 1
}

function Wait-UiNode([string]$Text, [switch]$Wildcard, [int]$TimeoutSeconds = 15) {
    $deadline = [DateTimeOffset]::Now.AddSeconds($TimeoutSeconds)
    do {
        $document = Get-UiDocument
        $node = Find-UiNode -Document $document -Text $Text -Wildcard:$Wildcard
        if ($node) { return $node }
        Start-Sleep -Milliseconds 500
    } while ([DateTimeOffset]::Now -lt $deadline)
    throw "UI text not found: $Text"
}

function Tap-UiText([string]$Text, [switch]$Wildcard, [int]$TimeoutSeconds = 15) {
    $node = Wait-UiNode -Text $Text -Wildcard:$Wildcard -TimeoutSeconds $TimeoutSeconds
    $match = [regex]::Match([string]$node.bounds, "\[(\d+),(\d+)\]\[(\d+),(\d+)\]")
    if (-not $match.Success) { throw "Invalid bounds for '$Text': $($node.bounds)" }
    $x = ([int]$match.Groups[1].Value + [int]$match.Groups[3].Value) / 2
    $y = ([int]$match.Groups[2].Value + [int]$match.Groups[4].Value) / 2
    & $adb -s $Serial shell input tap ([int]$x) ([int]$y) | Out-Null
}

function Save-Screenshot([string]$Name) {
    $devicePath = "/sdcard/$Name"
    & $adb -s $Serial shell screencap -p $devicePath | Out-Null
    & $adb -s $Serial pull $devicePath (Join-Path $evidence $Name) | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to capture $Name" }
}

function Get-OwnQr {
    $node = Wait-UiNode -Text "DM1:*" -Wildcard -TimeoutSeconds 20
    return [string]$node.text
}

function Get-TextSha256([string]$Value) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($Value)
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}

if (-not $SkipBuild) {
    Push-Location (Join-Path $root "apps\android")
    try {
        & .\gradlew :app:assembleDevDebug --no-parallel
        if ($LASTEXITCODE -ne 0) { throw "DevDebug APK build failed." }
    } finally {
        Pop-Location
    }
}

$apk = Join-Path $root "apps\android\app\build\outputs\apk\dev\debug\app-dev-debug.apk"
if (-not (Test-Path -LiteralPath $apk)) { throw "APK not found: $apk" }
& $adb -s $Serial install -r -t $apk | Out-Host
if ($LASTEXITCODE -ne 0) { throw "APK installation failed." }

if (-not $PreserveAppData) {
    & $adb -s $Serial shell pm clear $PackageName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to clear $PackageName for a deterministic smoke run." }
}
foreach ($permission in @(
    "android.permission.BLUETOOTH_SCAN",
    "android.permission.BLUETOOTH_CONNECT",
    "android.permission.BLUETOOTH_ADVERTISE",
    "android.permission.POST_NOTIFICATIONS"
)) {
    & $adb -s $Serial shell pm grant $PackageName $permission | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to grant $permission" }
}
& $adb -s $Serial shell svc bluetooth enable | Out-Null
& $adb -s $Serial logcat -c

$activity = ((& $adb -s $Serial shell cmd package resolve-activity --brief $PackageName) | Select-Object -Last 1).Trim()
if (-not $activity.Contains("/")) { throw "Launcher activity not found for $PackageName" }
& $adb -s $Serial shell am force-stop $PackageName | Out-Null
& $adb -s $Serial shell am start -W -n $activity | Out-Null

Wait-UiNode -Text "한계를 이해하고 계속" | Out-Null
Save-Screenshot "01-onboarding.png"
Tap-UiText "한계를 이해하고 계속"
Wait-UiNode -Text "신뢰할 연락처 관리" | Out-Null
Wait-UiNode -Text "제한된 진단 내보내기" | Out-Null
Save-Screenshot "02-home.png"

Tap-UiText "신뢰할 연락처 관리"
Wait-UiNode -Text "내 연락처 QR 문자열" | Out-Null
$identityBefore = Get-OwnQr
$identityHash = Get-TextSha256 $identityBefore
& $adb -s $Serial shell input keyevent KEYCODE_BACK | Out-Null
Wait-UiNode -Text "릴레이 모드" | Out-Null
Wait-UiNode -Text "신뢰할 연락처 관리" | Out-Null

Tap-UiText "릴레이 모드"
Wait-UiNode -Text "대기 모드 시작" | Out-Null
Tap-UiText "대기 모드 시작"
Start-Sleep -Seconds 2
$services = (& $adb -s $Serial shell dumpsys activity services $PackageName) -join "`n"
if ($services -notmatch "EmergencyRelayService") { throw "Foreground relay service did not start." }
$notifications = (& $adb -s $Serial shell dumpsys notification --noredact) -join "`n"
if ($notifications -notmatch [regex]::Escape($PackageName) -or $notifications -notmatch "DisasterMesh standby") {
    throw "Foreground relay notification was not posted."
}
Save-Screenshot "03-relay.png"
Tap-UiText "릴레이 중지"
Start-Sleep -Seconds 1
$services = (& $adb -s $Serial shell dumpsys activity services $PackageName) -join "`n"
if ($services -match "EmergencyRelayService") { throw "Foreground relay service did not stop." }
& $adb -s $Serial shell input keyevent KEYCODE_BACK | Out-Null
Wait-UiNode -Text "신뢰할 연락처 관리" | Out-Null

Tap-UiText "제한된 진단 내보내기"
Wait-UiNode -Text "제한된 진단 ZIP 미리보기" | Out-Null
Wait-UiNode -Text "• README.txt" | Out-Null
Save-Screenshot "04-diagnostics.png"
& $adb -s $Serial shell rm -f /sdcard/Download/disastermesh-diagnostics.zip | Out-Null
Tap-UiText "저장 위치 선택"
Tap-UiText "SAVE" -TimeoutSeconds 20

$deviceZip = "/sdcard/Download/disastermesh-diagnostics.zip"
$deadline = [DateTimeOffset]::Now.AddSeconds(15)
$lastSize = -1L
$stableSizeReads = 0
do {
    $sizeText = ((& $adb -s $Serial shell stat -c %s $deviceZip 2>$null) -join "").Trim()
    $size = 0L
    if ([long]::TryParse($sizeText, [ref]$size) -and $size -gt 0) {
        if ($size -eq $lastSize) { $stableSizeReads++ } else { $stableSizeReads = 0 }
        $lastSize = $size
        if ($stableSizeReads -ge 2) { break }
    }
    Start-Sleep -Milliseconds 500
} while ([DateTimeOffset]::Now -lt $deadline)
if ($stableSizeReads -lt 2) { throw "Diagnostic ZIP was not completely saved by DocumentsUI." }
& $adb -s $Serial shell sync | Out-Null

$localZip = Join-Path ([IO.Path]::GetTempPath()) "disastermesh-emulator-diagnostics.zip"
& $adb -s $Serial pull $deviceZip $localZip | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Failed to pull diagnostic ZIP." }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($localZip)
try {
    $entryNames = @($archive.Entries | ForEach-Object FullName)
    $expectedEntries = @("README.txt", "metadata.json", "relay.txt", "events.csv")
    if (($entryNames -join "|") -cne ($expectedEntries -join "|")) {
        throw "Unexpected diagnostic ZIP entries: $($entryNames -join ', ')"
    }
    $metadataEntry = $archive.GetEntry("metadata.json")
    $reader = [IO.StreamReader]::new($metadataEntry.Open())
    try { $metadata = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    if ([int]$metadata.android_api -ne 36) { throw "Diagnostic ZIP did not record API 36." }
} finally {
    $archive.Dispose()
    Remove-Item -LiteralPath $localZip -Force -ErrorAction SilentlyContinue
}

& $adb -s $Serial shell am force-stop $PackageName | Out-Null
& $adb -s $Serial shell am start -W -n $activity | Out-Null
Wait-UiNode -Text "한계를 이해하고 계속" | Out-Null
Tap-UiText "한계를 이해하고 계속"
Wait-UiNode -Text "신뢰할 연락처 관리" | Out-Null
Tap-UiText "신뢰할 연락처 관리"
Wait-UiNode -Text "내 연락처 QR 문자열" | Out-Null
$identityAfter = Get-OwnQr
if ($identityBefore -cne $identityAfter) { throw "Contact identity changed after process restart." }

$resumed = (& $adb -s $Serial shell dumpsys activity activities) -join "`n"
if ($resumed -notmatch "topResumedActivity.*$([regex]::Escape($PackageName))") {
    throw "DisasterMesh is not the resumed activity at the end of the smoke run."
}
$crashes = (& $adb -s $Serial logcat -d -b crash) -join "`n"
if ($crashes -match "FATAL EXCEPTION|Process:\s*$([regex]::Escape($PackageName))") {
    throw "Crash buffer contains a fatal app crash:`n$crashes"
}

$result = [ordered]@{
    schema_version = 1
    tested_at = [DateTimeOffset]::Now.ToString("o")
    source_commit = $sourceCommitAtStart
    source_dirty = $sourceDirtyAtStart
    avd = ((& $adb -s $Serial emu avd name) | Select-Object -First 1).Trim()
    serial = $Serial
    android_api = [int](((& $adb -s $Serial shell getprop ro.build.version.sdk) -join "").Trim())
    android_release = ((& $adb -s $Serial shell getprop ro.build.version.release) -join "").Trim()
    device_model = ((& $adb -s $Serial shell getprop ro.product.model) -join "").Trim()
    package = $PackageName
    app_version = [string]$metadata.app_version
    identity_qr_sha256 = $identityHash
    checks = @(
        "cold_start",
        "onboarding_and_home",
        "system_back_to_home",
        "keystore_database_identity_restart",
        "foreground_relay_start_notification_stop",
        "diagnostic_preview_save_and_zip_schema",
        "no_fatal_crash"
    )
    limitations = @(
        "Android Emulator does not prove physical BLE scan/advertise/GATT/MTU behavior.",
        "Direct and multi-hop radio acceptance still requires physical devices."
    )
}
$resultPath = Join-Path $evidence "result.json"
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $resultPath -Encoding utf8

Write-Output "PASS: Android Emulator smoke test completed"
Write-Output "RESULT=$resultPath"
