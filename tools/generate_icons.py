#!/usr/bin/env python3
"""Generates the VYRO LINK app icons (iOS AppIcon set, Android adaptive and
legacy launcher icons) from one vector-style drawing. Run from the repo root:
    python3 tools/generate_icons.py
Needs Pillow. The mark: a V-shaped relay path between three phones. Flat colours only (Doc 16 D4)."""
import json
import os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLUE = (31, 79, 216)
WHITE = (255, 255, 255)
SS = 4  # supersampling factor


def mark(draw, size, cx, cy, scale):
    """Draws the white mark centred at (cx, cy); scale = 1.0 for a full icon.
    A V-shaped relay path: two phones (outer nodes) and the phone in between
    that carries the message (larger centre node)."""
    s = size * scale
    left = (cx - s * 0.22, cy - s * 0.14)
    mid = (cx, cy + s * 0.16)
    right = (cx + s * 0.22, cy - s * 0.14)
    width = int(s * 0.075)
    draw.line([left, mid, right], fill=WHITE, width=width, joint="curve")
    for (x, y), r in ((left, 0.075), (right, 0.075), (mid, 0.105)):
        rr = s * r
        draw.ellipse([x - rr, y - rr, x + rr, y + rr], fill=WHITE)


def full_icon(px):
    big = px * SS
    img = Image.new("RGB", (big, big), BLUE)
    mark(ImageDraw.Draw(img), big, big / 2, big / 2, 1.0)
    return img.resize((px, px), Image.LANCZOS)


def foreground(px):
    """Adaptive-icon foreground: 108 dp canvas, mark inside the 66 dp safe circle."""
    big = px * SS
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    mark(ImageDraw.Draw(img), big, big / 2, big / 2, 0.66)
    return img.resize((px, px), Image.LANCZOS)


def ios():
    folder = os.path.join(ROOT, "app/ios/Runner/Assets.xcassets/AppIcon.appiconset")
    with open(os.path.join(folder, "Contents.json")) as f:
        contents = json.load(f)
    for image in contents["images"]:
        points = float(image["size"].split("x")[0])
        scale = int(image["scale"].rstrip("x"))
        full_icon(round(points * scale)).save(os.path.join(folder, image["filename"]))


def android():
    res = os.path.join(ROOT, "app/android/app/src/main/res")
    for density, mult in (("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)):
        folder = os.path.join(res, f"mipmap-{density}")
        os.makedirs(folder, exist_ok=True)
        full_icon(round(48 * mult)).save(os.path.join(folder, "ic_launcher.png"))
        foreground(round(108 * mult)).save(os.path.join(folder, "ic_launcher_foreground.png"))
    anydpi = os.path.join(res, "mipmap-anydpi-v26")
    os.makedirs(anydpi, exist_ok=True)
    with open(os.path.join(anydpi, "ic_launcher.xml"), "w") as f:
        f.write(
            '<?xml version="1.0" encoding="utf-8"?>\n'
            '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
            '  <background android:drawable="@color/ic_launcher_background" />\n'
            '  <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n'
            '  <monochrome android:drawable="@mipmap/ic_launcher_foreground" />\n'
            "</adaptive-icon>\n"
        )
    with open(os.path.join(res, "values/ic_launcher_background.xml"), "w") as f:
        f.write(
            '<?xml version="1.0" encoding="utf-8"?>\n<resources>\n'
            '  <color name="ic_launcher_background">#1F4FD8</color>\n</resources>\n'
        )


if __name__ == "__main__":
    ios()
    android()
    full_icon(512).save(os.path.join(ROOT, "app/assets_icon_preview.png"))
    print("icons written")
