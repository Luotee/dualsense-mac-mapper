#!/usr/bin/env python3
"""Render side-by-side mockups of the GUI under different colour
palettes so the user can pick a direction before we commit any CSS
change. Each mockup shows the toolbar, controller silhouette, chip
list rows, and a settings card — exactly the surfaces the user
inspects in `dualsense-mapper`'s main window.

Run with `python3 scripts/palette_mockup.py` from the repo root.
"""
from PIL import Image, ImageDraw, ImageFont
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from build_icons import _body_polygon, GUI_W, GUI_H  # noqa: E402

OUT = REPO / "dist" / "palette_mockups.png"

# Each palette mimics the variables in `rust/web/solarized.css`:
#   bg, card, border_light, border, text, text_strong, muted,
#   accent, soft, success, macro, macro_soft, yellow, red
PALETTES = [
    {
        "name": "Solarized Light (current)",
        "bg": "#fdf6e3", "card": "#eee8d5", "text": "#657b83",
        "text_strong": "#073642", "muted": "#839496",
        "accent": "#268bd2", "soft": "#dceefb",
        "macro": "#cb4b16", "macro_soft": "#fbe2d4",
        "success": "#859900", "border": "#d4ccb4",
    },
    {
        "name": "Solarized Dark",
        "bg": "#002b36", "card": "#073642", "text": "#93a1a1",
        "text_strong": "#fdf6e3", "muted": "#586e75",
        "accent": "#268bd2", "soft": "#0f4151",
        "macro": "#cb4b16", "macro_soft": "#3d2519",
        "success": "#859900", "border": "#0d3e4c",
    },
    {
        "name": "Nord (cool slate)",
        "bg": "#eceff4", "card": "#e5e9f0", "text": "#4c566a",
        "text_strong": "#2e3440", "muted": "#7b8794",
        "accent": "#5e81ac", "soft": "#dde4ee",
        "macro": "#bf616a", "macro_soft": "#f3dadf",
        "success": "#a3be8c", "border": "#d8dee9",
    },
    {
        "name": "Catppuccin Latte",
        "bg": "#eff1f5", "card": "#e6e9ef", "text": "#4c4f69",
        "text_strong": "#1e2030", "muted": "#7c7f93",
        "accent": "#1e66f5", "soft": "#dce3f7",
        "macro": "#fe640b", "macro_soft": "#fde2cf",
        "success": "#40a02b", "border": "#dce0e8",
    },
    {
        "name": "Gruvbox Light",
        "bg": "#fbf1c7", "card": "#f2e5bc", "text": "#3c3836",
        "text_strong": "#282828", "muted": "#7c6f64",
        "accent": "#076678", "soft": "#d6e7eb",
        "macro": "#af3a03", "macro_soft": "#f5d2bd",
        "success": "#79740e", "border": "#ebdbb2",
    },
    {
        "name": "Tokyo Night",
        "bg": "#1a1b26", "card": "#24283b", "text": "#a9b1d6",
        "text_strong": "#c0caf5", "muted": "#565f89",
        "accent": "#7aa2f7", "soft": "#2c3257",
        "macro": "#ff9e64", "macro_soft": "#3b2d20",
        "success": "#9ece6a", "border": "#2f3447",
    },
]

# ── Mockup geometry (per palette card) ──────────────────────────────
CARD_W   = 540
CARD_H   = 420
GAP_X    = 24
GAP_Y    = 24


def hex_to_rgb(c):
    c = c.lstrip("#")
    return tuple(int(c[i:i+2], 16) for i in (0, 2, 4))


def draw_controller(d, p, ox, oy, scale):
    """Draw the GUI's controller silhouette filled in palette.card,
    overlaid with key/macro/unbound coloured face buttons."""
    sx, sy = scale, scale
    pts = [(ox + x * sx, oy + y * sy) for x, y in _body_polygon()]
    d.polygon(pts, fill=hex_to_rgb(p["card"]),
              outline=hex_to_rgb(p["border"]))

    def sc(x, y):
        return ox + x * sx, oy + y * sy

    def circle(cx, cy, r, fill, outline=None):
        x1, y1 = sc(cx - r, cy - r)
        x2, y2 = sc(cx + r, cy + r)
        d.ellipse([x1, y1, x2, y2], fill=fill, outline=outline)

    def rrect(x, y, w, h, fill, rad=2):
        x1, y1 = sc(x, y)
        x2, y2 = sc(x + w, y + h)
        d.rounded_rectangle([x1, y1, x2, y2], radius=rad * sx, fill=fill)

    accent = hex_to_rgb(p["accent"])
    macro  = hex_to_rgb(p["macro"])
    muted  = hex_to_rgb(p["muted"])
    bg     = hex_to_rgb(p["bg"])
    border = hex_to_rgb(p["border"])

    # Touchpad notch
    rrect(101, 36, 38, 16, bg, rad=5)
    # D-pad arms
    rrect(52, 54, 14, 5, accent, rad=1)
    rrect(57, 49,  4, 14, accent, rad=1)
    # Face buttons
    circle(184, 50, 4, accent)          # Triangle
    circle(192, 58, 4, accent)          # Circle
    circle(184, 66, 4, accent)          # Cross
    circle(176, 58, 4, accent)          # Square
    # Stick wells with bound stick dirs (use macro to show variety)
    circle(84,  82, 9, bg, outline=border)
    circle(84,  82, 5, accent)
    circle(156, 82, 9, bg, outline=border)
    circle(156, 82, 5, muted)
    # PS marker
    circle(120, 62, 3, muted)


