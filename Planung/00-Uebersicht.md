# Preset Manager – Planung

Stand: 17.11.2026 · Status: Logik fertig (M1–M3), Oberfläche zum Anzeigen, Importieren und Verwalten gebaut (M4, M5); offen: Abnahme am echten Ordner, M6 (Version 1.0)

## Worum geht es

Eine Windows-Desktop-App in Rust, mit der Presets (als Ordner oder ZIP) per
Drag & Drop oder Auswahldialog installiert werden, sodass sie in Camera Raw
(Photoshop, Photoshop Beta) und Lightroom Classic zur Verfügung stehen. Die
installierten Presets lassen sich in der App verwalten: löschen, umbenennen,
sortieren – einzeln oder per Mehrfachauswahl.

## Dokumente

| Datei | Inhalt |
|---|---|
| [01-Anforderungen.md](01-Anforderungen.md) | Was die App können muss (und was bewusst nicht) |
| [02-Adobe-Grundlagen.md](02-Adobe-Grundlagen.md) | Wo und wie Adobe Presets speichert – auf diesem Rechner geprüft |
| [03-Architektur.md](03-Architektur.md) | Aufbau, Module, Bibliotheken, Entscheidungen |
| [04-Fehlerbehandlung.md](04-Fehlerbehandlung.md) | Fehlerkonzept und Schutz vor Datenverlust |
| [05-Teststrategie.md](05-Teststrategie.md) | Wie wir Qualität sicherstellen |
| [06-Roadmap.md](06-Roadmap.md) | Meilensteine mit Abnahmekriterien |
| [07-Oberflaeche.md](07-Oberflaeche.md) | Gestaltung und Bedienung der Oberfläche (dunkel, im Stil der Adobe-Programme) |

## Die drei wichtigsten Erkenntnisse der Planung

1. **Es gibt nur einen Zielordner, nicht drei.** Photoshop, Photoshop Beta und
   Lightroom Classic lesen Camera-Raw-Presets aus demselben Ordner
   (`%APPDATA%\Adobe\CameraRaw\Settings`). „In alle Anwendungen kopieren“ heißt
   also: einmal dorthin kopieren. Das macht die App einfacher und vermeidet
   Duplikate.
2. **Name und Gruppe stehen in der Datei, nicht im Dateisystem.** Was Camera Raw
   als Presetname und Presetgruppe anzeigt, steht als `crs:Name` und `crs:Group`
   in der `.xmp`-Datei. Umbenennen und Gruppieren bedeutet deshalb: XMP-Datei
   gezielt ändern, nicht nur Datei/Ordner umbenennen.
3. **Die App ändert Nutzerdaten – Sicherheit geht vor.** Löschen geht in den
   Papierkorb, vor jeder Dateiänderung wird gesichert, jede Aktion wird erst als
   Plan angezeigt und dann ausgeführt.

## Durch Versuche geklärt (17.11.2026)

- Camera Raw liest Unterordner von `Settings` → Import legt je Paket einen Unterordner an.
- Camera Raw sortiert nach `crs:SortName` → Sortieren ohne Namenspräfix möglich.
- Änderungen erscheinen nach Neustart von Photoshop bzw. Lightroom; die Index-Dateien bleiben unangetastet.
- Photoshop 2027, Photoshop (Beta) und Lightroom Classic verhalten sich gleich.

Details: [02-Adobe-Grundlagen.md](02-Adobe-Grundlagen.md#ergebnisse-der-versuche-17112026)

## Offene Punkte

- Lizenz für die Veröffentlichung auf GitHub (Vorschlag: MIT).
