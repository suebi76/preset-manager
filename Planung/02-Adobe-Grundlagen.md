# 02 – Adobe-Grundlagen

Dieses Dokument hält fest, wie Adobe Presets ablegt. Alles unter „Geprüft“ wurde
am 17.11.2026 auf dem Entwicklungsrechner nachgesehen; alles unter „Noch zu
prüfen“ ist eine Annahme, die in Meilenstein 0 mit einem Versuch bestätigt wird.

## Ordner

| Zweck | Pfad (Windows) | Dateityp |
|---|---|---|
| Presets (eigene/kopierte) | `%APPDATA%\Adobe\CameraRaw\Settings` | `.xmp` |
| Presets (über den Adobe-Importdialog) | `%APPDATA%\Adobe\CameraRaw\ImportedSettings` | `.xmp` |
| Kameraprofile | `%APPDATA%\Adobe\CameraRaw\CameraProfiles` | `.dcp` |
| Alte Lightroom-Presets | `%APPDATA%\Adobe\Lightroom\Develop Presets` | `.lrtemplate` |

Dieser Ordner wird von **Photoshop, Photoshop Beta und Lightroom Classic
gemeinsam** benutzt. Es gibt keine Kopie je Anwendung oder je Version.

## Geprüft auf diesem Rechner

- `Settings` enthält 19 `.xmp`-Dateien flach im Ordner, dazu einen Unterordner
  `LUTs` und zwei `Index_*.dat`-Dateien.
- `ImportedSettings` enthält 391 `.xmp`-Dateien, überwiegend in Unterordnern je
  Presetpaket (z. B. `Sommerlicht Presets\Sommerlicht - Bondi.xmp`).
- Installiert sind Photoshop 2021–2027 und Photoshop (Beta) sowie Lightroom
  Classic. Lightroom (Cloud) ist nicht installiert.
- In `Lightroom\Develop Presets` liegen `.lrtemplate`-Dateien mit Präfix `~~`.
  Das Präfix setzt Lightroom Classic, nachdem es die Datei selbst nach XMP
  konvertiert hat.

### Aufbau einer Preset-Datei

Ein Preset ist eine XML-Datei (XMP). Die für uns wichtigen Felder:

```xml
<rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    crs:PresetType="Normal"
    crs:UUID="487EBEDAE9D9E245B4290E9A20030BF5"
    crs:Exposure2012="0.00" ...>
  <crs:Name>
    <rdf:Alt><rdf:li xml:lang="x-default">RT 01 - Basis - Hart</rdf:li></rdf:Alt>
  </crs:Name>
  <crs:SortName>
    <rdf:Alt><rdf:li xml:lang="x-default"/></rdf:Alt>
  </crs:SortName>
  <crs:Group>
    <rdf:Alt><rdf:li xml:lang="x-default">Reisetagebuch - Nord &amp; Süd - RT LR</rdf:li></rdf:Alt>
  </crs:Group>
</rdf:Description>
```

| Feld | Bedeutung | Folge für die App |
|---|---|---|
| `crs:PresetType` | Kennzeichnet die Datei als Preset | Erkennungsmerkmal beim Import (F-06) |
| `crs:UUID` | Eindeutige Kennung | Dublettenerkennung; bei „beide behalten“ neue UUID vergeben |
| `crs:Name` | Angezeigter Name | Umbenennen ändert dieses Feld |
| `crs:Group` | Angezeigte Gruppe | Gruppen sind **nicht** die Ordner auf der Platte |
| `crs:SortName` | Sortierschlüssel | Vorhanden in allen 410 Dateien, überall leer |

Sonderzeichen sind XML-kodiert (`&` → `&amp;`). Die App muss beim Lesen
dekodieren und beim Schreiben kodieren.

### Presets und Profile

`crs:PresetType` hat zwei Werte: `Normal` (267 Dateien) ist ein Preset in der
Preset-Liste, `Look` (143 Dateien) ist ein kreatives Profil im Profilbrowser.
Beide liegen in denselben Ordnern; die App liest beide und unterscheidet sie
über `PresetType`.

