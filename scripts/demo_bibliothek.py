"""Legt einen Camera-Raw-Ordner mit selbst ausgedachten Demo-Presets an.

Gedacht zum Ausprobieren und für Bildschirmfotos – ohne die eigenen (oder
gekauften) Presets anzufassen:

    python scripts/demo_bibliothek.py C:\\Temp\\DemoCameraRaw
    python scripts/demo_bibliothek.py C:\\Temp\\DemoCameraRaw --english
    set APM_CAMERA_RAW_DIR=C:\\Temp\\DemoCameraRaw
    Start.bat

Jedes Demo-Preset hat einen ausgedachten, aber glaubwürdigen Look (Regler,
Kurve, Farbmischer, Color Grading, teils eine Maske), damit die Ansicht
„Wirkung“ etwas zu zeigen hat.
"""

import sys
import uuid
from pathlib import Path
from xml.sax.saxutils import escape

PRESETS = {
    "Sommerlicht": ["Goldene Stunde", "Strandtag", "Warmer Abend", "Pastell", "Gegenlicht"],
    "Stadt & Nacht": ["Neon", "Blaue Stunde", "Beton", "Regenstraße"],
    "Schwarzweiß Klassik": ["Weich", "Kontrastreich", "Körnig", "Selen"],
    "Portrait": ["Hauttöne natürlich", "Hell & luftig", "Matt"],
}
PROFILES = {
    "Film-Looks": ["Diafilm Warm", "Negativ Kühl", "Sofortbild"],
    "Kreativ": ["Cross-Entwicklung", "Verblasst"],
}

# Dieselbe Bibliothek mit englischen Namen (für Bildschirmfotos der englischen Oberfläche).
PRESETS_EN = {
    "Summer Light": ["Golden Hour", "Beach Day", "Warm Evening", "Pastel", "Backlight"],
    "City & Night": ["Neon", "Blue Hour", "Concrete", "Rainy Street"],
    "Black & White Classic": ["Soft", "High Contrast", "Grainy", "Selenium"],
    "Portrait": ["Natural Skin", "Light & Airy", "Matte"],
}
PROFILES_EN = {
    "Film Looks": ["Slide Film Warm", "Negative Cool", "Instant Film"],
    "Creative": ["Cross Process", "Faded"],
}

