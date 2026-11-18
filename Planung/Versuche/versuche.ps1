<#
.SYNOPSIS
    Legt Testpresets fuer die Versuche P-1 bis P-5 an, aendert sie und entfernt sie wieder.

.DESCRIPTION
    Alle Testpresets tragen Gruppen, die mit "APM-Test" beginnen, und feste
    UUIDs, die mit "A9A9" beginnen. Das Skript fasst nur Dateien an, die es
    selbst anlegt (Liste $TestFiles). Echte Presets werden nicht beruehrt.

    Ablauf und Auswertung: siehe Protokoll.md im selben Ordner.

.PARAMETER Schritt
    Installieren  - Testpresets in CameraRaw\Settings anlegen
    Umbenennen    - fuer P-3: Namen von "P3 Vorher" auf "P3 Nachher" aendern
    Status        - zeigen, welche Testdateien vorhanden sind
    Entfernen     - alle Testdateien und den Testunterordner loeschen

.EXAMPLE
    .\versuche.ps1 -Schritt Installieren
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('Installieren', 'Umbenennen', 'Status', 'Entfernen')]
    [string] $Schritt
)

$ErrorActionPreference = 'Stop'

$Settings = Join-Path $env:APPDATA 'Adobe\CameraRaw\Settings'
$SubFolder = Join-Path $Settings 'APM-Test-Unterordner'

# Jede Testdatei: relativer Pfad, angezeigter Name, Gruppe, UUID, Sortierschluessel.
$TestFiles = @(
    # P-1: Liest Camera Raw Unterordner? Kontrolle liegt flach, Testdatei im Unterordner.
    @{ File = 'APM-Test P1 Kontrolle.xmp';                Name = 'P1 Kontrolle (flach)';     Group = 'APM-Test P1';    Uuid = 'A9A90000000000000000000000000101'; Sort = '' }
    @{ File = 'APM-Test-Unterordner\APM-Test P1 Sub.xmp'; Name = 'P1 Unterordner';           Group = 'APM-Test P1';    Uuid = 'A9A90000000000000000000000000102'; Sort = '' }

    # P-2: Beachtet Camera Raw crs:SortName? Alphabetisch A-B-C, SortName umgekehrt 3-2-1.
    @{ File = 'APM-Test P2 A.xmp';                        Name = 'A Erster';                 Group = 'APM-Test P2';    Uuid = 'A9A90000000000000000000000000201'; Sort = '3' }
    @{ File = 'APM-Test P2 B.xmp';                        Name = 'B Zweiter';                Group = 'APM-Test P2';    Uuid = 'A9A90000000000000000000000000202'; Sort = '2' }
    @{ File = 'APM-Test P2 C.xmp';                        Name = 'C Dritter';                Group = 'APM-Test P2';    Uuid = 'A9A90000000000000000000000000203'; Sort = '1' }

    # P-2 (gut ablesbar): gleiche Logik, aber stark unterschiedlich lange Namen, damit die
    # Reihenfolge auch bei kleiner Schrift erkennbar ist. Alphabetisch: kurz, mittel, lang.
    @{ File = 'APM-Test P2L A.xmp';                       Name = 'A';                        Group = 'APM-Test P2L';   Uuid = 'A9A90000000000000000000000000211'; Sort = '3' }
    @{ File = 'APM-Test P2L B.xmp';                       Name = 'B mittellanger Name';      Group = 'APM-Test P2L';   Uuid = 'A9A90000000000000000000000000212'; Sort = '2' }
    @{ File = 'APM-Test P2L C.xmp';                       Name = 'C sehr langer Name zum Erkennen der Reihenfolge'; Group = 'APM-Test P2L'; Uuid = 'A9A90000000000000000000000000213'; Sort = '1' }

    # P-3: Wird eine Namensaenderung nach Neustart uebernommen? Siehe Schritt "Umbenennen".
    @{ File = 'APM-Test P3.xmp';                          Name = 'P3 Vorher';                Group = 'APM-Test P3';    Uuid = 'A9A90000000000000000000000000301'; Sort = '' }

    # P-4: Was passiert bei gleicher UUID? Beide Dateien haben dieselbe.
    @{ File = 'APM-Test P4 Original.xmp';                 Name = 'P4 Original';              Group = 'APM-Test P4';    Uuid = 'A9A90000000000000000000000000401'; Sort = '' }
    @{ File = 'APM-Test P4 Kopie.xmp';                    Name = 'P4 Kopie (gleiche UUID)';  Group = 'APM-Test P4';    Uuid = 'A9A90000000000000000000000000401'; Sort = '' }

    # P-5: Werden Gruppen zusammengefasst, die sich nur in der Schreibweise unterscheiden?
    @{ File = 'APM-Test P5 klein.xmp';                    Name = 'P5 aus Gruppe Gross';      Group = 'APM-Test P5 Gross'; Uuid = 'A9A90000000000000000000000000501'; Sort = '' }
    @{ File = 'APM-Test P5 GROSS.xmp';                    Name = 'P5 aus Gruppe GROSS';      Group = 'APM-TEST P5 GROSS'; Uuid = 'A9A90000000000000000000000000502'; Sort = '' }
)

