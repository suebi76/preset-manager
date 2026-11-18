# 01 – Anforderungen

Priorität: **M** = muss in Version 1.0, **S** = soll (1.x), **K** = kann (später).

## Funktionale Anforderungen

### Installieren (Import)

| ID | Anforderung | Prio |
|---|---|---|
| F-01 | Ordner per Drag & Drop auf das Fenster ziehen | M |
| F-02 | ZIP-Datei per Drag & Drop auf das Fenster ziehen | M |
| F-03 | Ordner oder ZIP über Auswahldialog wählen | M |
| F-04 | Mehrere Quellen gleichzeitig annehmen | M |
| F-05 | Quelle rekursiv nach Presets durchsuchen (`.xmp`), auch in verschachtelten ZIPs | M |
| F-06 | Nur echte Camera-Raw-Presets übernehmen (XMP mit `crs:PresetType`), XMP-Sidecars von Fotos ignorieren | M |
| F-07 | Vor dem Kopieren eine Vorschau zeigen: was wird installiert, wohin, welche Konflikte | M |
| F-08 | Konflikte behandeln: überspringen / ersetzen / beide behalten | M |
| F-09 | Presetgruppe beim Import festlegen oder aus der Datei übernehmen | S |
| F-10 | Kameraprofile (`.dcp`) nach `CameraProfiles` installieren | S |
| F-11 | Alte Lightroom-Presets (`.lrtemplate`) nach `Lightroom\Develop Presets` installieren | K |

### Verwalten

| ID | Anforderung | Prio |
|---|---|---|
| F-20 | Installierte Presets als Baum anzeigen: Gruppe → Presets | M |
| F-21 | Mehrfachauswahl (Strg+Klick, Umschalt+Klick, Strg+A) | M |
| F-22 | Presets löschen (einzeln / mehrere) – in den Papierkorb | M |
| F-23 | Ganze Gruppen löschen | M |
| F-24 | Preset umbenennen (ändert `crs:Name` und Dateinamen) | M |
| F-25 | Gruppe umbenennen (ändert `crs:Group` in allen enthaltenen Presets) | M |
| F-26 | Presets in andere Gruppe verschieben | M |
| F-27 | Reihenfolge innerhalb einer Gruppe festlegen (Drag & Drop oder hoch/runter) | M |
| F-28 | Suche/Filter nach Name | **M** (vorher S) |
| F-29 | Letzte Aktion rückgängig machen | S |
| F-30 | Dubletten erkennen (gleiche `crs:UUID`) | S |

### Verwalten – erweitert (Wünsche vom 17.11.2026)

Ziel: alles, was man braucht, um Presets und Profile bequem zu ordnen –
ohne Camera Raw öffnen zu müssen.

| ID | Anforderung | Prio |
|---|---|---|
| F-31 | **Art auf einen Blick:** jeder Eintrag zeigt per Symbol und Farbe, was er ist – Preset, Profil, Kameraprofil (`.dcp`), altes Lightroom-Preset (`.lrtemplate`), unbekannt oder defekt | M |
| F-32 | **Presets und Profile getrennt** anzeigen, wie Camera Raw es tut (Vorgaben-Liste vs. Profilbrowser); zusätzlich Filter „Alle / Presets / Profile“ | M |
| F-33 | **Neue Gruppe anlegen** – leer über „＋“ oder direkt aus der Auswahl („Neue Gruppe aus Auswahl“, Strg+G) | M |
| F-34 | **Ziehen & Ablegen** von Presets und Profilen in eine andere Gruppe, auch mehrere auf einmal; ungültige Ziele (Profil in eine Preset-Gruppe) werden sichtbar abgelehnt | M |
| F-35 | **Gruppen zusammenführen**: Gruppe auf eine andere ziehen | S |
| F-36 | Preset oder Profil **duplizieren** (neue UUID, Name „… Kopie“) | S |
| F-37 | Auswahl oder Gruppe **als ZIP exportieren** – zum Sichern oder Weitergeben | S |
| F-38 | **Mehrere umbenennen**: Text voranstellen/anhängen, Suchen & Ersetzen, durchnummerieren | S |
| F-39 | **Details**: welche Einstellungen ein Preset ändert (z. B. „Belichtung +0,50 · Kontrast +10 · Profil: Adobe Farbe“) | K |
| F-45 | **Im Explorer zeigen** für jeden Eintrag | M |
| F-12 | Ordner/ZIP **direkt auf eine Gruppe ziehen** → wird in diese Gruppe importiert (setzt F-09 um) | M |