# Die Looks, in derselben Reihenfolge wie die Namen oben (deutsch und englisch).
# "curve": Punkte der Gradationskurve, "masks": (Name, Art, lokale Werte).
GW = {"ConvertToGrayscale": "True"}
LOOKS = [
    [  # Sommerlicht
        {"Exposure2012": "+0.20", "IncrementalTemperature": "+18", "IncrementalTint": "+4",
         "Highlights2012": "-30", "Shadows2012": "+25", "Vibrance": "+15",
         "SplitToningHighlightHue": "45", "SplitToningHighlightSaturation": "25",
         "SplitToningShadowHue": "210", "SplitToningShadowSaturation": "10",
         "curve": [(0, 10), (64, 62), (192, 200), (255, 250)],
         "masks": [("Himmel", "Mask/Gradient",
                    {"LocalExposure2012": "-0.35", "LocalTemperature": "0.15"})]},
        {"Exposure2012": "+0.35", "Contrast2012": "-10", "Whites2012": "+15", "Vibrance": "+20",
         "Saturation": "-5", "SaturationAdjustmentAqua": "+20", "LuminanceAdjustmentBlue": "+10",
         "Dehaze": "+8"},
        {"IncrementalTemperature": "+25", "Exposure2012": "-0.15", "Contrast2012": "+15",
         "SaturationAdjustmentOrange": "+20", "ColorGradeMidtoneHue": "30",
         "ColorGradeMidtoneSat": "15", "PostCropVignetteAmount": "-15",
         "PostCropVignetteMidpoint": "40"},
        {"Contrast2012": "-25", "Highlights2012": "-20", "Shadows2012": "+40", "Saturation": "-15",
         "SaturationAdjustmentGreen": "-40", "HueAdjustmentGreen": "+15",
         "ColorGradeGlobalHue": "330", "ColorGradeGlobalSat": "8",
         "curve": [(0, 30), (128, 135), (255, 235)]},
        {"Exposure2012": "+0.40", "Dehaze": "-15", "Clarity2012": "-10", "Highlights2012": "-40",
         "IncrementalTemperature": "+10", "GrainAmount": "10",
         "masks": [("Sonne", "Mask/CircularGradient",
                    {"LocalExposure2012": "0.25", "LocalDehaze": "-0.2"})]},
    ],
    [  # Stadt & Nacht
        {"Contrast2012": "+25", "Vibrance": "+30", "IncrementalTemperature": "-15",
         "IncrementalTint": "+12", "SaturationAdjustmentPurple": "+30",
         "SaturationAdjustmentMagenta": "+25", "HueAdjustmentBlue": "-15",
         "SplitToningShadowHue": "220", "SplitToningShadowSaturation": "25", "Clarity2012": "+15"},
        {"IncrementalTemperature": "-25", "Exposure2012": "+0.10", "Shadows2012": "+20",
         "SplitToningShadowHue": "215", "SplitToningShadowSaturation": "30",
         "SplitToningHighlightHue": "40", "SplitToningHighlightSaturation": "10"},
        {"Saturation": "-45", "Clarity2012": "+30", "Texture": "+20", "Contrast2012": "+20",
         "PostCropVignetteAmount": "-20", "curve": [(0, 0), (64, 52), (192, 205), (255, 255)]},
        {"Exposure2012": "-0.20", "Dehaze": "+12", "IncrementalTemperature": "-10",
         "SaturationAdjustmentOrange": "+25", "SaturationAdjustmentBlue": "-20",
         "GrainAmount": "15"},
    ],
    [  # Schwarzweiß Klassik
        {**GW, "Contrast2012": "-15", "Clarity2012": "-15", "GrayMixerRed": "+10",
         "GrayMixerOrange": "+15", "GrayMixerBlue": "-10", "curve": [(0, 12), (255, 245)]},
        {**GW, "Contrast2012": "+40", "Whites2012": "+20", "Blacks2012": "-25",
         "Clarity2012": "+25", "GrayMixerBlue": "-30", "GrayMixerRed": "+15"},
        {**GW, "GrainAmount": "45", "GrainSize": "30", "GrainFrequency": "60",
         "Contrast2012": "+20", "PostCropVignetteAmount": "-25"},
        {**GW, "SplitToningShadowHue": "300", "SplitToningShadowSaturation": "18",
         "SplitToningHighlightHue": "30", "SplitToningHighlightSaturation": "8",
         "Contrast2012": "+10"},
    ],
    [  # Portrait
        {"IncrementalTemperature": "+6", "SaturationAdjustmentOrange": "-8",
         "LuminanceAdjustmentOrange": "+12", "LuminanceAdjustmentRed": "+5",
         "Texture": "-15", "Clarity2012": "-10"},
        {"Exposure2012": "+0.50", "Contrast2012": "-15", "Highlights2012": "-20",
         "Shadows2012": "+30", "Whites2012": "+10", "Saturation": "-10",
         "LuminanceAdjustmentOrange": "+15", "curve": [(0, 8), (255, 255)]},
        {"Contrast2012": "-10", "Saturation": "-15", "SplitToningShadowHue": "200",
         "SplitToningShadowSaturation": "12",
         "curve": [(0, 35), (70, 70), (190, 195), (255, 240)]},
    ],
]
PROFILE_LOOKS = [
    [  # Film-Looks
        {"IncrementalTemperature": "+12", "Contrast2012": "+15", "Saturation": "+10",
         "curve": [(0, 6), (64, 58), (192, 204), (255, 252)]},
        {"IncrementalTemperature": "-12", "Contrast2012": "-10",
         "SaturationAdjustmentGreen": "-25", "curve": [(0, 20), (255, 245)]},
        {"Contrast2012": "-15", "Saturation": "-20", "ColorGradeGlobalHue": "40",
         "ColorGradeGlobalSat": "10", "curve": [(0, 28), (255, 230)]},
    ],
    [  # Kreativ
        {"SplitToningShadowHue": "180", "SplitToningShadowSaturation": "30",
         "SplitToningHighlightHue": "60", "SplitToningHighlightSaturation": "30",
         "Contrast2012": "+20"},
        {"Saturation": "-35", "Contrast2012": "-20", "curve": [(0, 40), (255, 225)]},
    ],
]

