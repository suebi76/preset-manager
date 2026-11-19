<#
.SYNOPSIS
    Baut das Paket für den Microsoft Store (MSIX) – oder richtet es zum Ausprobieren lokal ein.

.DESCRIPTION
    Die Store-Fassung ist dasselbe Programm ohne Selbst-Update (cargo --no-default-features): Neue Versionen
    installiert der Store. Microsoft signiert das Paket nach der Prüfung; hier wird nichts signiert.

    1. Das Programm ohne Velopack bauen, mit fest eingebauter C-Laufzeit (keine VC++-Abhängigkeit im Paket).
    2. Den Paketordner zusammenstellen: Programm, Lizenz, Symbole, Manifest (packaging/msix).
    3. resources.pri (welches Symbol bei welcher Größe) und das .msix-Paket mit dem Windows SDK packen.

    Name und Herausgeber kommen aus dem Partner Center (Produkt → Produktidentität). Ohne Angabe entsteht ein
    Testpaket, das sich nur lokal einrichten lässt (-Register), nicht im Store hochladen.

.PARAMETER OutputDir
    Wohin das .msix kommt (Standard: %USERPROFILE%\.preset-manager-build\store).

.PARAMETER IdentityName
    Package/Identity/Name aus dem Partner Center, z. B. 12345SteffenSchwabe.PresetManager.

.PARAMETER Publisher
    Package/Identity/Publisher aus dem Partner Center, z. B. CN=1A2B3C4D-....

.PARAMETER PublisherDisplayName
    Package/Properties/PublisherDisplayName aus dem Partner Center.

.PARAMETER Register
    Zum Ausprobieren (Versuch P-10): den Paketordner unsigniert als App einrichten. Braucht den Entwicklermodus
    von Windows. Entfernen: Get-AppxPackage *PresetManager* | Remove-AppxPackage

.EXAMPLE
    scripts/New-StorePackage.ps1 -Register
    Testpaket bauen und lokal einrichten; danach steht „Preset Manager“ im Startmenü.

.EXAMPLE
    scripts/New-StorePackage.ps1 -IdentityName 12345SteffenSchwabe.PresetManager -Publisher 'CN=1A2B…' -PublisherDisplayName 'Steffen Schwabe'
    Paket zum Hochladen im Partner Center.
#>
param(
    [string]$OutputDir,
    [string]$IdentityName = 'PresetManager.Test',
    [string]$Publisher = 'CN=Preset Manager Test',
    [string]$PublisherDisplayName = 'Steffen Schwabe',
    [switch]$Register
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

function Invoke-Checked([string]$What, [scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What ist gescheitert (Exit-Code $LASTEXITCODE)." }
}

# makeappx und makepri aus dem neuesten Windows SDK.
function Find-SdkTool([string]$Name) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tool = Get-ChildItem $kits -Directory -Filter '10.*' -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
    if (-not $tool) { throw "$Name nicht gefunden – bitte das Windows SDK installieren." }
    $tool
}

$metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'preset_manager').version
# Der Store kennt nur Versionen aus vier Zahlen, die letzte 0. Testversionen gibt es dort nicht.
if ($version.Contains('-')) { throw "Version $version ist eine Testversion – die gibt es nur auf GitHub." }
$packageVersion = "$version.0"
Write-Host "Preset Manager $version für den Microsoft Store ($IdentityName)"

$build = Join-Path $env:USERPROFILE '.preset-manager-build'
$layout = Join-Path $build 'store-layout'
$outDir = if ($OutputDir) { [IO.Path]::GetFullPath($OutputDir) } else { Join-Path $build 'store' }
$package = Join-Path $outDir "PresetManager-$version-Store.msix"

# 1. Das Programm. Eigener Zielordner: andere Merkmale und RUSTFLAGS als der GitHub-Build.
$targetDir = Join-Path $root 'target\store'
$previousFlags = $env:RUSTFLAGS
$env:RUSTFLAGS = '-C target-feature=+crt-static'
try {
    Invoke-Checked 'cargo build' {
        cargo build --release --locked --manifest-path (Join-Path $root 'Cargo.toml') -p preset_manager --bin preset_manager `
            --no-default-features --target-dir $targetDir
    }
}
finally {
    $env:RUSTFLAGS = $previousFlags
}

# 2. Der Paketordner. Ein eingerichtetes Testpaket läuft aus diesem Ordner – vorher abmelden.
if (-not $env:GITHUB_ACTIONS) {
    Get-AppxPackage | Where-Object { $_.IsDevelopmentMode -and $_.InstallLocation -eq $layout } | ForEach-Object {
        Write-Host "Bisheriges Testpaket $($_.Name) wird abgemeldet."
        $_ | Remove-AppxPackage
    }
}
if (Test-Path $layout) { Remove-Item $layout -Recurse -Force }
New-Item -ItemType Directory -Force $layout | Out-Null
Copy-Item (Join-Path $targetDir 'release\preset_manager.exe') (Join-Path $layout 'PresetManager.exe')
Copy-Item (Join-Path $root 'LICENSE') (Join-Path $layout 'LICENSE.txt')
Copy-Item (Join-Path $root 'packaging\msix\Assets') (Join-Path $layout 'Assets') -Recurse

$escape = { param([string]$value) [Security.SecurityElement]::Escape($value) }
$manifest = (Get-Content (Join-Path $root 'packaging\msix\AppxManifest.xml') -Raw -Encoding utf8).
    Replace('{{IDENTITY_NAME}}', (& $escape $IdentityName)).
    Replace('{{PUBLISHER}}', (& $escape $Publisher)).
    Replace('{{PUBLISHER_DISPLAY_NAME}}', (& $escape $PublisherDisplayName)).
    Replace('{{VERSION}}', $packageVersion)
if ($manifest -match '{{[A-Z_]+}}') { throw 'Im Manifest ist ein Platzhalter übrig geblieben.' }
[IO.File]::WriteAllText((Join-Path $layout 'AppxManifest.xml'), $manifest, [Text.UTF8Encoding]::new($false))

# 3. resources.pri und Paket. Die Konfiguration liegt außerhalb des Paketordners, sonst landet sie im Paket.
$makepri = Find-SdkTool 'makepri.exe'
$makeappx = Find-SdkTool 'makeappx.exe'
$priConfig = Join-Path $build 'store-priconfig.xml'
Invoke-Checked 'makepri createconfig' { & $makepri createconfig /cf $priConfig /dq de-DE_en-US /pv 10.0.0 /o | Out-Null }
# Ein einzelnes Paket, kein Bündel: alle Größen in eine resources.pri statt je Größe eine eigene.
$config = [xml](Get-Content $priConfig -Raw -Encoding utf8)
$config.SelectNodes('//packaging') | ForEach-Object { [void]$_.ParentNode.RemoveChild($_) }
$config.Save($priConfig)
Invoke-Checked 'makepri new' {
    & $makepri new /pr $layout /cf $priConfig /mn (Join-Path $layout 'AppxManifest.xml') /of (Join-Path $layout 'resources.pri') /o | Out-Null
}
New-Item -ItemType Directory -Force $outDir | Out-Null
Invoke-Checked 'makeappx pack' { & $makeappx pack /d $layout /p $package /o | Out-Null }
Write-Host "Gebaut: $package"

if ($Register) {
    Add-AppxPackage -Register (Join-Path $layout 'AppxManifest.xml')
    Write-Host 'Eingerichtet: „Preset Manager“ im Startmenü (Testpaket).'
}
