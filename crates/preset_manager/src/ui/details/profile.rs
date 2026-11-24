//! Profil-Hinweise in den Details (F-50): Welches Profil braucht das
//! Preset, ist es da – und welche Presets nutzen ein Profil.

use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::library::ProfileStatus;
use preset_core::preset::Preset;

use crate::app::{profile_ref, App, Place};
use crate::texts::{self, tr, tr_format};
use crate::ui::theme;

use super::Action;

/// Unter dem Titel: Hinweis, wenn das Profil fehlt, sonst eine Zeile mit
/// dem Profil; bei Profilen, wie viele Presets sie nutzen.
pub(super) fn profile_lines(ui: &mut Ui, app: &App, preset: &Preset, actions: &mut Vec<Action>) {
    if let Some((profile, status)) = app.needed_profile(preset).cloned() {
        let users = app.users_of(&profile);
        let name = if profile.name.is_empty() {
            tr("ohne Namen", "unnamed").to_owned()
        } else {
            profile.name.clone()
        };
        match status {
            ProfileStatus::Installed => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}  {}",
                            icon::APERTURE,
                            tr_format!(
                                "Baut auf dem Profil „{name}“ auf",
                                "Built on the profile “{name}”"
                            )
                        ))
                        .color(theme::MUTED),
                    );
                });
            }
            ProfileStatus::Missing | ProfileStatus::Inactive => {
                let text = if status == ProfileStatus::Missing {
                    tr_format!(
                        "Das Profil „{name}“ fehlt. Photoshop und Lightroom zeigen das Preset ohne seinen Look. Meist liegt das Profil im selben Paket wie das Preset – importiere es mit.",
                        "The profile “{name}” is missing. Photoshop and Lightroom show the preset without its look. The profile usually comes in the same pack as the preset – import it too."
                    )
                } else {
                    tr_format!(
                        "Das Profil „{name}“ ist deaktiviert. Aktiviere es, damit das Preset in Photoshop und Lightroom richtig wirkt.",
                        "The profile “{name}” is deactivated. Activate it so the preset looks right in Photoshop and Lightroom."
                    )
                };
                Frame::new()
                    .fill(theme::WARN_BG)
                    .corner_radius(6.0)
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(icon::WARNING).color(theme::WARN));
                            ui.label(RichText::new(text).color(theme::WARN_TEXT));
                        });
                    });
            }
        }
        if users > 1 {
            show_users_button(ui, &profile, users, actions);
        }
        ui.add_space(8.0);
    } else if let Some(profile) = profile_ref(preset) {
        let users = app.users_of(&profile);
        if users > 0 {
            show_users_button(ui, &profile, users, actions);
            ui.add_space(8.0);
        }
    }
}

fn show_users_button(
    ui: &mut Ui,
    profile: &preset_core::preset::ProfileRef,
    users: usize,
    actions: &mut Vec<Action>,
) {
    let count = texts::entries(users);
    let label = if users == 1 {
        tr_format!(
            "{}  {count} nutzt dieses Profil – zeigen",
            "{}  {count} uses this profile – show",
            icon::LIST_BULLETS
        )
    } else {
        tr_format!(
            "{}  {count} nutzen dieses Profil – zeigen",
            "{}  {count} use this profile – show",
            icon::LIST_BULLETS
        )
    };
    if ui
        .add(egui::Button::new(label).frame(false))
        .on_hover_text(tr(
            "Alle Presets und Profile, die auf diesem Profil aufbauen",
            "All presets and profiles built on this profile",
        ))
        .clicked()
    {
        actions.push(Action::ShowPlace(Place::UsesProfile(profile.clone())));
    }
}
