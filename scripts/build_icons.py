#!/usr/bin/env python3
"""Generate the three ICO assets for the GUI: app icon, tray-connected,
tray-disconnected. Each ICO is multi-resolution (16/32/48/256) with the
small sizes hand-tuned for legibility instead of just down-sampled.

Design: a stylised DualSense rendered in the same proportions as the
in-app SVG controller (`rust/web/controller.js`). At 256 / 48 the body
carries the GUI's touchpad, d-pad cross, four face-button dots, two
stick wells, PS / Share / Options markers — all rendered as negative
cutouts in a solid silhouette so the icon reads as a clear stencil.
At 32 the detail drops to body + d-pad + sticks; at 16 it is a pure
silhouette. Triggers (L2 / R2) and shoulders (L1 / R1) — the parts
that protrude above the body — are dropped per user request.

Solarized palette so the icon sits next to the in-app SVG without
colour drift.

Run with `python3 scripts/build_icons.py` from the repo root.
"""
import struct
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw

REPO = Path(__file__).resolve().parent.parent
ICONS_DIR = REPO / "rust" / "icons"

# Solarized palette — same hex values as rust/web/solarized.css.
ACCENT  = "#268bd2"  # app icon
SUCCESS = "#859900"  # tray when controller connected
MUTED   = "#839496"  # tray when controller disconnected

# GUI controller geometry (matches rust/web/controller.js):
# - SVG viewBox is 240 × 130 (wide). For a square ICO we centre that
#   in a `s × s` canvas with a small breathing margin. The controller
#   silhouette therefore sits in the middle band of the icon with
#   transparent padding above and below — exactly how the in-app SVG
#   renders when its container is taller than 240×130.

GUI_W, GUI_H = 240, 130


def _bezier_quad(p0, p1, p2, steps=24):
    """Sample a quadratic Bezier (SVG Q command) at `steps+1` points."""
    out = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        x = u * u * p0[0] + 2 * u * t * p1[0] + t * t * p2[0]
        y = u * u * p0[1] + 2 * u * t * p1[1] + t * t * p2[1]
        out.append((x, y))
    return out


def _body_polygon():
    """Return the GUI's BODY_PATH (`controller.js`) as a list of (x, y)
    points in SVG (240×130) coordinates, suitable for `ImageDraw.polygon`."""
    pts = []
    # Path: M 50 30 Q 38 30 36 50 Q 32 80 62 92 Q 75 102 95 102 L 145 102
    #       Q 165 102 178 92 Q 208 80 204 50 Q 202 30 190 30 L 50 30 Z
    pts.append((50, 30))
    pts += _bezier_quad((50, 30), (38, 30), (36, 50))[1:]
    pts += _bezier_quad((36, 50), (32, 80), (62, 92))[1:]
    pts += _bezier_quad((62, 92), (75, 102), (95, 102))[1:]
    pts.append((145, 102))
    pts += _bezier_quad((145, 102), (165, 102), (178, 92))[1:]
    pts += _bezier_quad((178, 92), (208, 80), (204, 50))[1:]
    pts += _bezier_quad((204, 50), (202, 30), (190, 30))[1:]
    pts.append((50, 30))
    return pts