def draw_card(palette):
    p = palette
    img = Image.new("RGB", (CARD_W, CARD_H), hex_to_rgb(p["bg"]))
    d = ImageDraw.Draw(img)

    # Toolbar bar at top
    d.rectangle([0, 0, CARD_W, 40], fill=hex_to_rgb(p["card"]))
    d.rectangle([0, 39, CARD_W, 40], fill=hex_to_rgb(p["border"]))

    # Status dot + text
    d.ellipse([14, 16, 22, 24], fill=hex_to_rgb(p["success"]))
    d.text((28, 13), "Connected · DualSense · USB", fill=hex_to_rgb(p["text_strong"]))

    # Tabs bar
    d.rectangle([0, 40, CARD_W, 72], fill=hex_to_rgb(p["bg"]))
    d.rectangle([0, 71, CARD_W, 72], fill=hex_to_rgb(p["border"]))
    d.text((14, 50), "Mappings", fill=hex_to_rgb(p["accent"]))
    d.rectangle([8, 70, 80, 72], fill=hex_to_rgb(p["accent"]))
    d.text((100, 50), "Macros", fill=hex_to_rgb(p["muted"]))
    d.text((164, 50), "Settings", fill=hex_to_rgb(p["muted"]))

    # Palette name footer
    d.text((14, CARD_H - 20), p["name"], fill=hex_to_rgb(p["text_strong"]))

    # ── Controller silhouette (left half of mappings area) ──
    # Render scaled controller. SVG viewBox is 240x130 — scale to fit
    # a 320×180 region inside the card.
    target_w, target_h = 320, 180
    scale = min(target_w / GUI_W, target_h / GUI_H) * 0.95
    cw = GUI_W * scale
    ch = GUI_H * scale
    ox = 16 + (target_w - cw) / 2
    oy = 90 + (target_h - ch) / 2
    draw_controller(d, p, ox, oy, scale)

    # ── Chip list (right column) ──
    chip_x = 350
    chip_y = 90
    rows = [
        ("Cross",   "Alt",     "key"),
        ("Circle",  "z",       "key"),
        ("Square",  "Shift",   "key"),
        ("Triangle","a",       "key"),
        ("L1",      None,      "unbound"),
        ("R1",      None,      "unbound"),
        ("L2",      "macro_A", "macro"),
        ("R2",      "Shift",   "key"),
        ("D-pad ↑", "Up",      "key"),
    ]
    for i, (label, val, kind) in enumerate(rows):
        ry = chip_y + i * 28
        # Row background
        d.rounded_rectangle(
            [chip_x, ry, chip_x + 170, ry + 24],
            radius=4, fill=hex_to_rgb(p["card"]),
        )
        d.text((chip_x + 10, ry + 6), label, fill=hex_to_rgb(p["text"]))
        if kind == "key":
            # key chip
            tag_w = max(28, 14 + 7 * len(val))
            d.rounded_rectangle(
                [chip_x + 170 - tag_w - 8, ry + 5,
                 chip_x + 170 - 8, ry + 19],
                radius=3, fill=hex_to_rgb(p["soft"]),
            )
            d.text(
                (chip_x + 170 - tag_w - 4, ry + 7),
                val, fill=hex_to_rgb(p["accent"]),
            )
        elif kind == "macro":
            tag_w = 60
            d.rounded_rectangle(
                [chip_x + 170 - tag_w - 8, ry + 5,
                 chip_x + 170 - 8, ry + 19],
                radius=3, fill=hex_to_rgb(p["macro_soft"]),
            )
            d.text(
                (chip_x + 170 - tag_w - 4, ry + 7),
                "⚡ " + val, fill=hex_to_rgb(p["macro"]),
            )
        else:
            d.text(
                (chip_x + 170 - 60, ry + 7),
                "unbound", fill=hex_to_rgb(p["muted"]),
            )

    # Border around card so light themes don't blend into the canvas
    d.rectangle([0, 0, CARD_W - 1, CARD_H - 1],
                outline=hex_to_rgb(p["border"]))
    return img


def main():
    cols = 2
    rows = (len(PALETTES) + cols - 1) // cols
    W = cols * CARD_W + (cols + 1) * GAP_X
    H = rows * CARD_H + (rows + 1) * GAP_Y
    sheet = Image.new("RGB", (W, H), (240, 240, 240))
    for i, p in enumerate(PALETTES):
        r, c = divmod(i, cols)
        x = GAP_X + c * (CARD_W + GAP_X)
        y = GAP_Y + r * (CARD_H + GAP_Y)
        sheet.paste(draw_card(p), (x, y))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(OUT)
    print(f"saved {OUT.relative_to(REPO)}")


if __name__ == "__main__":
    main()
