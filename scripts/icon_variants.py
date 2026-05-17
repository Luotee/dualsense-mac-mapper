"""Generate gruvbox-themed 256x256 transparent-bg controller icons
from ICON.png line drawing. Floods each enclosed region with a
palette color, recolors lines, makes outside transparent."""
from __future__ import annotations
from pathlib import Path

from PIL import Image, ImageDraw
import numpy as np

SRC = Path('/mnt/c/Users/Joe96/Downloads/ICON.png')
OUT_DIR = Path('/mnt/c/Users/Joe96/Downloads/icon-variants')
OUT_DIR.mkdir(exist_ok=True)

# Seed points (x, y) inside each enclosed region of ICON.png (1030x720)
SEEDS = {
    'body':       [(515, 500), (515, 600), (130, 450)],   # multi-seed for safety
    'touchpad':   [(515, 165)],
    'dpad_up':    [(218, 130)],
    'dpad_right': [(290, 200)],
    'dpad_down':  [(218, 270)],
    'dpad_left':  [(146, 200)],
    'face_up':    [(820, 130)],
    'face_right': [(905, 200)],
    'face_down':  [(820, 275)],
    'face_left':  [(735, 200)],
    'ps':         [(515, 380)],
    'lstick_in':  [(340, 360)],
    'rstick_in':  [(680, 360)],
}


def hex2rgb(h: str) -> tuple[int, int, int]:
    h = h.lstrip('#')
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))


