//! Tastenkürzel (`Planung/07-Oberflaeche.md`). Gedrückte Tasten werden
//! einmal je Bild abgefragt und dann den Aktionen zugeordnet.

use eframe::egui;

use crate::app::{App, Filter, Place};

impl App {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context, now: f64) {
        use egui::{Key, Modifiers};
        // Nur ein tatsächlich fokussiertes Eingabefeld sperrt die Kürzel. Ein
        // Umbenennen, dessen Feld gerade nicht sichtbar ist (weggescrollt),
        // darf die Tastatur nicht dauerhaft blockieren.
        let typing = ctx.text_edit_focused();
        let keys = ctx.input_mut(|input| {
            let mut take = |modifiers, key| input.consume_key(modifiers, key);
            Keys {
                find: take(Modifiers::COMMAND, Key::F),
                settings: take(Modifiers::COMMAND, Key::Comma),
                all_filter: take(Modifiers::COMMAND, Key::Num1),
                presets: take(Modifiers::COMMAND, Key::Num2),
                profiles: take(Modifiers::COMMAND, Key::Num3),
                zip: take(Modifiers::COMMAND | Modifiers::SHIFT, Key::I),
                select_all: !typing && take(Modifiers::COMMAND, Key::A),
                new_group: !typing && take(Modifiers::COMMAND, Key::G),
                toggle_active: !typing && take(Modifiers::COMMAND, Key::D),
                export: !typing && take(Modifiers::COMMAND, Key::E),
                // Strg+Umschalt+Z vor Strg+Z prüfen – sonst finge Strg+Z beide.
                redo: !typing
                    && (take(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                        || take(Modifiers::COMMAND, Key::Y)),
                undo: !typing && take(Modifiers::COMMAND, Key::Z),
                delete: !typing && take(Modifiers::NONE, Key::Delete),
                rename: !typing && take(Modifiers::NONE, Key::F2),
                nudge_up: !typing && take(Modifiers::ALT, Key::ArrowUp),
                nudge_down: !typing && take(Modifiers::ALT, Key::ArrowDown),
                up: !typing && take(Modifiers::NONE, Key::ArrowUp),
                down: !typing && take(Modifiers::NONE, Key::ArrowDown),
                extend_up: !typing && take(Modifiers::SHIFT, Key::ArrowUp),
                extend_down: !typing && take(Modifiers::SHIFT, Key::ArrowDown),
                escape: !typing && take(Modifiers::NONE, Key::Escape),
            }
        });

        if keys.find {
            self.focus_search = true;
        }
        if keys.settings {
            self.settings_open = true;
        }
        if keys.all_filter {
            self.set_filter(Filter::All);
        }
        if keys.presets {
            self.set_filter(Filter::Presets);
        }
        if keys.profiles {
            self.set_filter(Filter::Profiles);
        }
        if keys.zip {
            self.pick_zips(now);
        }
        if keys.select_all {
            self.select_all_listed();
        }
        if keys.new_group {
            self.group_from_selection(now);
        }
        if keys.toggle_active {
            self.toggle_selection_active(now);
        }
        if keys.export {
            self.export_selection_or_group(now);
        }
        if keys.undo {
            self.undo(now);
        }
        if keys.redo {
            self.redo(now);
        }
        if keys.delete {
            self.request_delete_selection();
        }
        if keys.rename {
            if let Some(preset) = self.single_selected().cloned() {
                self.start_rename_preset(&preset);
            } else if let (true, Some(Place::Group { section, name })) =
                (self.selection.is_empty(), self.place.clone())
            {
                self.start_rename_group(section, &name);
            }
        }
        if keys.nudge_up || keys.nudge_down {
            self.nudge(keys.nudge_up, now);
        }
        if keys.up || keys.down {
            self.step_selection(keys.down, false);
        }
        if keys.extend_up || keys.extend_down {
            self.step_selection(keys.extend_down, true);
        }
        if keys.escape {
            self.selection.clear();
        }
    }
}

struct Keys {
    find: bool,
    settings: bool,
    all_filter: bool,
    presets: bool,
    profiles: bool,
    zip: bool,
    select_all: bool,
    new_group: bool,
    toggle_active: bool,
    export: bool,
    undo: bool,
    redo: bool,
    delete: bool,
    rename: bool,
    nudge_up: bool,
    nudge_down: bool,
    up: bool,
    down: bool,
    extend_up: bool,
    extend_down: bool,
    escape: bool,
}
