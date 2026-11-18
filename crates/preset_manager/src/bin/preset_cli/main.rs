//! Preset Manager – Programmstart.
//!
//! Die Oberfläche entsteht in Meilenstein 4 (`app.rs`, `worker.rs`, `texts.rs`,
//! `ui/`). Bis dahin ist das Programm ein Konsolenwerkzeug mit zwei Befehlen
//! (siehe [`cli::USAGE`]):
//!
//! - `anzeigen`: installierte Presets nach Gruppen auflisten
//! - `importieren`: Plan zeigen und – nur mit `--ausfuehren` – installieren

// In Tests dürfen Abkürzungen benutzt werden, die im Programmcode verboten sind.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod cli;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use preset_core::adobe::{AdobeLocations, FolderKind};
use preset_core::import::{scan, ConflictPolicy, ImportPlan, PlannedAction, PlannedImport};
use preset_core::library::Library;
use preset_core::preset::{PresetType, Section};
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{default_backup_root, BackupSession};

use cli::Command;

fn main() -> ExitCode {
    let arguments = match cli::parse(std::env::args_os().skip(1)) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::USAGE);
            return ExitCode::FAILURE;
        }
    };
    if arguments.command == Command::Help {
        println!("{}", cli::USAGE);
        return ExitCode::SUCCESS;
    }
    if arguments.command == Command::Locations {
        show_locations();
        return ExitCode::SUCCESS;
    }

    let locations = match AdobeLocations::resolve(arguments.camera_raw_dir.as_deref()) {
        Ok(locations) => locations,
        Err(error) => {
            eprintln!("Fehler: {}", describe(&error));
            return ExitCode::FAILURE;
        }
    };

    match arguments.command {
        Command::Import {
            sources,
            execute,
            policy,
            include_installed,
        } => import(&locations, &sources, execute, policy, include_installed),
        Command::Show | Command::Locations | Command::Help => {
            show(&locations);
            ExitCode::SUCCESS
        }
    }
}

/// Listet alle gefundenen Camera-Raw-Ordner auf.
fn show_locations() {
    let found = AdobeLocations::find_all();
    if found.is_empty() {
        println!("Kein Camera-Raw-Ordner gefunden.");
    }
    for folder in found {
        let what = match &folder.kind {
            FolderKind::User => {
                "Benutzerordner (Photoshop, Photoshop Beta, Lightroom Classic)".to_owned()
            }
            FolderKind::LightroomCatalog { catalog, in_use } => format!(
                "Lightroom-Katalog {} – „Presets mit diesem Katalog speichern“ ist {}",
                catalog.display(),
                if *in_use { "an" } else { "aus" }
            ),
        };
        println!(
            "{what}
  {}",
            folder.camera_raw_dir.display()
        );
    }
}

/// Listet die installierten Presets nach Gruppen auf.
fn show(locations: &AdobeLocations) {
    println!("Presets:             {}", locations.settings.display());
    println!(
        "Importierte Presets: {}",
        locations.imported_settings.display()
    );
    println!();

    let (library, report) = Library::load(locations);

    let sections = [
        (Section::Presets, "PRESETS (Camera Raw: Vorgaben)"),
        (
            Section::Profiles,
            "PROFILE (Camera Raw: Bearbeiten → Profil → Durchsuchen)",
        ),
    ];
    let mut counts = Vec::new();
    for (section, heading) in sections {
        println!("{heading}");
        let mut entries = 0;
        for group in library.groups_in(section) {
            let name = if group.name.is_empty() {
                "(ohne Gruppe)"
            } else {
                &group.name
            };
            println!("  {name:<58} {:>4}", group.presets.len());
            entries += group.presets.len();
        }
        println!();
        counts.push(entries);
    }

    println!(
        "{} Presets · {} Profile · {} Gruppen · {} übersprungen · {} Fehler",
        counts.first().copied().unwrap_or_default(),
        counts.get(1).copied().unwrap_or_default(),
        library.groups().len(),
        report.skipped_count(),
        report.failed_count()
    );
    print_problems(&report);
}

