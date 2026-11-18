"""Legt einen Camera-Raw-Ordner mit selbst ausgedachten Demo-Presets an.

Gedacht zum Ausprobieren und für Bildschirmfotos – ohne die eigenen (oder
gekauften) Presets anzufassen:

    python scripts/demo_bibliothek.py C:\\Temp\\DemoCameraRaw
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


def write_group(settings: Path, group: str, names: list, kind: str) -> None:
    folder = settings / group.replace("&", "und")
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
    if len(sys.argv) != 2:
        sys.exit("Aufruf: python scripts/demo_bibliothek.py <Zielordner>")
    target = Path(sys.argv[1])
    settings = target / "Settings"
    if settings.exists() and any(settings.iterdir()):
        sys.exit(f"{settings} ist nicht leer – bitte einen leeren Ordner angeben.")
    for group, names in PRESETS.items():
        write_group(settings, group, names, "Normal")
    for group, names in PROFILES.items():
        write_group(settings, group, names, "Look")
    print(f"Demo-Bibliothek angelegt: {target}")


if __name__ == "__main__":
    main()
