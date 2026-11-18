<#
.SYNOPSIS
    Baut Installationsprogramm und Update-Pakete von Preset Manager mit Velopack – und veröffentlicht ein Release.

.DESCRIPTION
    Releases entstehen auf GitHub, nicht auf einem privaten PC: Der Release-Workflow (.github/workflows/release.yml)
    führt dieses Skript für jedes Versions-Tag auf einem GitHub-Rechner aus. So lässt sich jedes Release auf seinen
    Quellcode zurückführen. (Aufbau wie in CloudDrive-Sync.)

    Bauen:
    1. Das Programm als Release-Build (eine .exe, nichts weiter zu installieren, keine Administratorrechte nötig).
    2. Das neueste Release von GitHub holen, damit Velopack ein kleines Delta-Update erzeugen kann.
    3. Installationsprogramm, portable ZIP, Update-Pakete und den Feed (releases.win.json) packen.

    Die Version kommt aus Cargo.toml, der Release-Text aus ihrem Abschnitt in CHANGELOG.md. Versionen mit Zusatz wie
    „-beta.1“ werden Vorabversionen – Testversionen, die nur bekommt, wer „Testversionen erhalten“ eingeschaltet hat.

.PARAMETER Publish
    Auf dem PC: setzt das Tag „v<Version>“ auf den aktuellen Commit und pusht es; der Release-Workflow baut und
    veröffentlicht dann. Der Commit muss schon auf GitHubs main-Zweig liegen.
    Im Release-Workflow: baut und legt das GitHub-Release für das Tag an.

.PARAMETER OutputDir
    Wohin die gebauten Dateien kommen (Standard: %USERPROFILE%\.preset-manager-build\releases).

.PARAMETER Version
    Nur für Probe-Builds: eine andere Paketversion als in Cargo.toml (z. B. 1.1.0-probe.1), um Installation und
    Update lokal mit APM_UPDATE_DIR zu testen. Holt kein Release von GitHub.

.EXAMPLE
    scripts/New-Release.ps1
    Ein Probe-Build. Nichts wird veröffentlicht.

.EXAMPLE
    scripts/New-Release.ps1 -Publish
    Setzt und pusht das Versions-Tag; GitHub baut und veröffentlicht. Braucht Git mit Anmeldung bei GitHub.