/// Durchsucht die Quellen, zeigt den Plan und führt ihn auf Wunsch aus.
fn import(
    locations: &AdobeLocations,
    sources: &[PathBuf],
    execute: bool,
    policy: ConflictPolicy,
    include_installed: bool,
) -> ExitCode {
    let mut scanned = Vec::new();
    let mut any_source_rejected = false;
    for source in sources {
        match scan(source) {
            Ok(source) => scanned.push(source),
            Err(error) => {
                eprintln!("Quelle abgelehnt: {}", describe(&error));
                any_source_rejected = true;
            }
        }
    }
    if scanned.is_empty() {
        return ExitCode::FAILURE;
    }

    let (library, _) = Library::load(locations);
    let mut plan = ImportPlan::new(scanned, locations, &library);
    if include_installed {
        for index in 0..plan.items().len() {
            plan.set_selected(index, true);
        }
    }
    print_plan(&plan, &locations.settings);

    let selected = plan.items().iter().filter(|item| item.selected).count();
    if selected == 0 {
        println!();
        println!("Es gibt nichts zu installieren – alle Einträge sind schon installiert.");
        print_where_to_find(plan.items().iter());
        // Eine abgelehnte Quelle ist ein Fehler und hat Vorrang.
        return if any_source_rejected {
            ExitCode::FAILURE
        } else {
            nothing_to_do()
        };
    }

    if !execute {
        println!();
        println!("Nur Vorschau – nichts wurde kopiert.");
        return exit_code(!any_source_rejected);
    }

    let Some(backup_root) = default_backup_root() else {
        eprintln!("Fehler: %LOCALAPPDATA% ist nicht gesetzt, Sicherungen wären nicht möglich.");
        return ExitCode::FAILURE;
    };
    let mut backups = BackupSession::new(backup_root);
    let report = plan.execute(policy, &mut backups);

    println!();
    println!(
        "Ergebnis: {} installiert · {} übersprungen · {} Fehler",
        report.done_count(),
        report.skipped_count(),
        report.failed_count()
    );
    print_problems(&report);
    if let Some(folder) = backups.folder() {
        println!("Ersetzte Dateien gesichert in: {}", folder.display());
    }
    if report.done_count() > 0 {
        println!("Photoshop bzw. Lightroom neu starten, damit die Presets erscheinen.");
        print_where_to_find(plan.items().iter().filter(|item| item.selected));
    }

    // Eine abgelehnte Quelle zählt als Fehler, auch wenn der Rest klappte.
    exit_code(report.is_clean() && !any_source_rejected)
}

/// Rückgabewert, wenn es nichts zu installieren gibt. `Start.bat` fragt dann
/// nicht nach, ob installiert werden soll.
fn nothing_to_do() -> ExitCode {
    ExitCode::from(2)
}

/// Sagt, wo die Einträge in Camera Raw zu finden sind. Profile stehen nicht
/// in der Vorgabenliste, sondern im Profilbrowser – das ist leicht zu
/// übersehen.
fn print_where_to_find<'a>(items: impl Iterator<Item = &'a PlannedImport>) {
    let mut preset_groups: Vec<&str> = Vec::new();
    let mut profile_groups: Vec<&str> = Vec::new();
    for item in items {
        let metadata = &item.preset.metadata;
        let groups = if metadata.preset_type == PresetType::Profile {
            &mut profile_groups
        } else {
            &mut preset_groups
        };
        if !groups.contains(&metadata.group.as_str()) {
            groups.push(&metadata.group);
        }
    }
    println!();
    if !preset_groups.is_empty() {
        println!(
            "Presets: in Camera Raw unter „Vorgaben“, Gruppe(n): {}",
            preset_groups.join(", ")
        );
    }
    if !profile_groups.is_empty() {
        println!(
            "Profile: in Camera Raw unter „Bearbeiten → Profil → Durchsuchen“ (nicht bei den \
             Vorgaben), Gruppe(n): {}",
            profile_groups.join(", ")
        );
    }
}

fn exit_code(success: bool) -> ExitCode {
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_plan(plan: &ImportPlan, settings: &Path) {
    for source in plan.sources() {
        println!(
            "Quelle: {}  →  Settings\\{}",
            source.path.display(),
            source.package_name
        );
        print_problems(&source.report);
    }
    println!();

    for item in plan.items() {
        let mark = if item.selected { "[x]" } else { "[ ]" };
        let kind = if item.preset.metadata.preset_type == PresetType::Profile {
            "Profil"
        } else {
            "Preset"
        };
        println!(
            "  {mark} {kind:<7}{:<12} {}",
            label(item),
            relative(&item.target, settings)
        );
        if let PlannedAction::DuplicateInImport { first } = &item.action {
            println!(
                "               doppelt, zuerst gefunden in {}",
                first.display()
            );
        }
        for other in &item.installed_with_same_uuid {
            println!(
                "               gleiche UUID schon installiert: {}",
                other.display()
            );
        }
    }

    let summary = plan.summary();
    println!();
    println!(
        "Plan: {} neu · {} vorhanden (Konflikt) · {} doppelt im Import · {} schon anderswo installiert",
        summary.new_files,
        summary.conflicts,
        summary.duplicates_in_import,
        summary.already_installed_elsewhere
    );
    if summary.deselected > 0 {
        println!(
            "{} Einträge sind abgewählt ([ ]) und werden nicht installiert, weil dasselbe \
             Preset schon installiert ist (Befehlszeile: --auch-installierte schaltet das ab).",
            summary.deselected
        );
    }
}

fn label(item: &PlannedImport) -> &'static str {
    match item.action {
        PlannedAction::Install => "neu",
        PlannedAction::Conflict => "vorhanden",
        PlannedAction::DuplicateInImport { .. } => "doppelt",
    }
}

fn relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Gibt übersprungene und gescheiterte Einträge eines Berichts aus.
fn print_problems(report: &OperationReport) {
    for (path, outcome) in report.items() {
        match outcome {
            ItemOutcome::Skipped { reason } => {
                println!("  übersprungen: {} ({reason})", path.display())
            }
            ItemOutcome::Failed(error) => println!("  Fehler: {}", describe(error)),
            ItemOutcome::Done => {}
        }
    }
}

/// Fehlermeldung samt Ursache, z. B. „Dateizugriff gescheitert: … (Zugriff
/// verweigert)“.
fn describe(error: &dyn Error) -> String {
    match error.source() {
        Some(cause) => format!("{error} ({cause})"),
        None => error.to_string(),
    }
}
