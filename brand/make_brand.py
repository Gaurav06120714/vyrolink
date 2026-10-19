"""VYRO LINK brand assets, drawn from exact geometry (no generator, no filters).

The mark: a V whose left arm is one solid stroke (the message leaves your
phone) and whose right arm breaks into hops that end in a dot (it jumps from
phone to phone and arrives). One flat colour, deliberate asymmetry.

Run: python3 brand/make_brand.py  -> brand/*.svg, brand/*.png
"""
import math
import pathlib
from PIL import Image, ImageDraw, ImageFont

HERE = pathlib.Path(__file__).parent
BLUE = (0x1F, 0x4F, 0xD8)
PAPER = (0xF6, 0xF6, 0xF3)
INK = (0x12, 0x14, 0x17)
FONT = "/Users/apple/Desktop/MyProjects/Recordly/src/assets/fonts/dm-sans/DMSans-Variable.ttf"

# Geometry on a 1024 unit square.
W = 84                       # stroke width
A, V, B = (286, 270), (512, 758), (738, 270)
HOP_EDGES = [(0.0, 0.47), (0.57, 0.80)]   # solid parts of the right arm, as edge positions
DOT_R = W / 2 * 1.12         # the arrival dot is a touch heavier than the stroke


def lerp(p, q, t):
    return (p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t)


def capsules():
    """(start, end) centre points of every round-capped stroke in the mark."""
    r = W / 2
    L = math.dist(V, B)
    out = [(A, V)]
    for e0, e1 in HOP_EDGES:
        # The first hop starts at the vertex itself, so the V's join stays round.
        t0 = 0.0 if e0 == 0 else e0 + r / L
        out.append((lerp(V, B, t0), lerp(V, B, e1 - r / L)))
    return out


def svg(bg=BLUE, fg=PAPER, size=1024, square=True):
    hexc = lambda c: "#%02X%02X%02X" % c
    lines = "".join(
        f'<line x1="{p[0]:.1f}" y1="{p[1]:.1f}" x2="{q[0]:.1f}" y2="{q[1]:.1f}"/>' for p, q in capsules()
    )
    back = f'<rect width="1024" height="1024" fill="{hexc(bg)}"/>' if square else ""
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="{size}" height="{size}">'
        f"{back}<g stroke=\"{hexc(fg)}\" stroke-width=\"{W}\" stroke-linecap=\"round\">{lines}</g>"
        f'<circle cx="{B[0]}" cy="{B[1]}" r="{DOT_R:.1f}" fill="{hexc(fg)}"/></svg>\n'
    )


def draw_mark(img, fg, scale, offset=(0, 0)):
    """Draws the mark with PIL at `scale` pixels per unit (supersampled by the caller)."""
    d = ImageDraw.Draw(img)
    ox, oy = offset
    tr = lambda p: (ox + p[0] * scale, oy + p[1] * scale)
    r = W / 2 * scale
    for p, q in capsules():
        P, Q = tr(p), tr(q)
        ang = math.atan2(Q[1] - P[1], Q[0] - P[0]) + math.pi / 2
        dx, dy = math.cos(ang) * r, math.sin(ang) * r
        d.polygon([(P[0] + dx, P[1] + dy), (Q[0] + dx, Q[1] + dy), (Q[0] - dx, Q[1] - dy), (P[0] - dx, P[1] - dy)], fill=fg)
        for c in (P, Q):
            d.ellipse([c[0] - r, c[1] - r, c[0] + r, c[1] + r], fill=fg)
    c, rr = tr(B), DOT_R * scale
    d.ellipse([c[0] - rr, c[1] - rr, c[0] + rr, c[1] + rr], fill=fg)


def icon(px, bg=BLUE, fg=PAPER):
    ss = 4
    img = Image.new("RGB", (px * ss, px * ss), bg)
    draw_mark(img, fg, px * ss / 1024)
    return img.resize((px, px), Image.LANCZOS)


def font(size, weight):
    f = ImageFont.truetype(FONT, size)
    f.set_variation_by_name(weight)
    return f


def feature_graphic():
    """Play's 1024 x 500 banner: mark, wordmark and one honest line."""
    ss = 3
    w, h = 1024 * ss, 500 * ss
    img = Image.new("RGB", (w, h), BLUE)
    mark = 300 * ss
    draw_mark(img, PAPER, mark / 1024, offset=(90 * ss, (h - mark) // 2 + 6 * ss))
    d = ImageDraw.Draw(img)
    x = 420 * ss
    d.text((x, 168 * ss), "VYRO LINK", font=font(84 * ss, b"SemiBold"), fill=PAPER)
    d.text((x, 280 * ss), "Messages that travel", font=font(36 * ss, b"Regular"), fill=PAPER)
    d.text((x, 326 * ss), "phone to phone.", font=font(36 * ss, b"Regular"), fill=PAPER)
    return img.resize((1024, 500), Image.LANCZOS)


def foreground(px):
    """Android adaptive-icon foreground: transparent, mark inside the safe zone."""
    ss = 4
    img = Image.new("RGBA", (px * ss, px * ss), (0, 0, 0, 0))
    inner = px * ss * 0.70
    off = (px * ss - inner) / 2
    draw_mark(img, PAPER + (255,), inner / 1024, offset=(off, off))
    return img.resize((px, px), Image.LANCZOS)


def install(app):
    """Writes every launcher, iOS and web icon of the app from this geometry."""
    res = app / "android/app/src/main/res"
    for dpi, legacy, fg in [("mdpi", 48, 108), ("hdpi", 72, 162), ("xhdpi", 96, 216), ("xxhdpi", 144, 324), ("xxxhdpi", 192, 432)]:
        icon(legacy).save(res / f"mipmap-{dpi}/ic_launcher.png")
        foreground(fg).save(res / f"mipmap-{dpi}/ic_launcher_foreground.png")
    ios = app / "ios/Runner/Assets.xcassets/AppIcon.appiconset"
    for f in ios.glob("Icon-App-*.png"):
        size, scale = f.stem.removeprefix("Icon-App-").split("@")
        icon(round(float(size.split("x")[0]) * int(scale.removesuffix("x")))).save(f)
    web = app / "web"
    for px in (192, 512):
        icon(px).save(web / f"icons/Icon-{px}.png")
        icon(px).save(web / f"icons/Icon-maskable-{px}.png")
    icon(32).save(web / "favicon.png")


if __name__ == "__main__":
    (HERE / "logo-mark.svg").write_text(svg(square=False, fg=BLUE))
    (HERE / "app-icon.svg").write_text(svg())
    icon(512).save(HERE / "play-icon-512.png")
    icon(1024).save(HERE / "app-icon-1024.png")
    feature_graphic().save(HERE / "play-feature-graphic-1024x500.png")
    # Small-size check: the mark must still read at launcher and favicon sizes.
    strip = Image.new("RGB", (48 + 32 + 16 + 40, 48), (255, 255, 255))
    for x, s in [(0, 48), (56, 32), (96, 16)]:
        strip.paste(icon(s), (x, (48 - s) // 2))
    strip.save(HERE / "small-sizes.png")
    install(HERE.parent / "app")
    print("ok")
