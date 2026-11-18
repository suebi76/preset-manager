//! Importvorschau (F-07): was wird installiert, was nicht, und warum.

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Ui};
use egui_phosphor::regular as icon;
use preset_core::import::{ConflictPolicy, ImportPlan, PlannedAction, PlannedImport};

use crate::app::{file_name, App};
use crate::texts::{self, tr_format};
use crate::ui::{theme, widgets};

pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(dialog) = &mut app.import else {
        return;
    };
    let mut cancel = false;
    let mut install = false;

    let response = egui::Modal::new(egui::Id::new("import-dialog"))
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0, theme::LINE))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.set_width(640.0);
            ui.label(RichText::new(texts::import_title()).size(16.0).strong());
            ui.add_space(6.0);
            match &mut dialog.plan {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(texts::installing());
                    });
                }
                Some(plan) => {
                    sources(ui, plan, &dialog.rejected);
                    entries(ui, plan);
                    policy(ui, &mut dialog.policy, plan);
                    ui.separator();
                    let count = install_count(plan, dialog.policy);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(texts::will_install(count, plan.items().len()))
                                .color(theme::MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::primary_button(ui, texts::install_button(count), count > 0)
                                .clicked()
                            {
                                install = true;
                            }
                            if ui.button(texts::cancel()).clicked() {
                                cancel = true;
                            }
                        });
                    });
                }
            }
        });

    let plan_open = dialog.plan.is_some();
    if plan_open && (cancel || response.should_close()) {
        app.import = None;
    } else if install {
        app.execute_import();
    }
}

/// Woher kommt das Paket, wohin geht es, was wurde in der Quelle übergangen.
fn sources(ui: &mut Ui, plan: &ImportPlan, rejected: &[(std::path::PathBuf, String)]) {
    for source in plan.sources() {
        ui.horizontal_wrapped(|ui| {
            let source_icon = if source.path.is_dir() {
                icon::FOLDER_SIMPLE
            } else {
                icon::FILE_ZIP
            };
            ui.label(RichText::new(source_icon).color(theme::MUTED));
            ui.label(RichText::new(file_name(&source.path)).strong());
            ui.label(
                RichText::new(format!("→ Settings\\{}", source.package_name)).color(theme::MUTED),
            );
            let skipped = source.report.skipped_count();
            let failed = source.report.failed_count();
            if skipped + failed > 0 {
                let details: Vec<String> = source
                    .report
                    .items()
                    .iter()
                    .map(|(path, _)| file_name(path))
                    .collect();
                ui.label(
                    RichText::new(tr_format!(
                        "{} übergangen",
                        "{} skipped",
                        texts::files(skipped + failed)
                    ))
                    .small()
                    .color(theme::WARN_TEXT),
                )
                .on_hover_text(tr_format!(
                    "Keine Presets oder nicht lesbar:\n{}",
                    "Not presets or not readable:\n{}",
                    details.join("\n")
                ));
            }
        });
    }
    for (path, reason) in rejected {
        ui.label(
            RichText::new(format!("{} {}: {reason}", icon::WARNING, file_name(path)))
                .color(theme::ERROR),
        );
    }
    ui.add_space(6.0);
}

fn entries(ui: &mut Ui, plan: &mut ImportPlan) {
    let mut toggles = Vec::new();
    ScrollArea::vertical()
        .max_height(360.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (index, item) in plan.items().iter().enumerate() {
                let mut checked = item.selected;
                let group = plan.group_after_import(item);
                let response = widgets::list_row(
                    ui,
                    false,
                    egui::Sense::click(),
                    |ui| {
                        status(ui, item);
                        let group = if group.is_empty() {
                            texts::no_group()
                        } else {
                            group
                        };
                        widgets::truncated(ui, RichText::new(group).color(theme::MUTED));
                    },
                    |ui| {
                        if ui.checkbox(&mut checked, "").changed() {
                            toggles.push((index, checked));
                        }
                        let (glyph, color) = widgets::type_icon(&item.preset.metadata.preset_type);
                        widgets::glyph(ui, glyph, color);
                        widgets::truncated(ui, item.preset.metadata.name.as_str());
                    },
                );
                if response.clicked() {
                    toggles.push((index, !item.selected));
                }
            }
        });
    for (index, selected) in toggles {
        plan.set_selected(index, selected);
    }
}

fn status(ui: &mut Ui, item: &PlannedImport) {
    let (text, fg, bg, tip) = if !item.installed_with_same_uuid.is_empty() {
        (
            texts::status_installed(),
            theme::MUTED,
            theme::RAISED,
            texts::status_installed_tip(),
        )
    } else {
        match item.action {
            PlannedAction::Install => (texts::status_new(), theme::OK_TEXT, darker(theme::OK), ""),
            PlannedAction::Conflict => (
                texts::status_conflict(),
                theme::WARN_TEXT,
                theme::WARN_BG,
                texts::status_conflict_tip(),
            ),
            PlannedAction::DuplicateInImport { .. } => (
                texts::status_duplicate(),
                theme::MUTED,
                theme::RAISED,
                texts::status_duplicate_tip(),
            ),
        }
    };
    let chip = widgets::chip(ui, text, fg, bg);
    if !tip.is_empty() {
        chip.on_hover_text(tip);
    }
}

fn darker(color: egui::Color32) -> egui::Color32 {
    egui::Color32::from_rgb(color.r() / 4, color.g() / 4, color.b() / 4)
}

fn policy(ui: &mut Ui, policy: &mut ConflictPolicy, plan: &ImportPlan) {
    let has_conflicts = plan.summary().conflicts > 0;
    ui.add_space(8.0);
    ui.add_enabled_ui(has_conflicts, |ui| {
        ui.label(RichText::new(texts::import_policy()).color(theme::MUTED));
        ui.horizontal(|ui| {
            ui.radio_value(policy, ConflictPolicy::Skip, texts::policy_skip());
            ui.radio_value(policy, ConflictPolicy::Replace, texts::policy_replace());
            ui.radio_value(policy, ConflictPolicy::KeepBoth, texts::policy_keep_both());
        });
    });
    ui.add_space(4.0);
}

/// Wie viele Einträge tatsächlich installiert würden.
fn install_count(plan: &ImportPlan, policy: ConflictPolicy) -> usize {
    plan.items()
        .iter()
        .filter(|item| item.selected)
        .filter(|item| match item.action {
            PlannedAction::Install => true,
            PlannedAction::Conflict => policy != ConflictPolicy::Skip,
            PlannedAction::DuplicateInImport { .. } => false,
        })
        .count()
}
