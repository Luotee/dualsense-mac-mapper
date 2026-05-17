#!/usr/bin/env python3
"""Trace ALL controller features from user PNG line art via cv2 hierarchy.

Uses RETR_TREE so each pen-line stroke (outer + inner) is paired by
parent-child relation. Identifies:
  - body outline (top-level largest)
  - L1/R1 caps (small contours between body outer and inner)
  - touchpad outer+inner
  - feature outers (parent = body inner): D-pad arms, face buttons,
    sticks outer ring, stick spokes, PS button

Outputs dist/traced_features.json + dist/traced_preview.svg.
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
STICK_SPOKE_ROTATE_DEG = 45.0


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


def trace(src_path: Path):
    img = cv2.imread(str(src_path), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise SystemExit(f"failed to read {src_path}")

    _, bw = cv2.threshold(img, 200, 255, cv2.THRESH_BINARY_INV)
    k = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    closed = cv2.morphologyEx(bw, cv2.MORPH_CLOSE, k, iterations=3)
    contours, hier = cv2.findContours(closed, cv2.RETR_TREE, cv2.CHAIN_APPROX_NONE)
    hier = hier[0] if hier is not None else []

    items = []
    for i, c in enumerate(contours):
        a = cv2.contourArea(c)
        if a < 200:
            continue
        x, y, w, h = cv2.boundingRect(c)
        items.append({
            "i": i, "c": c, "a": a,
            "x": x, "y": y, "w": w, "h": h,
            "parent": hier[i][3] if len(hier) else -1,
        })

    if not items:
        raise SystemExit("no contours")

    # Body outer = largest top-level area.
    top_level = [it for it in items if it["parent"] == -1]
    body = max(top_level, key=lambda it: it["a"])
    body_i = body["i"]
    body_inner = next(
        (it for it in items if it["parent"] == body_i),
        None,
    )
    body_inner_i = body_inner["i"] if body_inner else None

    # Compute viewBox transform from body bbox.
    bx, by, bw_px, bh_px = body["x"], body["y"], body["w"], body["h"]
    avail_w = VIEWBOX_W - 2 * MARGIN_SIDE
    avail_h = VIEWBOX_H - MARGIN_TOP - MARGIN_BOT
    scale = min(avail_w / bw_px, avail_h / bh_px)
    ox = (VIEWBOX_W - bw_px * scale) / 2 - bx * scale
    oy = MARGIN_TOP - by * scale

    def tx(p): return p * scale + ox
    def ty(p): return p * scale + oy
    def tr(v): return v * scale

    # ── Body path (smooth) ──
    arc = cv2.arcLength(body["c"], True)
    eps = 0.0004 * arc
    body_smooth = cv2.approxPolyDP(body["c"], eps, True).squeeze(1).astype(np.float64)
    body_view = body_smooth * scale + np.array([ox, oy])
    body_path = _smooth_polygon_to_qpath(body_view)

    # ── Classify ──
    # L1/R1 caps: parent = body_outer_i (between outer/inner body lines).
    # Feature outers: parent = body_inner_i.
    # Touchpad outer: top-level, NOT body.
    # Skip: body inner, feature inners, anything else.
    l1r1_caps = []
    feature_outers = []
    touchpad_outer = None

    for it in items:
        if it["i"] == body_i:
            continue
        if it["i"] == body_inner_i:
            continue
        if it["parent"] == -1:
            # Top-level non-body: touchpad (the only other top-level expected).
            if touchpad_outer is None or it["a"] > touchpad_outer["a"]:
                touchpad_outer = it
            continue
        if it["parent"] == body_i:
            l1r1_caps.append(it)
            continue
        if it["parent"] == body_inner_i:
            feature_outers.append(it)
            continue
        # parent = some other feature → it's an inner-line twin, skip.

    # ── Touchpad inner (child of touchpad outer) ──
    tp_inner = None
    if touchpad_outer is not None:
        for it in items:
            if it["parent"] == touchpad_outer["i"]:
                tp_inner = it
                break

    # ── Classify feature_outers by shape + position ──
    body_cx_px = bx + bw_px / 2
    body_cy_px = by + bh_px / 2

    face_buttons = []
    dpad_arms = []
    sticks_outer = []
    stick_spokes = []
    ps_button = None
    misc = []

    for f in feature_outers:
        w, h, a = f["w"], f["h"], f["a"]
        cx_px = f["x"] + w / 2
        cy_px = f["y"] + h / 2
        rel_x = cx_px - body_cx_px
        rel_y = cy_px - body_cy_px
        aspect = abs(w - h) / max(w, h)
        is_circle = aspect < 0.15

        # Position thresholds relative to body bbox
        right_edge_zone = bw_px * 0.22  # face button cluster / D-pad mirror
        centre_zone = bw_px * 0.1       # PS button band
        stick_band_x = bw_px * 0.4      # max |rel_x| for sticks

        # Sticks: large (r > 50px), centred around middle row
        if is_circle and w > 100 and abs(rel_x) < stick_band_x:
            r_px = (w + h) / 4
            sticks_outer.append({"cx_px": cx_px, "cy_px": cy_px, "r_px": r_px})
            continue

        # Face buttons: 4 medium circles, far right side
        if is_circle and 80 < w < 100 and rel_x > right_edge_zone:
            r_px = (w + h) / 4
            face_buttons.append({"cx_px": cx_px, "cy_px": cy_px, "r_px": r_px})
            continue

        # PS button: small circle near body centre x
        if is_circle and 60 < w < 85 and abs(rel_x) < centre_zone:
            r_px = (w + h) / 4
            ps_button = {"cx_px": cx_px, "cy_px": cy_px, "r_px": r_px}
            continue

        # D-pad arms: pentagon shapes on left side — keep full contour
        if rel_x < -right_edge_zone:
            dpad_arms.append({
                "contour": f["c"],
                "x_px": f["x"], "y_px": f["y"], "w_px": w, "h_px": h,
                "cx_px": cx_px, "cy_px": cy_px,
            })
            continue

        # Stick spokes: curved arc segments around sticks — keep full contour
        if 70 < w < 100:
            stick_spokes.append({
                "contour": f["c"],
                "x_px": f["x"], "y_px": f["y"], "w_px": w, "h_px": h,
                "cx_px": cx_px, "cy_px": cy_px,
            })
            continue

        misc.append({"cx_px": cx_px, "cy_px": cy_px, "w": w, "h": h, "a": a})

    sticks_outer.sort(key=lambda s: s["cx_px"])

    # ── L1/R1 caps to viewBox + propose L2/R2 above ──
    l1r1 = []
    for it in l1r1_caps:
        l1r1.append({
            "x": round(tx(it["x"]), 2),
            "y": round(ty(it["y"]), 2),
            "w": round(tr(it["w"]), 2),
            "h": round(tr(it["h"]), 2),
        })
    l1r1.sort(key=lambda r: r["x"])

    # L2/R2 proposal: identical shape stacked above L1/R1 with small gap.
    l2r2 = []
    gap = 1.5  # viewBox units between L1 and L2
    for cap in l1r1:
        l2 = {
            "x": cap["x"],
            "y": round(cap["y"] - cap["h"] - gap, 2),
            "w": cap["w"],
            "h": cap["h"],
        }
        l2r2.append(l2)

    # ── To viewBox helpers ──
    def circle(c):
        return {
            "cx": round(tx(c["cx_px"]), 2),
            "cy": round(ty(c["cy_px"]), 2),
            "r":  round(tr(c["r_px"]), 2),
        }

    def rect(r):
        return {
            "x": round(tx(r["x_px"]), 2),
            "y": round(ty(r["y_px"]), 2),
            "w": round(tr(r["w_px"]), 2),
            "h": round(tr(r["h_px"]), 2),
            "cx": round(tx(r["cx_px"]), 2),
            "cy": round(ty(r["cy_px"]), 2),
        }

    # Face buttons -> dict by position (triangle/circle/cross/square)
    fb_dict = None
    if len(face_buttons) == 4:
        fbs = [circle(c) for c in face_buttons]
        fbs.sort(key=lambda c: c["cy"])
        top = fbs[0]
        bot = fbs[-1]
        midsorted = sorted(fbs[1:3], key=lambda c: c["cx"])
        left, right = midsorted[0], midsorted[1]
        fb_dict = {"triangle": top, "circle": right, "cross": bot, "square": left}

    # Touchpad outer/inner — emit dense polylines straight from the raw
    # cv2 contour pixels. The 4 quad sub-paths (computed below) come
    # from the SAME polygon, so outline and fill match pixel-for-pixel
    # at the trapezoid boundary. Browser antialiasing handles smoothing.
    def smooth_to_path(c, eps_frac):  # eps_frac kept for signature compat
        raw = c.squeeze(1).astype(np.float64)
        pts = raw * scale + np.array([ox, oy])
        parts = [f"M {pts[0,0]:.2f} {pts[0,1]:.2f}"]
        for x, y in pts[1:]:
            parts.append(f"L {x:.2f} {y:.2f}")
        parts.append("Z")
        return " ".join(parts)

    tp_outer_rect = None
    tp_quad_paths = None
    if touchpad_outer is not None:
        tp_outer_rect = {
            "x": round(tx(touchpad_outer["x"]), 2),
            "y": round(ty(touchpad_outer["y"]), 2),
            "w": round(tr(touchpad_outer["w"]), 2),
            "h": round(tr(touchpad_outer["h"]), 2),
            "path": smooth_to_path(touchpad_outer["c"], 0.001),
        }
        # Compute 4 sub-paths = trapezoid ∩ quadrant region (with cross
        # gap). Each sub-path is rendered as a dense polyline (M + many
        # L commands) directly from cv2's raw contour pixels — no
        # approxPolyDP, no Q-curve smoothing. Re-smoothing the polygon
        # intersection vertices added a low-frequency wobble along the
        # outline (each new shapely-introduced vertex broke the midpoint
        # Q-curve continuity). The raw contour has ~600 vertices around
        # the touchpad perimeter, so the polyline reads smooth.
        from shapely.geometry import Polygon, box
        raw_pts = touchpad_outer["c"].squeeze(1).astype(np.float64)
        tp_view_pts = raw_pts * scale + np.array([ox, oy])
        trapezoid = Polygon(tp_view_pts)
        if not trapezoid.is_valid:
            trapezoid = trapezoid.buffer(0)
        cx_v = tp_outer_rect["x"] + tp_outer_rect["w"] / 2
        cy_v = tp_outer_rect["y"] + tp_outer_rect["h"] / 2
        gap = 1.5
        regions = {
            "tl": box(0, 0, cx_v - gap / 2, cy_v - gap / 2),
            "tr": box(cx_v + gap / 2, 0, VIEWBOX_W, cy_v - gap / 2),
            "bl": box(0, cy_v + gap / 2, cx_v - gap / 2, VIEWBOX_H),
            "br": box(cx_v + gap / 2, cy_v + gap / 2, VIEWBOX_W, VIEWBOX_H),
        }

        def polyline_path(coords):
            parts = [f"M {coords[0][0]:.2f} {coords[0][1]:.2f}"]
            for x, y in coords[1:]:
                parts.append(f"L {x:.2f} {y:.2f}")
            parts.append("Z")
            return " ".join(parts)

        tp_quad_paths = {}
        for k_, region in regions.items():
            sub = trapezoid.intersection(region)
            if sub.is_empty:
                tp_quad_paths[k_] = None
                continue
            xs, ys = sub.exterior.xy
            coords = list(zip(list(xs)[:-1], list(ys)[:-1]))
            tp_quad_paths[k_] = polyline_path(coords)

    tp_inner_rect = None
    if tp_inner is not None:
        tp_inner_rect = {
            "x": round(tx(tp_inner["x"]), 2),
            "y": round(ty(tp_inner["y"]), 2),
            "w": round(tr(tp_inner["w"]), 2),
            "h": round(tr(tp_inner["h"]), 2),
            "path": smooth_to_path(tp_inner["c"], 0.001),
        }

    # ── Build D-pad arm paths (full pentagon contours) ──
    def contour_to_path(c, eps_frac=0.001, scale_=scale, ox_=ox, oy_=oy,
                        rotate_cx=None, rotate_cy=None, rotate_deg=0):
        """Smooth a cv2 contour to a Q-curve SVG path in viewBox coords.
        Optional rotation around (rotate_cx, rotate_cy) in viewBox space."""
        arc_len = cv2.arcLength(c, True)
        eps_ = eps_frac * arc_len
        smooth = cv2.approxPolyDP(c, eps_, True).squeeze(1).astype(np.float64)
        # Transform px → viewBox
        pts = smooth * scale_ + np.array([ox_, oy_])
        if rotate_deg != 0 and rotate_cx is not None:
            cos_a = float(np.cos(np.deg2rad(rotate_deg)))
            sin_a = float(np.sin(np.deg2rad(rotate_deg)))
            dx = pts[:, 0] - rotate_cx
            dy = pts[:, 1] - rotate_cy
            new_dx = dx * cos_a - dy * sin_a
            new_dy = dx * sin_a + dy * cos_a
            pts = np.column_stack([new_dx + rotate_cx, new_dy + rotate_cy])
        return _smooth_polygon_to_qpath(pts)

    # Compute D-pad cluster centre, then label each arm by its position
    # relative to that centre.
    if dpad_arms:
        dp_cx = sum(a["cx_px"] for a in dpad_arms) / len(dpad_arms)
        dp_cy = sum(a["cy_px"] for a in dpad_arms) / len(dpad_arms)
    else:
        dp_cx = dp_cy = 0
    dpad_arm_paths_by_dir = {}
    for a in dpad_arms:
        dx_ = a["cx_px"] - dp_cx
        dy_ = a["cy_px"] - dp_cy
        if abs(dx_) > abs(dy_):
            direction = "right" if dx_ > 0 else "left"
        else:
            direction = "down" if dy_ > 0 else "up"
        dpad_arm_paths_by_dir[direction] = {
            "path": contour_to_path(a["contour"], eps_frac=0.005),
            "cx": round(tx(a["cx_px"]), 2),
            "cy": round(ty(a["cy_px"]), 2),
        }
    dpad_arm_paths = dpad_arm_paths_by_dir

    # ── Spoke paths rotated 45° around nearest stick centre, labelled
    #    with cardinal direction relative to that stick.
    spokes_rotated = []
    if len(sticks_outer) == 2 and stick_spokes:
        l_cx = tx(sticks_outer[0]["cx_px"])
        l_cy = ty(sticks_outer[0]["cy_px"])
        r_cx = tx(sticks_outer[1]["cx_px"])
        r_cy = ty(sticks_outer[1]["cy_px"])
        cos_a = float(np.cos(np.deg2rad(STICK_SPOKE_ROTATE_DEG)))
        sin_a = float(np.sin(np.deg2rad(STICK_SPOKE_ROTATE_DEG)))
        for s in stick_spokes:
            spx = tx(s["cx_px"])
            spy = ty(s["cy_px"])
            dl = (spx - l_cx) ** 2 + (spy - l_cy) ** 2
            dr = (spx - r_cx) ** 2 + (spy - r_cy) ** 2
            anchor = "left" if dl < dr else "right"
            cxv, cyv = (l_cx, l_cy) if dl < dr else (r_cx, r_cy)
            path = contour_to_path(
                s["contour"], eps_frac=0.003,
                rotate_cx=cxv, rotate_cy=cyv,
                rotate_deg=STICK_SPOKE_ROTATE_DEG,
            )
            # Direction label: rotate spoke's centre by 45° too, then
            # compare against stick centre.
            dx_pre = spx - cxv
            dy_pre = spy - cyv
            ndx = dx_pre * cos_a - dy_pre * sin_a
            ndy = dx_pre * sin_a + dy_pre * cos_a
            if abs(ndx) > abs(ndy):
                direction = "right" if ndx > 0 else "left"
            else:
                direction = "down" if ndy > 0 else "up"
            spokes_rotated.append({
                "path": path,
                "anchor": anchor,
                "anchor_cx": round(cxv, 2),
                "anchor_cy": round(cyv, 2),
                "dir": direction,
            })

    # ── Share / Options (Menu) proposal ──
    # User's drawing omits these. PS5 has them as small slanted pills
    # at the top-left / top-right of the touchpad, angled outward
    # (Share's outer end up, Options' outer end up — a soft V).
    # Render as rect with rx=h/2 (full capsule), rotated around centre.
    share_options = None
    if tp_outer_rect is not None:
        sw, sh = 6.5, 2.2
        slant = 22.0  # degrees outward
        tp_x = tp_outer_rect["x"]
        tp_y = tp_outer_rect["y"]
        tp_w = tp_outer_rect["w"]
        # Centres inside touchpad rounded-rect, near top corners
        share_cx = round(tp_x + 4.5, 2)
        share_cy = round(tp_y + 3.5, 2)
        opt_cx = round(tp_x + tp_w - 4.5, 2)
        opt_cy = round(tp_y + 3.5, 2)
        share_options = {
            "share": {
                "cx": share_cx, "cy": share_cy,
                "w": sw, "h": sh,
                "rx": round(sh / 2, 2),
                "angle_deg": -slant,  # Share: outer (left) end tilts up
            },
            "options": {
                "cx": opt_cx, "cy": opt_cy,
                "w": sw, "h": sh,
                "rx": round(sh / 2, 2),
                "angle_deg": +slant,  # Options: outer (right) end tilts up
            },
        }

    geom = {
        "viewBox": [0, 0, VIEWBOX_W, VIEWBOX_H],
        "body_path": body_path,
        "l1r1": l1r1,
        "l2r2_proposed": l2r2,
        "touchpad_outer": tp_outer_rect,
        "touchpad_inner": tp_inner_rect,
        "touchpad_quad_paths": tp_quad_paths,
        "dpad_arms_rect": [rect(a) for a in dpad_arms],
        "dpad_arms": dpad_arm_paths,
        "face_buttons": fb_dict or [circle(c) for c in face_buttons],
        "sticks_outer": [circle(c) for c in sticks_outer],
        "stick_spokes_raw": [rect(s) for s in stick_spokes],
        "stick_spokes_rotated": spokes_rotated,
        "ps_button": circle(ps_button) if ps_button else None,
        "share_options": share_options,
        "misc": misc,
        "transform": {"scale": scale, "ox": ox, "oy": oy},
    }
    return geom


def render_preview_svg(geom, out_path: Path):
    body = geom["body_path"]
    parts = [
        '<?xml version="1.0"?>',
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 130">',
        '<rect width="100%" height="100%" fill="#282828"/>',
    ]
    # L2/R2 first (so body draws over their lower edge)
    for cap in geom["l2r2_proposed"]:
        parts.append(
            f'<rect x="{cap["x"]}" y="{cap["y"]}" '
            f'width="{cap["w"]}" height="{cap["h"]}" rx="2" '
            f'fill="#3c3836" stroke="#ebdbb2" stroke-width="0.5"/>'
        )
    parts.append(
        f'<path d="{body}" fill="#3c3836" stroke="#ebdbb2" stroke-width="0.6"/>'
    )
    for cap in geom["l1r1"]:
        parts.append(
            f'<rect x="{cap["x"]}" y="{cap["y"]}" '
            f'width="{cap["w"]}" height="{cap["h"]}" rx="2" '
            f'fill="#3c3836" stroke="#ebdbb2" stroke-width="0.5"/>'
        )
    tp = geom["touchpad_outer"]
    if tp:
        parts.append(
            f'<rect x="{tp["x"]}" y="{tp["y"]}" '
            f'width="{tp["w"]}" height="{tp["h"]}" rx="4" '
            f'fill="#504945" stroke="#ebdbb2" stroke-width="0.4"/>'
        )
    tpi = geom["touchpad_inner"]
    if tpi:
        parts.append(
            f'<rect x="{tpi["x"]}" y="{tpi["y"]}" '
            f'width="{tpi["w"]}" height="{tpi["h"]}" rx="3" '
            f'fill="none" stroke="#ebdbb2" stroke-width="0.3"/>'
        )
    dpad = geom["dpad_arms"]
    dpad_iter = dpad.values() if isinstance(dpad, dict) else dpad
    for arm in dpad_iter:
        parts.append(
            f'<path d="{arm["path"]}" fill="none" '
            f'stroke="#a89984" stroke-width="0.5"/>'
        )
    fb = geom["face_buttons"]
    fb_iter = fb.values() if isinstance(fb, dict) else fb
    for c in fb_iter:
        parts.append(
            f'<circle cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}" '
            f'fill="none" stroke="#a89984" stroke-width="0.5"/>'
        )
    for c in geom["sticks_outer"]:
        parts.append(
            f'<circle cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}" '
            f'fill="none" stroke="#a89984" stroke-width="0.5"/>'
        )
    for s in geom["stick_spokes_rotated"]:
        parts.append(
            f'<path d="{s["path"]}" fill="none" '
            f'stroke="#a89984" stroke-width="0.45"/>'
        )
    if geom["ps_button"]:
        c = geom["ps_button"]
        parts.append(
            f'<circle cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}" '
            f'fill="none" stroke="#a89984" stroke-width="0.5"/>'
        )
    so = geom.get("share_options")
    if so:
        for k in ("share", "options"):
            r = so[k]
            cx, cy = r["cx"], r["cy"]
            x = cx - r["w"] / 2
            y = cy - r["h"] / 2
            parts.append(
                f'<rect x="{x:.2f}" y="{y:.2f}" '
                f'width="{r["w"]}" height="{r["h"]}" rx="{r["rx"]}" '
                f'transform="rotate({r["angle_deg"]} {cx} {cy})" '
                f'fill="none" stroke="#a89984" stroke-width="0.4"/>'
            )
    parts.append('</svg>')
    out_path.write_text("\n".join(parts))


def main():
    if len(sys.argv) != 2:
        print("usage: trace_all.py <source.png>", file=sys.stderr)
        sys.exit(1)
    src = Path(sys.argv[1])
    if not src.exists():
        print(f"no such file: {src}", file=sys.stderr)
        sys.exit(1)

    repo = Path(__file__).resolve().parent.parent
    out_dir = repo / "dist"
    out_dir.mkdir(parents=True, exist_ok=True)

    geom = trace(src)

    (out_dir / "traced_features.json").write_text(json.dumps(geom, indent=2))
    (out_dir / "traced_body_path.txt").write_text(geom["body_path"] + "\n")
    render_preview_svg(geom, out_dir / "traced_preview.svg")

    print("Traced features summary:")
    print(f"  body_path chars: {len(geom['body_path'])}")
    print(f"  L1/R1 caps: {len(geom['l1r1'])}")
    for i, c in enumerate(geom["l1r1"]):
        print(f"    [{i}] {c}")
    print(f"  L2/R2 proposed: {len(geom['l2r2_proposed'])}")
    for i, c in enumerate(geom["l2r2_proposed"]):
        print(f"    [{i}] {c}")
    print(f"  touchpad_outer: {geom['touchpad_outer']}")
    print(f"  touchpad_inner: {geom['touchpad_inner']}")
    print(f"  dpad_arms: {len(geom['dpad_arms'])}")
    for a in geom["dpad_arms"]:
        print(f"    {a}")
    print(f"  face_buttons: "
          f"{list(geom['face_buttons'].keys()) if isinstance(geom['face_buttons'], dict) else len(geom['face_buttons'])}")
    if isinstance(geom["face_buttons"], dict):
        for k, v in geom["face_buttons"].items():
            print(f"    {k}: {v}")
    print(f"  sticks_outer: {len(geom['sticks_outer'])}")
    for c in geom["sticks_outer"]:
        print(f"    {c}")
    print(f"  stick_spokes_raw: {len(geom['stick_spokes_raw'])}")
    print(f"  stick_spokes_rotated: {len(geom['stick_spokes_rotated'])}")
    print(f"  ps_button: {geom['ps_button']}")
    print(f"  misc: {len(geom['misc'])}")
    print(f"wrote {out_dir/'traced_features.json'}")
    print(f"wrote {out_dir/'traced_preview.svg'}")


if __name__ == "__main__":
    main()