**Was Camera Raw vorgibt – und was daraus folgt:**

- **Leere Gruppen gibt es in Camera Raw nicht.** Eine Gruppe ist nur ein
  Eintrag in den Preset-Dateien. Eine neu angelegte, leere Gruppe existiert
  deshalb zunächst nur in der App (gestrichelt dargestellt, Hinweis „Zieh
  Presets hierher“) und wird sichtbar, sobald etwas hineingezogen wird. Die
  App merkt sich leere Gruppen bis dahin in ihren Einstellungen.
- **Presets und Profile haben getrennte Gruppen.** Eine Preset-Gruppe
  „Sommer“ und eine Profil-Gruppe „Sommer“ sind in Camera Raw zwei
  verschiedene Dinge an zwei verschiedenen Stellen. Die App behandelt sie
  ebenso getrennt (Umbenennen, Löschen, Namensprüfung).
- **Die Reihenfolge der Gruppen** legt Camera Raw selbst fest (alphabetisch).
  Die App kann nur die Reihenfolge **innerhalb** einer Gruppe ändern.
- **Favoriten und ausgeblendete Presets** speichert Adobe in eigenen
  Programmeinstellungen, nicht in den Preset-Dateien. Die App fasst sie nicht
  an (siehe „Bewusst nicht im Umfang“).

### Allgemein

| ID | Anforderung | Prio |
|---|---|---|
| F-40 | Adobe-Ordner automatisch finden; erkannte Anwendungen anzeigen | M |
| F-41 | Hinweis, wenn Photoshop/Lightroom läuft (Änderungen erst nach Neustart sichtbar) | M |
| F-42 | Ergebnisbericht nach jeder Aktion: x erfolgreich, y übersprungen, z Fehler mit Grund | M |
| F-43 | Zielordner manuell überschreibbar (Einstellungen) | S |
| F-47 | Alle Camera-Raw-Ordner finden (Benutzerordner, Ordner neben Lightroom-Katalogen) und im Einstellungsdialog zur Wahl stellen; Warnung, wenn Lightroom Classic Presets mit dem Katalog speichert; Adobes mitgelieferte Presets (`%ProgramData%`) nie verwalten | S |
| F-44 | Oberfläche auf Deutsch und Englisch; jeder Text steht zweisprachig (`texts.rs` bzw. `tr("…", "…")` am Ort) | M |
| F-46 | Sprache im Einstellungsdialog wählbar; Voreinstellung: Sprache von Windows | M |

## Nicht-funktionale Anforderungen

| ID | Anforderung |
|---|---|
| N-01 | **Verständlicher Quellcode:** kleine Module mit je einer Aufgabe, sprechende Namen, Doc-Kommentar an jedem öffentlichen Element, kein `unsafe` |
| N-02 | **Kein Datenverlust:** nichts wird endgültig gelöscht oder ohne Sicherung überschrieben |
| N-03 | **Kein Absturz bei Fehlern:** kein `unwrap()`/`expect()`/`panic!` außerhalb von Tests; Fehler werden dem Nutzer verständlich gezeigt |
| N-04 | **Reaktionsfähig:** Dateioperationen laufen im Hintergrund, die Oberfläche friert nicht ein |
| N-05 | **Offline:** die App braucht kein Internet und sendet nichts |
| N-06 | **GitHub-tauglich:** Standard-Cargo-Workspace, README, CI (fmt, clippy, test), `.gitignore` |
| N-07 | Zielplattform Windows 10/11 (x64); pfadabhängiger Code gekapselt, damit macOS nachrüstbar ist |

## Bewusst nicht im Umfang

- Presets bearbeiten (Regler ändern) oder Vorschaubilder rendern
- Favoriten und ausgeblendete Presets in Camera Raw/Lightroom – liegen in
  Adobes Programmeinstellungen, nicht in den Dateien; ein Fehler dort könnte
  Adobes Einstellungen beschädigen
- Reihenfolge der Gruppen – legt Camera Raw alphabetisch fest
- Photoshop-eigene Presets (Pinsel, Aktionen, Stile) – eigenes Format, eigener Ordner je Version
- Lightroom (Cloud-Version): speichert Presets in einer synchronisierten Datenbank, nicht als Dateien, und ist auf diesem Rechner nicht installiert
- LUT-Dateien (`.cube`) – Camera Raw kann sie nur über den eigenen Profil-Dialog einbinden
- Automatische Updates