def render(palette: dict, out_path: Path):
    src = Image.open(SRC).convert('RGB')
    W, H = src.size

    # Capture line mask from ORIGINAL image up-front so later flood-fill
    # body colors (which can themselves be dark) won't be misidentified
    # as lines during recolor.
    orig_arr = np.array(src)
    line_mask = (
        (orig_arr[:, :, 0] < 180)
        & (orig_arr[:, :, 1] < 180)
        & (orig_arr[:, :, 2] < 180)
    )
    # Capture AA strength: how dark each line pixel was (0..1, 0 = pure black)
    line_strength = 1.0 - orig_arr[:, :, 0].astype(np.float32) / 180.0
    line_strength = np.clip(line_strength, 0.0, 1.0)

    # Step 1: mark outside-body with magenta sentinel (to remove later)
    OUTSIDE = (255, 0, 255)
    ImageDraw.floodfill(src, (5, 5), OUTSIDE, thresh=100)
    for corner in [(W - 5, 5), (5, H - 5), (W - 5, H - 5)]:
        px = src.getpixel(corner)
        if px[0] > 200 and px[1] > 200 and px[2] > 200:
            ImageDraw.floodfill(src, corner, OUTSIDE, thresh=100)

    # Step 2: explicit region fills
    region_to_color = {
        'body':       hex2rgb(palette['body']),
        'touchpad':   hex2rgb(palette['touchpad']),
        'dpad_up':    hex2rgb(palette['dpad']),
        'dpad_right': hex2rgb(palette['dpad']),
        'dpad_down':  hex2rgb(palette['dpad']),
        'dpad_left':  hex2rgb(palette['dpad']),
        'face_up':    hex2rgb(palette['face_up']),
        'face_right': hex2rgb(palette['face_right']),
        'face_down':  hex2rgb(palette['face_down']),
        'face_left':  hex2rgb(palette['face_left']),
        'ps':         hex2rgb(palette['ps']),
        'lstick_in':  hex2rgb(palette['stick']),
        'rstick_in':  hex2rgb(palette['stick']),
    }
    for region, seeds in SEEDS.items():
        color = region_to_color[region]
        for sx, sy in seeds:
            px = src.getpixel((sx, sy))
            if px[0] > 200 and px[1] > 200 and px[2] > 200:
                ImageDraw.floodfill(src, (sx, sy), color, thresh=100)

    arr = np.array(src)
    body = hex2rgb(palette['body'])

    # Capture outside-mask BEFORE recolor blend (so AA at outside-line edges
    # doesn't break the mask) — outside = exact magenta sentinel right now.
    outside_mask = (
        (arr[:, :, 0] == OUTSIDE[0])
        & (arr[:, :, 1] == OUTSIDE[1])
        & (arr[:, :, 2] == OUTSIDE[2])
    )

    # Step 3: any remaining bright (>235) interior pixels → body color
    bright_mask = (arr[:, :, 0] > 235) & (arr[:, :, 1] > 235) & (arr[:, :, 2] > 235)
    arr[bright_mask] = body

    # Step 4: recolor ORIGINAL line pixels (using stored line_mask),
    # blending by AA strength so edges stay smooth.
    line = np.array(hex2rgb(palette['line']), dtype=np.float32)
    strength = line_strength[..., None]
    arr_f = arr.astype(np.float32)
    arr_f = np.where(line_mask[..., None], strength * line + (1.0 - strength) * arr_f, arr_f)
    arr = np.clip(arr_f, 0, 255).astype(np.uint8)
    # Force exact line color where the original line was very dark
    arr[line_mask & (orig_arr[:, :, 0] < 80)] = line.astype(np.uint8)

    # Step 5: outside → transparent
    rgba = np.dstack([arr, np.full((H, W), 255, dtype=np.uint8)])
    rgba[outside_mask] = (0, 0, 0, 0)

    result = Image.fromarray(rgba, 'RGBA')

    # Step 6: tight-crop to non-transparent bbox, fit into 256x256
    alpha = rgba[:, :, 3]
    ys, xs = np.where(alpha > 0)
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    result = result.crop(bbox)

    TARGET = 256
    w, h = result.size
    scale = min((TARGET - 8) / w, (TARGET - 8) / h)
    new_w, new_h = int(w * scale), int(h * scale)
    result = result.resize((new_w, new_h), Image.LANCZOS)

    canvas = Image.new('RGBA', (TARGET, TARGET), (0, 0, 0, 0))
    canvas.paste(result, ((TARGET - new_w) // 2, (TARGET - new_h) // 2), result)
    canvas.save(out_path)


# Gruvbox Dark palette (mirrors rust/web/palette.css + standard gruvbox)
G = {
    'bg':       '#282828',
    'bg1':      '#3c3836',
    'bg2':      '#504945',
    'bg3':      '#665c54',
    'fg':       '#ebdbb2',
    'fg0':      '#fbf1c7',
    'fg2':      '#d5c4a1',
    'fg3':      '#bdae93',
    'fg4':      '#a89984',
    'blue':     '#83a598',
    'aqua':     '#8ec07c',
    'green':    '#b8bb26',
    'orange':   '#fe8019',
    'yellow':   '#fabd2f',
    'red':      '#fb4934',
    'purple':   '#d3869b',
    'neutral_blue':   '#458588',
    'neutral_red':    '#cc241d',
    'neutral_green':  '#98971a',
    'neutral_orange': '#d65d0e',
}

VARIANTS = {
    'v1-classic': {
        'body': G['bg1'], 'touchpad': G['bg2'], 'dpad': G['fg4'],
        'face_up': G['green'], 'face_right': G['red'],
        'face_down': G['blue'], 'face_left': G['orange'],
        'stick': G['bg3'], 'ps': G['yellow'], 'line': G['fg'],
    },
    'v2-cream': {
        'body': G['fg'], 'touchpad': G['fg2'], 'dpad': G['bg3'],
        'face_up': G['neutral_green'], 'face_right': G['neutral_red'],
        'face_down': G['neutral_blue'], 'face_left': G['neutral_orange'],
        'stick': G['fg3'], 'ps': G['orange'], 'line': G['bg'],
    },
    'v3-mono': {
        'body': G['bg1'], 'touchpad': G['bg2'], 'dpad': G['fg4'],
        'face_up': G['fg'], 'face_right': G['fg'],
        'face_down': G['fg'], 'face_left': G['fg'],
        'stick': G['bg3'], 'ps': G['yellow'], 'line': G['fg'],
    },
    'v4-high-contrast': {
        'body': G['bg'], 'touchpad': G['bg1'], 'dpad': G['yellow'],
        'face_up': G['green'], 'face_right': G['red'],
        'face_down': G['blue'], 'face_left': G['orange'],
        'stick': G['bg2'], 'ps': G['fg0'], 'line': G['fg0'],
    },
    'v5-blue-brand': {
        'body': G['neutral_blue'], 'touchpad': G['bg1'], 'dpad': G['fg'],
        'face_up': G['green'], 'face_right': G['red'],
        'face_down': G['fg'], 'face_left': G['orange'],
        'stick': G['bg2'], 'ps': G['yellow'], 'line': G['fg0'],
    },
    'v6-inverted': {
        'body': G['fg'], 'touchpad': G['fg2'], 'dpad': G['bg3'],
        'face_up': G['green'], 'face_right': G['red'],
        'face_down': G['blue'], 'face_left': G['orange'],
        'stick': G['bg3'], 'ps': G['yellow'], 'line': G['bg'],
    },
}

if __name__ == '__main__':
    for name, palette in VARIANTS.items():
        out = OUT_DIR / f'{name}.png'
        render(palette, out)
        print(f'wrote {out}')