TEMPLATE = """<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Preset Manager Demo">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:PresetType="{kind}"
   crs:UUID="{uuid}"
   crs:SupportsAmount="{amount}"
   crs:Version="18.1"
   crs:ProcessVersion="15.4"{attributes}>
   <crs:Name>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">{name}</rdf:li>
    </rdf:Alt>
   </crs:Name>
   <crs:Group>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">{group}</rdf:li>
    </rdf:Alt>
   </crs:Group>{body}
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"""


def body_for(look: dict) -> tuple:
    """Attribute und Inhalt (Kurve, Masken) für einen Look."""
    attributes = "".join(
        f'\n   crs:{key}="{value}"' for key, value in look.items() if key not in ("curve", "masks")
    )
    body = ""
    if "curve" in look:
        points = "".join(f"<rdf:li>{x}, {y}</rdf:li>" for x, y in look["curve"])
        body += f"\n   <crs:ToneCurvePV2012><rdf:Seq>{points}</rdf:Seq></crs:ToneCurvePV2012>"
    if "masks" in look:
        items = ""
        for name, what, values in look["masks"]:
            local = "".join(f' crs:{key}="{value}"' for key, value in values.items())
            items += (
                f'<rdf:li><rdf:Description crs:What="Correction" crs:CorrectionActive="true"'
                f' crs:CorrectionName="{escape(name)}"{local}><crs:CorrectionMasks><rdf:Seq>'
                f'<rdf:li crs:What="{what}"/></rdf:Seq></crs:CorrectionMasks>'
                f"</rdf:Description></rdf:li>"
            )
        body += (
            "\n   <crs:MaskGroupBasedCorrections><rdf:Seq>"
            f"{items}</rdf:Seq></crs:MaskGroupBasedCorrections>"
        )
    return attributes, body


def write_group(settings: Path, group: str, names: list, kind: str, and_word: str,
                looks: list) -> None:
    folder = settings / group.replace("&", and_word)
    folder.mkdir(parents=True, exist_ok=True)
    for index, name in enumerate(names):
        attributes, body = body_for(looks[index] if index < len(looks) else {})
        text = TEMPLATE.format(
            kind=kind,
            uuid=uuid.uuid4().hex.upper(),
            amount="True" if kind == "Look" else "False",
            attributes=attributes,
            body=body,
            name=escape(name),
            group=escape(group),
        )
        (folder / f"{name}.xmp").write_text(text, encoding="utf-8")


def main() -> None:
    args = [arg for arg in sys.argv[1:] if arg != "--english"]
    english = "--english" in sys.argv[1:]
    if len(args) != 1:
        sys.exit("Aufruf: python scripts/demo_bibliothek.py <Zielordner> [--english]")
    target = Path(args[0])
    presets, profiles = (PRESETS_EN, PROFILES_EN) if english else (PRESETS, PROFILES)
    and_word = "and" if english else "und"
    settings = target / "Settings"
    if settings.exists() and any(settings.iterdir()):
        sys.exit(f"{settings} ist nicht leer – bitte einen leeren Ordner angeben.")
    for number, (group, names) in enumerate(presets.items()):
        write_group(settings, group, names, "Normal", and_word, LOOKS[number])
    for number, (group, names) in enumerate(profiles.items()):
        write_group(settings, group, names, "Look", and_word, PROFILE_LOOKS[number])
    print(f"Demo-Bibliothek angelegt: {target}")


if __name__ == "__main__":
    main()