#>
param(
    [switch]$Publish,
    [string]$OutputDir,
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$repository = 'suebi76/preset-manager'
$repoUrl = "https://github.com/$repository"
$inWorkflow = $env:GITHUB_ACTIONS -eq 'true'
$packId = 'PresetManagerApp'   # Installationsordner %LOCALAPPDATA%\PresetManagerApp – getrennt von den Sicherungen

function Get-CargoVersion {
    $metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
    ($metadata.packages | Where-Object name -eq 'preset_manager').version
}

# Der Abschnitt dieser Version in CHANGELOG.md, ohne Überschrift.
function Get-ReleaseNotes([string]$Version) {
    $lines = Get-Content (Join-Path $root 'CHANGELOG.md') -Encoding utf8
    $start = [Array]::FindIndex([string[]]$lines, [Predicate[string]] { param($l) $l.StartsWith("## [$Version]", [StringComparison]::Ordinal) })
    if ($start -lt 0) { throw "CHANGELOG.md hat keinen Abschnitt für $Version." }
    $end = [Array]::FindIndex([string[]]$lines, $start + 1, [Predicate[string]] { param($l) $l -like '## `[*' })
    if ($end -lt 0) { $end = $lines.Count }
    ($lines[($start + 1)..($end - 1)] -join "`n").Trim()
}

function Invoke-Checked([string]$What, [scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What ist gescheitert (Exit-Code $LASTEXITCODE)." }
}

$probe = [bool]$Version
if ($probe -and $Publish) { throw '-Version ist nur für Probe-Builds; veröffentlicht wird die Version aus Cargo.toml.' }
if (-not $probe) { $Version = Get-CargoVersion }
$tag = "v$Version"
$isTestVersion = $Version.Contains('-')
Write-Host "Preset Manager $Version$(if ($isTestVersion) { ' – Testversion' })"
$notes = if ($probe) { "Probe-Build $Version" } else { Get-ReleaseNotes $Version }

# Auf dem PC heißt Veröffentlichen: Commit taggen und GitHub das Release bauen lassen.
if ($Publish -and -not $inWorkflow) {
    if (git -C $root status --porcelain) { throw 'Es gibt nicht committete Änderungen. Bitte erst committen.' }
    Invoke-Checked 'git fetch' { git -C $root fetch -q origin main }
    git -C $root merge-base --is-ancestor HEAD origin/main
    if ($LASTEXITCODE -ne 0) { throw 'Dieser Commit ist noch nicht auf GitHubs main-Zweig. Bitte erst pushen.' }
    Invoke-Checked 'git tag' { git -C $root tag -a $tag -m "Preset Manager $Version" }
    Invoke-Checked 'git push' { git -C $root push -q origin $tag }
    Write-Host "Tag $tag gepusht. GitHub baut und veröffentlicht: $repoUrl/actions/workflows/release.yml"
    return
}
# Im Release-Workflow muss das auslösende Tag diese Version nennen.
if ($Publish -and $env:GITHUB_REF_NAME -ne $tag) { throw "Der Workflow läuft für '$env:GITHUB_REF_NAME', die Version ist aber $Version (Tag $tag)." }

$build = Join-Path $env:USERPROFILE '.preset-manager-build'
$programDir = Join-Path $build 'release-program'
$releaseDir = if ($OutputDir) { [IO.Path]::GetFullPath($OutputDir) } else { Join-Path $build 'releases' }
$notesFile = Join-Path $build 'release-notes.md'

# 1. Das Programm.
Invoke-Checked 'cargo build' { cargo build --release --locked --manifest-path (Join-Path $root 'Cargo.toml') -p preset_manager }
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
if (Test-Path $programDir) { Remove-Item $programDir -Recurse -Force }
New-Item -ItemType Directory -Force $programDir | Out-Null
Copy-Item (Join-Path $targetDir 'release\preset_manager.exe') (Join-Path $programDir 'PresetManager.exe')
Copy-Item (Join-Path $targetDir 'release\preset_cli.exe') $programDir
Copy-Item (Join-Path $root 'LICENSE') $programDir

# Ein Probe-Build legt seine Pakete neben frühere Probe-Builds, damit Updates dazwischen funktionieren.
if (-not $probe -and (Test-Path $releaseDir)) { Remove-Item $releaseDir -Recurse -Force }
New-Item -ItemType Directory -Force $releaseDir | Out-Null

Push-Location $root
try {
    Invoke-Checked 'dotnet tool restore' { dotnet tool restore | Out-Null }

    # 2. Das neueste Release auf GitHub, für ein Delta-Update. Das erste Velopack-Release hat keins.
    if (-not $probe) {
        $download = @('vpk', 'download', 'github', '--repoUrl', $repoUrl, '--outputDir', $releaseDir)
        if ($isTestVersion) { $download += '--pre' }
        if ($env:GH_TOKEN) { $download += @('--token', $env:GH_TOKEN) }
        dotnet @download
        if ($LASTEXITCODE -ne 0) { Write-Host 'Kein früheres Release gefunden – nur das vollständige Paket wird gebaut.' }
        if (Test-Path (Join-Path $releaseDir "$packId-$Version-full.nupkg")) {
            if ($Publish) { throw "Version $Version ist schon veröffentlicht. Bitte die Version in Cargo.toml erhöhen." }
            Write-Host "Version $Version ist schon veröffentlicht – Probe-Build ohne Delta-Update."
            Get-ChildItem $releaseDir | Remove-Item -Recurse -Force
        }
    }

    # 3. Installationsprogramm, Pakete, Feed.
    New-Item -ItemType Directory -Force $build | Out-Null
    $notes | Set-Content $notesFile -Encoding utf8
    Invoke-Checked 'vpk pack' {
        dotnet vpk pack --packId $packId --packVersion $Version --packDir $programDir --mainExe 'PresetManager.exe' --runtime win-x64 `
            --packTitle 'Preset Manager' --packAuthors 'Steffen Schwabe' `
            --icon (Join-Path $root 'crates\preset_manager\assets\icon.ico') --releaseNotes $notesFile --outputDir $releaseDir
    }
}
finally {
    Pop-Location
}

# Namen, die man wiedererkennt.
$setup = Join-Path $releaseDir 'PresetManager-Setup.exe'
$portable = Join-Path $releaseDir 'PresetManager-Portable.zip'
Move-Item (Join-Path $releaseDir "$packId-win-Setup.exe") $setup -Force
Move-Item (Join-Path $releaseDir "$packId-win-Portable.zip") $portable -Force
$files = @($setup, $portable, (Join-Path $releaseDir 'releases.win.json')) +
    @(Get-ChildItem $releaseDir -Filter "$packId-$Version-*.nupkg" | ForEach-Object FullName)
Write-Host "Gebaut:`n  $(($files | ForEach-Object { Split-Path $_ -Leaf }) -join "`n  ")"

# 4. Veröffentlichen (im Release-Workflow).
if ($Publish) {
    $release = @('release', 'create', $tag, '--repo', $repository, '--verify-tag', '--title', "Preset Manager $Version", '--notes-file', $notesFile)
    if ($isTestVersion) { $release += '--prerelease' }
    Invoke-Checked 'gh release create' { gh @release @files }
    Write-Host "Veröffentlicht: $repoUrl/releases/tag/$tag"
}
