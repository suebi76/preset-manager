//! Kleine Bausteine, die in mehreren Bereichen vorkommen.

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, Label, Layout, Margin, Response, RichText,
    Sense, Stroke, Ui, UiBuilder,
};
use egui_phosphor::regular as icon;
use preset_core::preset::{PresetType, Section};

use crate::ui::theme;

/// Symbol und Farbe einer Art (F-31, `Planung/07-Oberflaeche.md`).
pub fn type_icon(preset_type: &PresetType) -> (&'static str, Color32) {
    match preset_type {
        PresetType::Preset => (icon::SLIDERS_HORIZONTAL, theme::PRESET),
        PresetType::Profile => (icon::APERTURE, theme::PROFILE),
        PresetType::Other(_) => (icon::QUESTION, theme::MUTED),
    }
}

/// Symbol und Farbe eines Bereichs.
pub fn section_icon(section: Section) -> (&'static str, Color32) {
    match section {
        Section::Presets => (icon::SLIDERS_HORIZONTAL, theme::PRESET),
        Section::Profiles => (icon::APERTURE, theme::PROFILE),
    }
}

/// Bezeichnung einer Art für Tooltips und Details.
pub fn type_label(preset_type: &PresetType) -> &str {
    match preset_type {
        PresetType::Preset => crate::texts::preset(),
        PresetType::Profile => crate::texts::profile(),
        PresetType::Other(value) => value,
    }
}

/// Ein Symbol in Listengröße.
pub fn glyph(ui: &mut Ui, glyph: &str, color: Color32) -> Response {
    ui.add(Label::new(RichText::new(glyph).size(theme::ICON_SIZE).color(color)).selectable(false))
}

/// Text, der bei Platzmangel mit „…“ gekürzt wird (der volle Text erscheint
/// beim Überfahren).
pub fn truncated(ui: &mut Ui, text: impl Into<RichText>) -> Response {
    ui.add(Label::new(text.into()).truncate().selectable(false))
}

/// Eine Listenzeile in voller Breite. `left` zeichnet von links, `right` von
/// rechts nach links (Anzahl, Symbole). `sense` legt fest, ob die Zeile nur
/// klickbar oder auch ziehbar ist.
pub fn list_row(
    ui: &mut Ui,
    selected: bool,
    sense: Sense,
    right: impl FnOnce(&mut Ui),
    left: impl FnOnce(&mut Ui),
) -> Response {
    let size = egui::vec2(ui.available_width(), theme::ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            theme::SELECTION
        } else if response.hovered() {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
        let inner = rect.shrink2(egui::vec2(8.0, 0.0));
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
        );
        right(&mut child);
        child.with_layout(Layout::left_to_right(Align::Center), left);
    }
    response
}

/// Rahmen um ein gültiges Ablageziel.
pub fn paint_drop_ok(ui: &Ui, rect: egui::Rect) {
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        theme::SELECTION,
        Stroke::new(1.5, theme::ACCENT),
        egui::StrokeKind::Inside,
    );
}

/// Abdunkeln eines ungültigen Ablageziels.
pub fn paint_drop_bad(ui: &Ui, rect: egui::Rect) {
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(4),
        Color32::from_rgba_unmultiplied(0, 0, 0, 110),
    );
}

/// Blaue Linie zwischen zwei Zeilen: hier wird eingefügt.
pub fn paint_insert_line(ui: &Ui, rect: egui::Rect, below: bool) {
    let y = if below {
        rect.bottom() + 2.0
    } else {
        rect.top() - 2.0
    };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 4.0, y),
            egui::pos2(rect.right() - 4.0, y),
        ],
        Stroke::new(2.0, theme::ACCENT),
    );
}

/// Was beim Umbenennen in der Zeile passiert ist.
pub enum RenameOutcome {
    Editing,
    Commit,
    Cancel,
}

/// Eingabefeld zum Umbenennen in einer Zeile. Enter übernimmt, Esc bricht
/// ab, ein Klick daneben übernimmt ebenfalls.
pub fn rename_field(ui: &mut Ui, text: &mut String, focus: &mut bool) -> RenameOutcome {
    let output = egui::TextEdit::singleline(text)
        .desired_width(ui.available_width())
        .margin(Margin::symmetric(4, 1))
        .show(ui);
    let response = output.response;
    if *focus {
        // Wie im Explorer: der alte Name ist markiert, Tippen ersetzt ihn.
        response.request_focus();
        let mut state = output.state;
        let end = text.chars().count();
        state
            .cursor
            .set_char_range(Some(egui::text_selection::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(end),
            )));
        state.store(ui.ctx(), response.id);
        *focus = false;
        return RenameOutcome::Editing;
    }
    if response.lost_focus() {
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            return RenameOutcome::Cancel;
        }
        return RenameOutcome::Commit;
    }
    RenameOutcome::Editing
}

/// Überschrift eines Bereichs („PRESETS“).
pub fn section_heading(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).small().strong().color(theme::MUTED));
}

