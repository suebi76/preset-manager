"""Legt einen Camera-Raw-Ordner mit selbst ausgedachten Demo-Presets an.

Gedacht zum Ausprobieren und für Bildschirmfotos – ohne die eigenen (oder
gekauften) Presets anzufassen:

    python scripts/demo_bibliothek.py C:\\Temp\\DemoCameraRaw
    python scripts/demo_bibliothek.py C:\\Temp\\DemoCameraRaw --english
    set APM_CAMERA_RAW_DIR=C:\\Temp\\DemoCameraRaw
    Start.bat

Die Dateien enthalten nur wenige, harmlose Einstellungen.
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

TEMPLATE = """<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Preset Manager Demo">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:PresetType="{kind}"
   crs:UUID="{uuid}"
   crs:SupportsAmount="{amount}"
   crs:Version="18.1"
   crs:ProcessVersion="15.4"
   crs:Exposure2012="{exposure}">
   <crs:Name>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">{name}</rdf:li>
    </rdf:Alt>
   </crs:Name>
   <crs:Group>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">{group}</rdf:li>
    </rdf:Alt>
   </crs:Group>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"""


def write_group(settings: Path, group: str, names: list, kind: str, and_word: str) -> None:
    folder = settings / group.replace("&", and_word)
    folder.mkdir(parents=True, exist_ok=True)
    for index, name in enumerate(names):
        text = TEMPLATE.format(
            kind=kind,
            uuid=uuid.uuid4().hex.upper(),
            amount="True" if kind == "Look" else "False",
            exposure=f"+{index * 0.1:.2f}",
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
    for group, names in presets.items():
        write_group(settings, group, names, "Normal", and_word)
    for group, names in profiles.items():
        write_group(settings, group, names, "Look", and_word)
    print(f"Demo-Bibliothek angelegt: {target}")


if __name__ == "__main__":
    main()