def draw_pad(size: int, color: str) -> Image.Image:
    """Render the gamepad into an RGBA image of `size x size` with detail
    hand-tuned per target size. The controller's natural viewBox is wide
    (240×130 aspect 1.85), so naively fitting it by width leaves huge
    top/bottom padding in a square icon. We render onto an oversize
    supersampled canvas, then crop the transparent border tight to the
    controller silhouette and rescale to fill ~92% of the final square
    — same trick common Windows app icons use to occupy the canvas."""
    SUPER = 4
    s = size * SUPER
    # Use a wider intermediate canvas (square × 1.3) so a 90%-wide
    # render doesn't get clipped before we crop and rescale.
    intermediate = int(s * 1.3)
    img = Image.new("RGBA", (intermediate, intermediate), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Compute scale so the GUI 240×130 viewBox fits the intermediate
    # canvas width at 95%. The crop-and-rescale step below will then
    # bring the controller back up to ~92% of the final square's
    # *shorter* dimension, which on a horizontal silhouette means the
    # width fills the canvas tightly.
    margin = 0.025
    scale = intermediate * (1 - 2 * margin) / GUI_W
    ox = (intermediate - GUI_W * scale) / 2
    oy = (intermediate - GUI_H * scale) / 2

    def sc(x, y):
        return (ox + x * scale, oy + y * scale)

    def rect(x, y, w, h, fill, radius=0):
        x1, y1 = sc(x, y)
        x2, y2 = sc(x + w, y + h)
        if radius > 0:
            d.rounded_rectangle([x1, y1, x2, y2], radius=radius * scale,
                                fill=fill)
        else:
            d.rectangle([x1, y1, x2, y2], fill=fill)

    def circle(cx, cy, r, fill):
        x1, y1 = sc(cx - r, cy - r)
        x2, y2 = sc(cx + r, cy + r)
        d.ellipse([x1, y1, x2, y2], fill=fill)

    # ── Body silhouette (filled) ──
    body_pts = [sc(x, y) for x, y in _body_polygon()]
    d.polygon(body_pts, fill=color)

    # Internal detail level depends on the final size. 16px stays a
    # clean silhouette; 32 adds stick wells and d-pad; 48 adds face
    # buttons and touchpad; 256 adds PS / Share / Options.
    if size >= 24:
        circle(84,  82, 9, (0, 0, 0, 0))
        circle(156, 82, 9, (0, 0, 0, 0))
        rect(52, 54, 14, 5, (0, 0, 0, 0), radius=1)   # d-pad horiz arm
        rect(57, 49,  4, 14, (0, 0, 0, 0), radius=1)  # d-pad vert arm
    if size >= 40:
        for cx, cy in [(184, 50), (192, 58), (184, 66), (176, 58)]:
            circle(cx, cy, 4, (0, 0, 0, 0))
        rect(101, 36, 38, 16, (0, 0, 0, 0), radius=5)  # touchpad
    if size >= 96:
        circle(120, 62, 3, (0, 0, 0, 0))               # PS button
        rect(82,  38, 7, 3, (0, 0, 0, 0), radius=1)    # Share
        rect(151, 38, 7, 3, (0, 0, 0, 0), radius=1)    # Options

    # ── Crop transparent border and rescale to fill ~92% of canvas ──
    # Without this step a wide-aspect controller leaves big empty bands
    # top/bottom in a square ICO. Tight-crop the alpha bbox and resize
    # so the silhouette's longer dimension fills the target square.
    bbox = img.getbbox()
    if bbox is None:
        return img.resize((size, size), Image.LANCZOS)
    cropped = img.crop(bbox)
    target = int(size * 0.94)
    cw, ch = cropped.size
    ratio = target / max(cw, ch)
    new_size = (max(1, int(round(cw * ratio))),
                max(1, int(round(ch * ratio))))
    scaled = cropped.resize(new_size, Image.LANCZOS)
    final = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    final.paste(
        scaled,
        ((size - new_size[0]) // 2, (size - new_size[1]) // 2),
        scaled,
    )
    return final


def write_ico(path: Path, color: str, sizes: list[int]) -> None:
    """Build a multi-resolution ICO with per-size hand-tuned drawings."""
    sizes = sorted(set(sizes), reverse=True)
    images = [draw_pad(z, color) for z in sizes]

    # PIL's `Image.save(format="ICO", sizes=...)` only re-samples one
    # master image, which loses our per-size tuning. Encode each frame
    # as a PNG blob and stitch into the ICO container manually.
    blobs = []
    for im in images:
        buf = BytesIO()
        im.save(buf, format="PNG")
        blobs.append(buf.getvalue())

    n = len(images)
    header = struct.pack("<HHH", 0, 1, n)
    offset = 6 + 16 * n
    entries = b""
    for im, blob in zip(images, blobs):
        w, h = im.size
        wb = 0 if w >= 256 else w
        hb = 0 if h >= 256 else h
        entries += struct.pack(
            "<BBBBHHII",
            wb, hb, 0, 0, 1, 32, len(blob), offset,
        )
        offset += len(blob)

    with open(path, "wb") as f:
        f.write(header)
        f.write(entries)
        for blob in blobs:
            f.write(blob)

    print(f"  wrote {path.relative_to(REPO)} ({', '.join(str(z) for z in sizes)})")


def main() -> None:
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    print(f"Writing ICOs to {ICONS_DIR.relative_to(REPO)}/")
    sizes = [16, 32, 48, 256]
    write_ico(ICONS_DIR / "icon.ico",              ACCENT,  sizes)
    write_ico(ICONS_DIR / "tray-connected.ico",    SUCCESS, sizes)
    write_ico(ICONS_DIR / "tray-disconnected.ico", MUTED,   sizes)
    print("done.")


if __name__ == "__main__":
    main()
