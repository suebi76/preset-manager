"""Zeichnet das Vorschaubild für GitHub (docs/social-preview.png, 1280×640).

Es erscheint, wenn der Link zum Repository geteilt wird. Hochladen von Hand:
GitHub → Settings → General → Social preview → Edit → Upload an image.

Aufruf (braucht Pillow und die Windows-Schrift Segoe UI):
    python scripts/social_preview.py
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
WIDTH, HEIGHT = 1280, 640
BG = (0x1D, 0x1D, 0x1D)
TEXT = (0xE8, 0xE8, 0xE8)
MUTED = (0x9A, 0x9A, 0x9A)
ACCENT = (0x26, 0x80, 0xEB)
FONTS = Path("C:/Windows/Fonts")


def font(name: str, size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONTS / name), size)


def main() -> None:
    image = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(image)

    # Bildschirmfoto rechts, angeschnitten, mit weichem Schatten
    shot = Image.open(ROOT / "docs" / "screenshot.png").convert("RGB")
    shot = shot.resize((760, round(shot.height * 760 / shot.width)), Image.LANCZOS)
    left, top = 560, 120
    shadow = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (left - 6, top - 2, left + shot.width + 6, top + shot.height + 14), radius=14, fill=(0, 0, 0, 170)
    )
    image.paste(shadow.filter(ImageFilter.GaussianBlur(18)), (0, 0), shadow.filter(ImageFilter.GaussianBlur(18)))
    mask = Image.new("L", shot.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, shot.width, shot.height), radius=10, fill=255)
    image.paste(shot, (left, top), mask)

    # Symbol, Name, Aussage
    icon = Image.open(ROOT / "crates" / "preset_manager" / "assets" / "icon.png").convert("RGBA")
    icon = icon.resize((112, 112), Image.LANCZOS)
    image.paste(icon, (64, 72), icon)
    draw.text((64, 205), "Preset Manager", font=font("segoeuib.ttf", 58), fill=TEXT)
    lines = [
        "Organize, understand and",
        "switch on or off the presets",
        "and profiles of Photoshop",
        "and Lightroom Classic.",
    ]
    y = 292
    for line in lines:
        draw.text((64, y), line, font=font("segoeui.ttf", 32), fill=TEXT)
        y += 44
    draw.text((64, 488), "Before/after preview · backups · updates", font=font("segoeui.ttf", 22), fill=MUTED)

    # Hinweis unten
    label = "Free · Open source · Windows"
    label_font = font("segoeuib.ttf", 20)
    width = draw.textlength(label, font=label_font)
    draw.rounded_rectangle((64, 548, 64 + width + 40, 548 + 44), radius=8, fill=ACCENT)
    draw.text((84, 556), label, font=label_font, fill=(255, 255, 255))

    out = ROOT / "docs" / "social-preview.png"
    image.save(out)
    print(f"geschrieben: {out}")


if __name__ == "__main__":
    main()
