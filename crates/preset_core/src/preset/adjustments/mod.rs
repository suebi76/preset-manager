//! Was ein Preset einstellt – geordnet wie in Camera Raw und in Worten.
//!
//! [`super::settings`] liefert die Rohwerte. Hier werden sie gedeutet:
//!
//! - [`PresetSettings::sliders`]: die Regler eines Bereichs, die vom Standard
//!   abweichen
//! - [`PresetSettings::point_curves`], [`PresetSettings::mixer`],
//!   [`PresetSettings::color_grading`], [`PresetSettings::white_balance`]:
//!   was sich nicht als einzelner Regler zeigen lässt
//! - [`PresetSettings::facts`]: Aussagen wie „heller“ oder „Grün entsättigt“,
//!   aus denen die Oberfläche einen Satz baut
//!
//! Alles bleibt sprachneutral; die Texte stehen in der Oberfläche.
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `sliders` | Bereiche, Regler, [`SLIDERS`] |
//! | `curve` | Gradationskurven |
//! | `color` | Farbmischer, Color Grading, Weißabgleich |
//! | `facts` | Aussagen über die Wirkung |

mod color;
mod curve;
mod facts;
mod sliders;
#[cfg(test)]
mod tests;

pub use color::{ColorGrading, HslColor, MixerRow, Wheel, WhiteBalance};
pub use curve::{curve_value, CurveChannel};
pub use facts::{ColorChange, Fact, ToneZone};
pub use sliders::{Panel, Scale, Slider, SliderSpec, SLIDERS};
