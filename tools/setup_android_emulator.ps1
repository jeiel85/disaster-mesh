[CmdletBinding()]
param(
    [ValidateSet("Create", "Start", "Stop", "Status", "All")]
    [string]$Action = "All",
    [string]$AvdName = "disastermesh-api36-smoke",
    [int]$Port = 5554,
    [int]$BootTimeoutSeconds = 240,
    [switch]$Window,
    [switch]$WipeData
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$LASTEXITCODE = 0

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
        if ($java) {
            return $java.Directory.Parent.FullName
        }
    }
    $command = Get-Command java.exe -ErrorAction SilentlyContinue
    if ($command) {
        return (Split-Path -Parent (Split-Path -Parent $command.Source))
    }
    throw "JDK 17 not found. Set JAVA_HOME before running this script."
}

$sdk = Find-AndroidSdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_HOME = $sdk
$env:JAVA_HOME = Find-JavaHome
$env:PATH = "$(Join-Path $env:JAVA_HOME 'bin');$(Join-Path $sdk 'platform-tools');$(Join-Path $sdk 'emulator');$env:PATH"

$adb = Join-Path $sdk "platform-tools\adb.exe"
$emulator = Join-Path $sdk "emulator\emulator.exe"
$sdkManager = Join-Path $sdk "cmdline-tools\latest\bin\sdkmanager.bat"
$avdManager = Join-Path $sdk "cmdline-tools\latest\bin\avdmanager.bat"
$imagePackage = "system-images;android-36;default;x86_64"
$serial = "emulator-$Port"

function Get-RunningAvdName {
    $line = & $adb devices | Select-String "^$([regex]::Escape($serial))\s+device$" | Select-Object -First 1
    if (-not $line) { return $null }
    $name = & $adb -s $serial emu avd name 2>$null | Select-Object -First 1
    if ($LASTEXITCODE -ne 0) { return $null }
    return ([string]$name).Trim()
}

function Ensure-Avd {
    if (-not (Test-Path -LiteralPath $sdkManager) -or -not (Test-Path -LiteralPath $avdManager)) {
        throw "Android command-line tools 'latest' are required under $sdk."
    }
    $imagePath = Join-Path $sdk "system-images\android-36\default\x86_64\system.img"
    if (-not (Test-Path -LiteralPath $imagePath)) {
        Write-Host "Installing $imagePackage"
        1..20 | ForEach-Object { "y" } | & $sdkManager $imagePackage
        if ($LASTEXITCODE -ne 0) { throw "sdkmanager failed to install $imagePackage" }
    }
    $avds = @(& $emulator -list-avds)
    if ($avds -notcontains $AvdName) {
        Write-Host "Creating AVD $AvdName"
        "no" | & $avdManager create avd --name $AvdName --package $imagePackage --device pixel_6
        if ($LASTEXITCODE -ne 0) { throw "avdmanager failed to create $AvdName" }
    }
    Write-Host "AVD_READY=$AvdName"
}

function Start-Avd {
    Ensure-Avd
    $running = Get-RunningAvdName
    if ($running -and $running -ne $AvdName) {
        throw "$serial is already used by AVD '$running'. Choose another -Port."
    }
    if (-not $running) {
        $arguments = @(
            "-avd", $AvdName,
            "-port", "$Port",
            "-no-audio",
            "-no-boot-anim",
            "-no-snapshot",
            "-gpu", "swiftshader_indirect",
            "-memory", "3072"
        )
        if (-not $Window) { $arguments += "-no-window" }
        if ($WipeData) { $arguments += "-wipe-data" }
        $start = @{
            FilePath = $emulator
            ArgumentList = $arguments
            PassThru = $true
        }
        if (-not $Window) { $start.WindowStyle = "Hidden" }
        $process = Start-Process @start
        Write-Host "EMULATOR_PID=$($process.Id)"
    }

    $deadline = [DateTimeOffset]::Now.AddSeconds($BootTimeoutSeconds)
    do {
        $state = (& $adb -s $serial get-state 2>$null) -join ""
        if ($state.Trim() -eq "device") {
            $booted = ((& $adb -s $serial shell getprop sys.boot_completed 2>$null) -join "").Trim()
            if ($booted -eq "1") {
                & $adb -s $serial shell settings put global window_animation_scale 0 | Out-Null
                & $adb -s $serial shell settings put global transition_animation_scale 0 | Out-Null
                & $adb -s $serial shell settings put global animator_duration_scale 0 | Out-Null
                & $adb -s $serial shell wm dismiss-keyguard | Out-Null
                Write-Host "AVD_BOOTED=$AvdName"
                Write-Output "SERIAL=$serial"
                return
            }
        }
        Start-Sleep -Seconds 2
    } while ([DateTimeOffset]::Now -lt $deadline)
    throw "AVD $AvdName did not finish booting within $BootTimeoutSeconds seconds."
}

function Stop-Avd {
    $running = Get-RunningAvdName
    if (-not $running) {
        Write-Host "AVD_STOPPED=$AvdName"
        return
    }
    if ($running -ne $AvdName) {
        throw "$serial belongs to '$running', not '$AvdName'."
    }
    & $adb -s $serial emu kill | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to stop $serial" }
    Write-Host "AVD_STOPPED=$AvdName"
}

function Show-Status {
    $running = Get-RunningAvdName
    if (-not $running) {
        Write-Output "STATUS=stopped"
        return
    }
    $api = ((& $adb -s $serial shell getprop ro.build.version.sdk) -join "").Trim()
    $booted = ((& $adb -s $serial shell getprop sys.boot_completed) -join "").Trim()
    Write-Output "STATUS=running AVD=$running SERIAL=$serial API=$api BOOTED=$booted"
}

switch ($Action) {
    "Create" { Ensure-Avd }
    "Start" { Start-Avd }
    "Stop" { Stop-Avd }
    "Status" { Show-Status }
    "All" { Ensure-Avd; Start-Avd }
}