function New-PresetXml {
    param([string] $Name, [string] $Group, [string] $Uuid, [string] $Sort)

    # Leichte, gut sichtbare Wirkung: etwas heller, etwas waermer.
    @"
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Preset Manager - Versuch">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:PresetType="Normal"
   crs:Cluster=""
   crs:UUID="$Uuid"
   crs:SupportsAmount="False"
   crs:SupportsColor="True"
   crs:SupportsMonochrome="True"
   crs:SupportsHighDynamicRange="True"
   crs:SupportsNormalDynamicRange="True"
   crs:SupportsSceneReferred="True"
   crs:SupportsOutputReferred="True"
   crs:RequiresRGBTables="False"
   crs:ShowInPresets="True"
   crs:ShowInQuickActions="False"
   crs:CameraModelRestriction=""
   crs:Copyright=""
   crs:ContactInfo=""
   crs:Version="18.1"
   crs:ProcessVersion="15.4"
   crs:Exposure2012="+0.30"
   crs:Temperature="+8"
   crs:HasSettings="True">
   <crs:Name>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">$Name</rdf:li>
    </rdf:Alt>
   </crs:Name>
   <crs:ShortName>
    <rdf:Alt>
     <rdf:li xml:lang="x-default"/>
    </rdf:Alt>
   </crs:ShortName>
   <crs:SortName>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">$Sort</rdf:li>
    </rdf:Alt>
   </crs:SortName>
   <crs:Group>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">$Group</rdf:li>
    </rdf:Alt>
   </crs:Group>
   <crs:Description>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">Testpreset des Preset Managers - kann geloescht werden</rdf:li>
    </rdf:Alt>
   </crs:Description>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"@
}

function Write-Utf8NoBom {
    param([string] $Path, [string] $Text)
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Show-AdobeWarning {
    $running = Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ProcessName -match '^(Photoshop|Lightroom)' }
    if ($running) {
        Write-Warning ("Laeuft gerade: " + (($running.ProcessName | Sort-Object -Unique) -join ', ') +
            ". Bitte beenden und danach neu starten, sonst sind Aenderungen nicht sichtbar.")
    }
}

if (-not (Test-Path -LiteralPath $Settings -PathType Container)) {
    throw "Camera-Raw-Ordner nicht gefunden: $Settings"
}

switch ($Schritt) {
    'Installieren' {
        Show-AdobeWarning
        New-Item -ItemType Directory -Force -Path $SubFolder | Out-Null
        foreach ($test in $TestFiles) {
            $path = Join-Path $Settings $test.File
            if (Test-Path -LiteralPath $path) {
                Write-Host "vorhanden, uebersprungen: $($test.File)"
                continue
            }
            Write-Utf8NoBom $path (New-PresetXml -Name $test.Name -Group $test.Group -Uuid $test.Uuid -Sort $test.Sort)
            Write-Host "angelegt: $($test.File)"
        }
    }

    'Umbenennen' {
        Show-AdobeWarning
        $path = Join-Path $Settings 'APM-Test P3.xmp'
        if (-not (Test-Path -LiteralPath $path)) { throw "Testdatei fehlt: $path - erst 'Installieren' ausfuehren." }
        $text = [System.IO.File]::ReadAllText($path)
        if ($text -notmatch '>P3 Vorher<') { throw "Erwarteten Namen 'P3 Vorher' nicht gefunden - schon umbenannt?" }
        Write-Utf8NoBom $path ($text -replace '>P3 Vorher<', '>P3 Nachher<')
        Write-Host "P3 umbenannt: 'P3 Vorher' -> 'P3 Nachher' (Dateiname und Index-Dateien unveraendert)"
    }

    'Status' {
        foreach ($test in $TestFiles) {
            $path = Join-Path $Settings $test.File
            $state = if (Test-Path -LiteralPath $path) { 'vorhanden' } else { 'fehlt    ' }
            Write-Host "$state  $($test.File)"
        }
    }

    'Entfernen' {
        foreach ($test in $TestFiles) {
            $path = Join-Path $Settings $test.File
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path
                Write-Host "entfernt: $($test.File)"
            }
        }
        if ((Test-Path -LiteralPath $SubFolder) -and -not (Get-ChildItem -LiteralPath $SubFolder -Force)) {
            Remove-Item -LiteralPath $SubFolder
            Write-Host "entfernt: APM-Test-Unterordner"
        }
    }
}
