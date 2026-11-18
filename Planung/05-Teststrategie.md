# 05 – Teststrategie

## Grundregel

**Tests arbeiten nie im echten Adobe-Ordner.** Alle Funktionen von
`preset_core` bekommen ihre Ordner als Parameter (`AdobeLocations`). Tests
übergeben einen temporären Ordner.

## Ebenen

| Ebene | Was | Wo | Werkzeug |
|---|---|---|---|
| Unit-Tests | einzelne Funktionen: XMP lesen/ändern, Namensprüfung, ZIP-Pfadprüfung | im jeweiligen Modul (`#[cfg(test)]`) | `cargo test` |
| Integrationstests | ganze Abläufe: Ordner importieren, ZIP importieren, Gruppe umbenennen, löschen | `crates/preset_core/tests/` | `cargo test`, `tempfile` |
| Manuelle Abnahme | Oberfläche und echtes Verhalten in Photoshop/Lightroom | Checkliste je Meilenstein | von Hand |
| Statische Prüfung | Format, Lints | CI | `cargo fmt --check`, `cargo clippy` |

Die Oberfläche selbst wird nicht automatisiert getestet. Sie enthält deshalb
keine Logik – sie ruft `preset_core` auf und zeigt Ergebnisse an.

## Testdaten

`crates/preset_core/tests/fixtures/` enthält ausschließlich **selbst erstellte**
Dateien (keine gekauften Presets – Urheberrecht):

| Datei | Zweck |
|---|---|
| `simple.xmp` | normales Preset |
| `special_chars.xmp` | Name und Gruppe mit `&`, `<`, Umlauten, Emoji |
| `no_group.xmp` | Preset ohne Gruppe |
| `sidecar.xmp` | Foto-Sidecar, kein Preset |
| `broken.xmp` | abgeschnittenes XML |
Alle ZIP-Archive – normale (Paket mit Unterordnern, ZIP im ZIP, macOS-ZIP)
wie bösartige (Zip Slip, übergroß, zu tief verschachtelt) – werden in
`tests/import.rs` per Code erzeugt und nicht eingecheckt. So steht im Test
selbst, was im Archiv steckt.

## Pflichtfälle

### XMP
- Lesen liefert Name, Gruppe, UUID, SortName korrekt dekodiert.
- Ändern von Name → erneutes Lesen liefert den neuen Namen.
- Nach dem Ändern sind alle übrigen Attribute und Elemente unverändert
  (Vergleich der Datei mit Ausnahme der geänderten Stelle).
- Namen mit `&`, `<`, `"`, Umlauten überleben Schreiben und Lesen.
- Sidecar wird als `NotAPreset` erkannt, defekte Datei als `MalformedXmp`.

### Import
- Ordner mit Unterordnern: alle Presets gefunden.
- ZIP, verschachteltes ZIP, macOS-ZIP: richtige Anzahl.
- Zip-Slip-Eintrag → `UnsafeArchiveEntry`, nichts außerhalb des Temp-Ordners geschrieben.
- Zu großes Archiv → `ArchiveTooLarge`.
- Jede Konfliktregel (überspringen / ersetzen / beide behalten) liefert das erwartete Ergebnis.
- Quelle ohne Presets → `NoPresetsFound`.
- Eine defekte Datei unter zehn: neun installiert, eine im Bericht.

Zusätzlich, von Hand gestartet und nur lesend:
`cargo test -p preset_core --test real_data -- --ignored` prüft das Ändern an
allen echten Presets des Rechners im Speicher.

### Verwalten
- Löschen entfernt die Datei aus dem Zielordner (Papierkorb im Test durch
  eine austauschbare Implementierung ersetzt).
- Löschen eines Pfads außerhalb der Adobe-Ordner wird abgelehnt.
- Gruppe umbenennen ändert alle Presets der Gruppe und keine anderen.
- Sortieren: nach `reorder` liefert `Library::load` die neue Reihenfolge.
- Umbenennen auf einen vorhandenen Namen → `NameConflict`, nichts verändert.
- Vor jeder Änderung existiert eine Sicherung mit dem Originalinhalt.

## Fehlerkontrolle im Entwicklungsablauf

Jede Änderung durchläuft vor dem Zusammenführen:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. Code-Review (zweite Person bzw. `/code-review`)
5. Manuelle Checkliste des Meilensteins, sofern die Oberfläche betroffen ist

Schritte 1–3 laufen automatisch in GitHub Actions auf `windows-latest`.

## Manuelle Abnahme vor Version 1.0

Auf dem echten Rechner, mit Sicherungskopie von `%APPDATA%\Adobe\CameraRaw`:

- [ ] Ordner per Drag & Drop importieren → in Camera Raw (Photoshop) sichtbar
- [ ] ZIP per Dialog importieren → in Photoshop Beta sichtbar
- [ ] dasselbe Preset in Lightroom Classic sichtbar
- [ ] Preset umbenennen → neuer Name nach Neustart in allen drei Anwendungen
- [ ] Gruppe umbenennen, Presets verschieben, Reihenfolge ändern
- [ ] 20 Presets per Mehrfachauswahl löschen → im Papierkorb, wiederherstellbar
- [ ] Import bei laufendem Photoshop → Hinweis erscheint, nichts bricht
- [ ] Defektes ZIP → verständliche Meldung, kein Absturz
