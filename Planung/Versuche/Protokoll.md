# Versuche P-1 bis P-5 – Anleitung und Protokoll

Ziel: die offenen Fragen aus [02-Adobe-Grundlagen.md](../02-Adobe-Grundlagen.md)
am echten Photoshop klären. Dauer etwa 10 Minuten.

Die Testpresets erzeugt [versuche.ps1](versuche.ps1). Alle liegen in Gruppen,
die mit **„APM-Test“** beginnen, und lassen sich am Ende mit einem Befehl
entfernen. Echte Presets werden nicht angefasst.

## Ablauf

### 1. Testpresets installieren

Photoshop und Lightroom Classic vorher schließen.

```powershell
& ".\Planung\Versuche\versuche.ps1" -Schritt Installieren
```

### 2. Erste Sichtung in Photoshop

1. Photoshop starten, irgendein Bild öffnen.
2. **Filter → Camera Raw-Filter …** (Strg+Umschalt+A).
3. Rechts auf das Symbol **Vorgaben** (zwei überlappende Kreise) klicken.
4. Nach unten zu den Gruppen „APM-Test …“ scrollen.

Die Ergebnisse unten in die Tabelle eintragen (P-1, P-2, P-4, P-5, P-3 Teil 1).

### 3. Umbenennen für P-3

1. Camera Raw abbrechen, **Photoshop schließen**.
2. Befehl ausführen:

```powershell
& ".\Planung\Versuche\versuche.ps1" -Schritt Umbenennen
```

3. Photoshop neu starten, Camera Raw-Filter öffnen, Gruppe „APM-Test P3“ ansehen.

### 4. Gegenprobe in Photoshop (Beta) und Lightroom Classic (optional, aber hilfreich)

- **Photoshop (Beta):** wie Schritt 2.
- **Lightroom Classic:** Modul **Entwickeln**, links das Bedienfeld **Vorgaben**.

### 5. Aufräumen

Photoshop und Lightroom schließen, dann:

```powershell
& ".\Planung\Versuche\versuche.ps1" -Schritt Entfernen
```

## Was du sehen solltest – und was es bedeutet

| Nr. | Wo schauen | Wenn du das siehst … | … dann gilt |
|---|---|---|---|
| P-1 | Gruppe „APM-Test P1“ | **beide** Einträge „P1 Kontrolle (flach)“ und „P1 Unterordner“ | Unterordner werden gelesen |
| | | nur „P1 Kontrolle (flach)“ | Unterordner werden **nicht** gelesen |
| P-2 | Gruppe „APM-Test P2“ | Reihenfolge **C, B, A** | `crs:SortName` wird beachtet |
| | | Reihenfolge **A, B, C** | `crs:SortName` wird ignoriert |
| P-3 | Gruppe „APM-Test P3“ nach dem Neustart | „P3 Nachher“ | Änderung wird ohne Weiteres übernommen |
| | | weiterhin „P3 Vorher“ | Camera Raw hält einen alten Stand im Cache |
| P-4 | Gruppe „APM-Test P4“ | beide Einträge | gleiche UUID stört die Anzeige nicht |
| | | nur einer (welcher?) | Dubletten verdrängen sich gegenseitig |
| P-5 | Gruppen „APM-Test P5 …“ | **eine** Gruppe mit beiden Presets | Gruppen werden ohne Rücksicht auf Schreibweise zusammengefasst |
| | | **zwei** Gruppen | Schreibweise trennt Gruppen |

Zusatz zu P-4: Klick auf „P4 Kopie“ und dann auf „P4 Original“. Verhalten
sich beide normal, oder springt die Auswahl / fehlt das Häkchen?

## Ergebnisse (17.11.2026, durchgeführt per Bildschirmsteuerung)

Photoshop 2027 mit Camera Raw 19.0 · Photoshop (Beta) · Lightroom Classic ·
Windows 11, 4K-Monitor

| Nr. | Photoshop 2027 | Photoshop (Beta) | Lightroom Classic | Ergebnis |
|---|---|---|---|---|
| P-1 | beide Einträge | beide Einträge | beide Einträge | **Unterordner werden gelesen** |
| P-2 | C, B, A | C, B, A | C, B, A | **`crs:SortName` wird beachtet** |
| P-2L | lang, mittel, kurz | lang, mittel, kurz | lang, mittel, kurz | bestätigt P-2 eindeutig (siehe unten) |
| P-3 | neuer Name nach Neustart | neuer Name | neuer Name nach Neustart | **Änderungen werden nach Neustart übernommen**, Index-Datei baut Camera Raw selbst neu |
| P-4 | beide Einträge | beide Einträge | beide Einträge | **gleiche UUID stört die Anzeige nicht** (Verhalten beim Anwenden nicht geprüft) |
| P-5 | zwei Gruppen | zwei Gruppen | zwei Gruppen | **Schreibweise trennt Gruppen** |

Bildschirmausschnitte der Vorgabenliste:
[Vorgaben-Photoshop-2027.jpg](Vorgaben-Photoshop-2027.jpg),
[Vorgaben-Photoshop-Beta.jpg](Vorgaben-Photoshop-Beta.jpg),
[Vorgaben-Lightroom-Classic.jpg](Vorgaben-Lightroom-Classic.jpg) (vor dem Umbenennen),
[Vorgaben-Lightroom-Classic-nach-Neustart.jpg](Vorgaben-Lightroom-Classic-nach-Neustart.jpg)
(P-3 nach Umbenennen und Neustart)

### Anmerkungen zur Durchführung

- **P-2L und zweite Umbenennung bei P-3:** Auf dem 4K-Monitor ist die Schrift
  der Vorgabenliste so klein, dass „A/B/C“ und „Vorher/Nachher“ in
  Bildschirmfotos nicht sicher zu lesen waren. Deshalb wurden zusätzlich
  Presets mit stark unterschiedlich langen Namen angelegt (P-2L) und P-3 ein
  zweites Mal auf einen sehr langen Namen umbenannt. Beides war eindeutig
  erkennbar.
- **Lightroom Classic mit eigenem Testkatalog:** Beim ersten Versuch wollte
  Lightroom den Katalog aktualisieren; das wurde abgebrochen. Beim zweiten
  Start öffnete Lightroom den Katalog „Bilder“ ohne Rückfrage. Um ihn nicht
  zu verändern, wurde über „Datei → Neuer Katalog“ ein leerer Testkatalog
  angelegt und nur das Testbild hinzugefügt (ohne Kopieren). Ausgewertet
  wurde das Bedienfeld **Vorgaben** im Modul **Entwickeln**. Vorher wurden die
  Lightroom-Einstellungen gesichert und danach zurückgespielt, damit Lightroom
  wieder mit „Bilder“ startet. Testkatalog und Testbild sind gelöscht.
  Lightroom Classic liest die Presets also aus demselben Ordner und verhält
  sich in allen fünf Punkten wie Camera Raw in Photoshop.
- **Ein Absturz von Photoshop 2027:** Beim zweiten Start stürzte Photoshop
  beim Öffnen der Vorgabenliste per Tastenkürzel ab. Beim dritten und vierten
  Start mit denselben Testdateien trat das nicht wieder auf; das
  Camera-Raw-Protokoll enthält keinen Fehler. Ein Zusammenhang mit den
  Testpresets ist nicht belegt.
- Alle Testpresets wurden danach mit `-Schritt Entfernen` gelöscht; im
  Settings-Ordner sind keine „APM-Test“-Dateien mehr vorhanden.