/// Kleines Etikett mit Hintergrund („neu“, „Datei vorhanden“).
pub fn chip(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) -> Response {
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(8, 1))
        .show(ui, |ui| ui.label(RichText::new(text).small().color(fg)))
        .response
}

/// Die eine hervorgehobene Schaltfläche („Presets importieren“, „Installieren“).
pub fn primary_button(ui: &mut Ui, text: impl Into<String>, enabled: bool) -> Response {
    let text = RichText::new(text.into()).color(Color32::WHITE).strong();
    ui.add_enabled(
        enabled,
        Button::new(text)
            .fill(theme::ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(4)),
    )
}

/// Eine zurückhaltende Schaltfläche mit Symbol.
pub fn icon_button(ui: &mut Ui, glyph: &str, text: &str) -> Response {
    ui.add(Button::new(format!("{glyph}  {text}")))
}

/// Leerer Zustand mit großem Symbol, Titel und Hinweis.
pub fn empty_state(ui: &mut Ui, glyph: &str, title: &str, hint: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(glyph).size(36.0).color(theme::FAINT));
        ui.add_space(6.0);
        ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
        ui.label(RichText::new(hint).color(theme::MUTED));
    });
}

/// Ein Schiebeschalter (an/aus). Gibt die Antwort des Klicks zurück; den
/// Zustand ändert der Aufrufer.
pub fn toggle_switch(ui: &mut Ui, on: bool) -> Response {
    let size = egui::vec2(34.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let how_on = ui.ctx().animate_bool_responsive(response.id, on);
    let radius = rect.height() / 2.0;
    let track = if on {
        theme::OK
    } else {
        theme::LINE.gamma_multiply(1.6)
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius as u8), track);
    let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), radius - 3.0, Color32::WHITE);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Kopf eines Detailbereichs: Symbol, Titel, Unterzeile – ohne Kasten.
pub fn title_block(ui: &mut Ui, glyph: &str, color: Color32, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(glyph).size(24.0).color(color));
        ui.vertical(|ui| {
            truncated(ui, RichText::new(title).size(15.0).strong());
            truncated(ui, RichText::new(subtitle).small().color(theme::MUTED));
        });
    });
}

/// Eine dünne Trennlinie mit etwas Luft.
pub fn divider(ui: &mut Ui) {
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::LINE);
    ui.add_space(8.0);
}

/// Umschalter zwischen Seiten, über die ganze Breite (wie Segmente).
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, options: &[(T, &str)]) {
    let width = ui.available_width();
    let count = options.len().max(1) as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), theme::BG);
    for (index, (value, label)) in options.iter().enumerate() {
        let segment = egui::Rect::from_min_size(
            egui::pos2(rect.left() + width / count * index as f32, rect.top()),
            egui::vec2(width / count, rect.height()),
        )
        .shrink(2.0);
        let response = ui.interact(segment, ui.id().with(("segment", index)), Sense::click());
        let selected = *current == *value;
        if selected {
            ui.painter()
                .rect_filled(segment, CornerRadius::same(5), theme::RAISED);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(segment, CornerRadius::same(5), theme::HOVER);
        }
        ui.painter().text(
            segment.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(13.0),
            if selected { theme::TEXT } else { theme::MUTED },
        );
        if response.clicked() {
            *current = *value;
        }
    }
}

/// Ein Rollbereich mit gut sichtbarem Griff. Die Bildlaufleiste nimmt die
/// Farben der Schaltflächen; damit sie auf dunklem Grund nicht verschwindet,
/// bekommt nur sie hellere Töne – der Inhalt sieht aus wie immer.
pub fn scroll<R>(ui: &mut Ui, area: egui::ScrollArea, add: impl FnOnce(&mut Ui) -> R) -> R {
    let content_widgets = ui.visuals().widgets.clone();
    ui.scope(|ui| {
        set_scroll_handle_colors(ui);
        area.show(ui, |ui| {
            ui.visuals_mut().widgets = content_widgets;
            add(ui)
        })
        .inner
    })
    .inner
}

/// Wie [`scroll`], aber nur die sichtbaren Zeilen werden gezeichnet.
pub fn scroll_rows<R>(
    ui: &mut Ui,
    area: egui::ScrollArea,
    row_height: f32,
    rows: usize,
    add: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R,
) -> R {
    let content_widgets = ui.visuals().widgets.clone();
    ui.scope(|ui| {
        set_scroll_handle_colors(ui);
        area.show_rows(ui, row_height, rows, |ui, range| {
            ui.visuals_mut().widgets = content_widgets;
            add(ui, range)
        })
        .inner
    })
    .inner
}

fn set_scroll_handle_colors(ui: &mut Ui) {
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.bg_fill = Color32::from_rgb(0x5a, 0x5a, 0x5a);
    widgets.hovered.bg_fill = Color32::from_rgb(0x78, 0x78, 0x78);
    widgets.active.bg_fill = theme::ACCENT;
}
