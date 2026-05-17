#!/usr/bin/env python3
"""Trace Share / Options pill shapes from a separate PNG that has them
drawn, then emit their viewBox-space paths so build_silhouette_preview
can render them. The new PNG has the same body aspect as the original,
so we recompute the viewBox transform from THIS PNG's body bbox.

Usage:
  python3 scripts/trace_share_options.py <png-with-share-options>

Writes: dist/share_options_geom.json
"""
import json
import sys
from pathlib import Path

import cv2
import numpy as np


VIEWBOX_W = 240.0
VIEWBOX_H = 130.0
MARGIN_TOP = 14.0
MARGIN_BOT = 4.0
MARGIN_SIDE = 4.0


def _smooth_polygon_to_qpath(pts):
    n = len(pts)
    mids = [(
        (pts[i, 0] + pts[(i + 1) % n, 0]) / 2,
        (pts[i, 1] + pts[(i + 1) % n, 1]) / 2,
    ) for i in range(n)]
    parts = [f"M {mids[-1][0]:.2f} {mids[-1][1]:.2f}"]
    for i in range(n):
        parts.append(
            f"Q {pts[i, 0]:.2f} {pts[i, 1]:.2f} "
            f"{mids[i][0]:.2f} {mids[i][1]:.2f}"
        )
    parts.append("Z")
    return " ".join(parts)


def trace(src: Path):
    img = cv2.imread(str(src), cv2.IMREAD_GRAYSCALE)
    H_img, W_img = img.shape

    _, bw = cv2.threshold(img, 200, 255, cv2.THRESH_BINARY_INV)
    k = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    closed = cv2.morphologyEx(bw, cv2.MORPH_CLOSE, k, iterations=3)
    contours, hier = cv2.findContours(closed, cv2.RETR_TREE, cv2.CHAIN_APPROX_NONE)
    hier = hier[0] if hier is not None else []

    items = []
    for i, c in enumerate(contours):
        a = cv2.contourArea(c)
        if a < 50:
            continue
        x, y, w, h = cv2.boundingRect(c)
        items.append({
            "i": i, "c": c, "a": a,
            "x": x, "y": y, "w": w, "h": h,
            "bbox_area": w * h,
            "parent": hier[i][3] if len(hier) else -1,
        })

    # Body = top-level contour with the largest BBOX area (covers most of img).
    top_level = [it for it in items if it["parent"] == -1]
    body = max(top_level, key=lambda it: it["bbox_area"])
    bx, by, bw_px, bh_px = body["x"], body["y"], body["w"], body["h"]

    avail_w = VIEWBOX_W - 2 * MARGIN_SIDE
    avail_h = VIEWBOX_H - MARGIN_TOP - MARGIN_BOT
    scale = min(avail_w / bw_px, avail_h / bh_px)
    ox = (VIEWBOX_W - bw_px * scale) / 2 - bx * scale
    oy = MARGIN_TOP - by * scale

    def tx(p): return p * scale + ox
    def ty(p): return p * scale + oy

    # Touchpad outer = top-level contour with aspect>1.6, in upper half,
    # AND has at least one direct child (the inner light-bar line).
    upper_cutoff = by + bh_px * 0.45
    children_of = {}
    for it in items:
        children_of.setdefault(it["parent"], []).append(it["i"])
    touchpad = None
    for it in items:
        if it is body or it["parent"] != -1:
            continue
        cy_px = it["y"] + it["h"] / 2
        if cy_px > upper_cutoff:
            continue
        if it["w"] / it["h"] < 1.6 or it["w"] < 200:
            continue
        if not children_of.get(it["i"]):
            continue
        if touchpad is None or it["bbox_area"] < touchpad["bbox_area"]:
            touchpad = it
    tp_top_y = touchpad["y"] if touchpad else by

    # Share/Options sit directly OUTSIDE touchpad's left and right edges,
    # vertically within touchpad's upper portion. Lock to those zones.
    if touchpad is None:
        raise SystemExit("touchpad not found — cannot anchor Share/Options")
    tp_x = touchpad["x"]
    tp_y = touchpad["y"]
    tp_w = touchpad["w"]
    tp_h = touchpad["h"]
    tp_right = tp_x + tp_w
    candidates = []
    for it in items:
        if it is body or it is touchpad:
            continue
        cx_px = it["x"] + it["w"] / 2
        cy_px = it["y"] + it["h"] / 2
        in_upper = (tp_y - 5) < cy_px < (tp_y + tp_h * 0.5)
        in_left_zone  = (tp_x - 80) < cx_px < (tp_x + 5)
        in_right_zone = (tp_right - 5) < cx_px < (tp_right + 80)
        if not in_upper:
            continue
        if not (in_left_zone or in_right_zone):
            continue
        if 15 < it["w"] < 60 and 25 < it["h"] < 80:
            candidates.append(it)

    # Drop inner twins by pairing: outer encloses inner. Keep outers only.
    candidates.sort(key=lambda c: c["bbox_area"], reverse=True)
    final = []
    used = set()
    for i, c in enumerate(candidates):
        if i in used:
            continue
        cx_i = c["x"] + c["w"] / 2
        cy_i = c["y"] + c["h"] / 2
        for j in range(i + 1, len(candidates)):
            if j in used:
                continue
            d = candidates[j]
            cx_d = d["x"] + d["w"] / 2
            cy_d = d["y"] + d["h"] / 2
            if (abs(cx_i - cx_d) < 5 and abs(cy_i - cy_d) < 5
                    and d["bbox_area"] < c["bbox_area"]):
                used.add(j)
        final.append(c)

    # Sort left → right
    final.sort(key=lambda c: c["x"])

    out = {"share": None, "options": None,
           "transform": {"scale": scale, "ox": ox, "oy": oy}}
    names = ["share", "options"]
    for name, c in zip(names, final[:2]):
        # Smooth contour to viewBox-space Q-curve path
        arc = cv2.arcLength(c["c"], True)
        eps = 0.003 * arc
        smooth = cv2.approxPolyDP(c["c"], eps, True).squeeze(1).astype(np.float64)
        pts = smooth * scale + np.array([ox, oy])
        path = _smooth_polygon_to_qpath(pts)
        out[name] = {
            "path": path,
            "bbox": {
                "x": round(tx(c["x"]), 2),
                "y": round(ty(c["y"]), 2),
                "w": round(c["w"] * scale, 2),
                "h": round(c["h"] * scale, 2),
                "cx": round(tx(c["x"] + c["w"] / 2), 2),
                "cy": round(ty(c["y"] + c["h"] / 2), 2),
            },
        }

    return out


def main():
    if len(sys.argv) != 2:
        print("usage: trace_share_options.py <png>", file=sys.stderr)
        sys.exit(1)
    src = Path(sys.argv[1])
    repo = Path(__file__).resolve().parent.parent
    out_dir = repo / "dist"
    out_dir.mkdir(parents=True, exist_ok=True)

    out = trace(src)
    (out_dir / "share_options_geom.json").write_text(json.dumps(out, indent=2))
    print("Share/Options traced:")
    for k in ("share", "options"):
        if out[k]:
            print(f"  {k}: bbox={out[k]['bbox']}")
            print(f"  {k} path: {out[k]['path'][:90]}...")
        else:
            print(f"  {k}: NOT FOUND")
    print(f"wrote {out_dir/'share_options_geom.json'}")


if __name__ == "__main__":
    main()