### Eingebettete Profile

29 Presets enthalten in `<crs:Look>` ein eingebettetes Profil mit eigenem
`rdf:Description`, eigenem `crs:Name`, `crs:UUID` und `crs:Group`. Diese Felder
gehören zum Profil. Die App wertet nur die Felder des **obersten**
`rdf:Description` aus (abgesichert durch den Test
`ignoriert_felder_eines_eingebetteten_profils`).

### Index-Dateien

`Index_*.dat` ist ein Cache von Camera Raw. Die App fasst diese Dateien nicht
an; Camera Raw baut sie beim nächsten Start selbst neu auf.

## Ergebnisse der Versuche (17.11.2026)

Durchgeführt in Photoshop 2027 (Camera Raw 19.0), Photoshop (Beta) und
Lightroom Classic – alle drei mit demselben Ergebnis. Einzelheiten:
[Versuche/Protokoll.md](Versuche/Protokoll.md).

| Nr. | Ergebnis | Folge für die App |
|---|---|---|
| P-1 | **Ja**, Presets in Unterordnern von `Settings` werden gelesen | Import legt jedes Paket in einen eigenen Unterordner `Settings\<Paketname>\`. Das hält den Ordner übersichtlich und macht Löschen eines Pakets einfach |
| P-2 | **Ja**, `crs:SortName` bestimmt die Reihenfolge innerhalb einer Gruppe | Sortieren schreibt `crs:SortName`; kein Nummernpräfix im Namen nötig. Werte mit führenden Nullen schreiben (`0001`, `0002` …), weil der Vergleich vermutlich textuell ist |
| P-3 | **Ja**, geänderte Namen erscheinen nach Neustart; Camera Raw baut die Index-Datei selbst neu | Die App fasst `Index_*.dat` nicht an. Nach jeder Änderung Hinweis „Photoshop/Lightroom neu starten“ |
| P-4 | Zwei Dateien mit gleicher UUID werden **beide angezeigt** | Dublettenerkennung bleibt „soll“. Beim Import mit „beide behalten“ trotzdem neue UUID vergeben, weil das Verhalten beim Anwenden ungeprüft ist |
| P-5 | **Nein**, Gruppen mit unterschiedlicher Schreibweise bleiben **getrennt** | `Library::from_presets` gruppiert bereits nach exaktem Namen – passt. „Gruppe umbenennen“ ändert nur exakt passende Schreibweise |

## Ursprüngliche Fragen

| Nr. | Frage | Versuch | Wenn nein |
|---|---|---|---|
| P-1 | Liest Camera Raw Presets aus Unterordnern von `Settings`? | Testpreset in `Settings\Test\` ablegen, Photoshop starten | Flach in `Settings` ablegen, Paketname als Dateipräfix |
| P-2 | Bestimmt `crs:SortName` die Reihenfolge in Camera Raw und Lightroom Classic? | Drei Presets mit umgekehrten SortNames anlegen | Reihenfolge über Nummernpräfix in `crs:Name` („01 …“) |
| P-3 | Übernimmt Camera Raw Änderungen an `crs:Name`/`crs:Group` nach Neustart ohne Löschen der Index-Datei? | Preset umbenennen, neu starten | Index-Datei mit Sicherung entfernen |
| P-4 | Was passiert bei zwei Presets mit gleicher UUID? | Datei duplizieren | Entscheidet, ob Dublettenwarnung M statt S wird |
| P-5 | Fasst Camera Raw Gruppen zusammen, die sich nur in Groß-/Kleinschreibung unterscheiden („Leica“/„LEICA“)? | Zwei Presets mit so unterschiedlichen Gruppen anlegen | Wenn ja: `Library::from_presets` muss ebenso zusammenfassen, und „Gruppe umbenennen“ muss beide Schreibweisen ändern |

Für alle Versuche gilt: mit einem selbst erstellten Testpreset arbeiten, nicht
mit den gekauften Paketen.
