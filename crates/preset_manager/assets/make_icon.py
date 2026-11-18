"""Zeichnet das App-Symbol neu: assets/icon.png und assets/icon.ico.

Aufruf (braucht Pillow):  python crates/preset_manager/assets/make_icon.py

Motiv: drei Schieberegler – wie die Regler in Camera Raw – auf einer
abgerundeten Fläche im Verlauf von Blau (Presets) nach Violett (Profile),
den beiden Farben, mit denen die App Presets und Profile kennzeichnet.
"""

from pathlib import Path

from PIL import Image, ImageDraw

HERE = Path(__file__).resolve().parent
SIZE = 1024  # gezeichnet wird groß, verkleinert wird mit Glättung
BLUE = (0x26, 0x80, 0xEB)
VIOLET = (0x92, 0x56, 0xD9)
WHITE = (255, 255, 255, 255)
TRACK = (255, 255, 255, 120)


def gradient(size: int) -> Image.Image:
    """Diagonaler Verlauf von oben links (Blau) nach unten rechts (Violett)."""
    image = Image.new("RGBA", (size, size))
    pixels = image.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1))
            pixels[x, y] = tuple(round(a + (b - a) * t) for a, b in zip(BLUE, VIOLET)) + (255,)
    return image


def draw() -> Image.Image:
    margin = SIZE * 0.06
    radius = SIZE * 0.22
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (margin, margin, SIZE - margin, SIZE - margin), radius=radius, fill=255
    )
    icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    icon.paste(gradient(SIZE), (0, 0), mask)

    # Eigene Ebene, damit halbtransparente Linien über dem Verlauf liegen,
    # statt Löcher hineinzustanzen.
    layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    pen = ImageDraw.Draw(layer)
    left, right = SIZE * 0.25, SIZE * 0.75
    line = SIZE * 0.055
    knob = SIZE * 0.085
    # Höhe der Regler und Stellung der Knöpfe (0 = links, 1 = rechts)
    for y, position in ((0.33, 0.30), (0.50, 0.68), (0.67, 0.45)):
        cy = SIZE * y
        pen.rounded_rectangle(
            (left, cy - line / 2, right, cy + line / 2), radius=line / 2, fill=TRACK
        )
        cx = left + (right - left) * position
        pen.rounded_rectangle(
            (left, cy - line / 2, cx, cy + line / 2), radius=line / 2, fill=WHITE
        )
        pen.ellipse((cx - knob, cy - knob, cx + knob, cy + knob), fill=WHITE)
    return Image.alpha_composite(icon, layer)


def main() -> None:
    icon = draw()
    icon.resize((256, 256), Image.LANCZOS).save(HERE / "icon.png")
    icon.save(
        HERE / "icon.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print("icon.png und icon.ico geschrieben")


if __name__ == "__main__":
    main()
