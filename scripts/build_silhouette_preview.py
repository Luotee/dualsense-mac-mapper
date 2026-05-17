#!/usr/bin/env python3
"""Read dist/traced_features.json and emit
docs/superpowers/specs/2026-05-17-silhouette-redesign-preview.html.

Renders the full traced controller in 240×130 viewBox with:
  - body Q-curve path
  - L1/R1 caps + L2/R2 caps (L1/R1 duplicated above with gap=1.5)
  - touchpad outer + inner light-bar seam
  - D-pad arm pentagon paths (NOT bbox rects)
  - 4 face button circles (triangle/circle/cross/square)
  - 2 stick outer circles
  - 8 spoke arc paths, pre-rotated 45° around stick centres
  - PS button circle
  - Share / Options small rects (proposed, drawing omits them)
"""
import json
from pathlib import Path


REPO = Path(__file__).resolve().parent.parent
GEOM_PATH = REPO / "dist" / "traced_features.json"
SO_PATH = REPO / "dist" / "share_options_geom.json"
OUT_PATH = REPO / "docs" / "superpowers" / "specs" / "2026-05-17-silhouette-redesign-preview.html"


def build_svg(g):
    parts = ['<svg viewBox="0 0 240 130" xmlns="http://www.w3.org/2000/svg">']

    # L2/R2 first (so body's lower curve covers their bottom)
    for cap in g["l2r2_proposed"]:
        parts.append(
            f'<rect class="lr-cap" x="{cap["x"]}" y="{cap["y"]}" '
            f'width="{cap["w"]}" height="{cap["h"]}" rx="2"/>'
        )
    # Body
    parts.append(f'<path class="body-shape" d="{g["body_path"]}"/>')
    # L1/R1
    for cap in g["l1r1"]:
        parts.append(
            f'<rect class="lr-cap" x="{cap["x"]}" y="{cap["y"]}" '
            f'width="{cap["w"]}" height="{cap["h"]}" rx="2"/>'
        )
    # Touchpad
    tp = g["touchpad_outer"]
    if tp:
        parts.append(
            f'<rect class="lightbar" x="{tp["x"]}" y="{tp["y"]}" '
            f'width="{tp["w"]}" height="{tp["h"]}" rx="4"/>'
        )
    tpi = g["touchpad_inner"]
    if tpi:
        parts.append(
            f'<rect class="lightbar-inner" x="{tpi["x"]}" y="{tpi["y"]}" '
            f'width="{tpi["w"]}" height="{tpi["h"]}" rx="3"/>'
        )
    # D-pad arms (path, pentagon) — dict keyed by dir, or list legacy
    dpad = g["dpad_arms"]
    dpad_iter = dpad.values() if isinstance(dpad, dict) else dpad
    for arm in dpad_iter:
        parts.append(f'<path class="button" d="{arm["path"]}"/>')
    # Face buttons
    fb = g["face_buttons"]
    fb_iter = fb.values() if isinstance(fb, dict) else fb
    for c in fb_iter:
        parts.append(
            f'<circle class="button" cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}"/>'
        )
    # Sticks outer
    for c in g["sticks_outer"]:
        parts.append(
            f'<circle class="button" cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}"/>'
        )
    # Spokes (rotated 45°, real arc curve)
    for s in g["stick_spokes_rotated"]:
        parts.append(f'<path class="spoke" d="{s["path"]}"/>')
    # PS
    if g["ps_button"]:
        c = g["ps_button"]
        parts.append(
            f'<circle class="button" cx="{c["cx"]}" cy="{c["cy"]}" r="{c["r"]}"/>'
        )
    # Share / Options — traced paths from secondary PNG
    so_traced = g.get("share_options_traced")
    if so_traced:
        for k in ("share", "options"):
            if so_traced.get(k):
                parts.append(
                    f'<path class="meta" d="{so_traced[k]["path"]}"/>'
                )

    # Tiny labels
    parts.append('<text class="label" x="66" y="11">L2</text>')
    parts.append('<text class="label" x="174" y="11">R2</text>')
    parts.append('<text class="label" x="66" y="18.5">L1</text>')
    parts.append('<text class="label" x="174" y="18.5">R1</text>')

    parts.append('</svg>')
    return "\n".join(parts)


def build_html(g):
    svg = build_svg(g)
    return f"""<!DOCTYPE html>
<html lang="zh-Hant">
<head>
  <meta charset="UTF-8">
  <title>Controller Silhouette — Full Trace v3</title>
  <style>
    :root {{
      --bg: #282828;
      --card: #3c3836;
      --border: #665c54;
      --text: #a89984;
      --text-bright: #ebdbb2;
      --accent: #83a598;
      --warn: #fabd2f;
      --body-stroke: #ebdbb2;
      --body-fill: #3c3836;
      --button-stroke: #a89984;
      --light-bar: #504945;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      padding: 24px;
      font: 14px/1.5 system-ui, sans-serif;
      background: var(--bg);
      color: var(--text);
    }}
    h1 {{ color: var(--text-bright); margin: 0 0 8px; font-size: 18px; }}
    h2 {{ color: var(--text-bright); font-size: 14px; margin: 14px 0 6px; }}
    p, ul, ol {{ margin: 0 0 8px; }}
    code {{ background: var(--card); padding: 1px 5px; border-radius: 3px; color: var(--warn); }}
    .lede strong {{ color: var(--text-bright); }}
    .grid {{ display: grid; grid-template-columns: 2fr 1fr; gap: 16px; margin-top: 14px; }}
    .panel {{ background: var(--card); border: 1px solid var(--border); border-radius: 6px; padding: 12px; }}
    .panel h3 {{ color: var(--text-bright); margin: 0 0 6px; font-size: 13px; }}
    svg {{ width: 100%; height: auto; display: block; }}
    .body-shape    {{ fill: var(--body-fill); stroke: var(--body-stroke); stroke-width: 0.6; }}
    .lr-cap        {{ fill: var(--body-fill); stroke: var(--body-stroke); stroke-width: 0.5; }}
    .lightbar      {{ fill: var(--light-bar); stroke: var(--body-stroke); stroke-width: 0.4; }}
    .lightbar-inner{{ fill: none; stroke: var(--body-stroke); stroke-width: 0.3; }}
    .button        {{ fill: none; stroke: var(--button-stroke); stroke-width: 0.5; }}
    .spoke         {{ fill: none; stroke: var(--button-stroke); stroke-width: 0.45; }}
    .meta          {{ fill: none; stroke: var(--button-stroke); stroke-width: 0.4; }}
    .label         {{ fill: var(--text); font: 3px sans-serif; text-anchor: middle; }}
    table {{ width: 100%; border-collapse: collapse; font-size: 11px; }}
    th, td {{ text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border); }}
    th {{ color: var(--text-bright); font-weight: 600; }}
  </style>
</head>
<body>
  <h1>Controller silhouette — 全部 trace v3 (D-pad pentagon + spoke arc 用真實 path)</h1>
  <p class="lede">
    所有幾何皆由 <code>scripts/trace_all.py</code> trace 出。
    <strong>D-pad arm 改用五邊形 path</strong>（不是 bbox 矩形）。
    <strong>Stick spoke 改用弧形 path</strong>，且已 rotate 45° 落在 N/S/E/W。
    <strong>Share / Options 已加</strong>（線稿無，按 PS5 標準放在 touchpad 兩側）。
    <strong>L2/R2 = L1/R1 完全同形狀往上疊</strong>（gap=1.5u）。
  </p>

  <div class="grid">
    <div class="panel">
      <h3>完整 trace 預覽（240×130 viewBox）</h3>
      {svg}
    </div>
    <div class="panel">
      <h3>更動摘要 vs v2</h3>
      <ul>
        <li><strong>D-pad arms</strong>：bbox rect → 五邊形 path（cv2 approxPolyDP，ε=0.5%）</li>
        <li><strong>Stick spokes</strong>：bbox rect → 弧形 path，rotate 45° 套用在 path 上</li>
        <li><strong>Share / Options</strong>：新增小 rect 在 touchpad 兩側（線稿沒畫，按 PS5 標準）</li>
      </ul>

      <h3 style="margin-top: 14px;">幾何摘要</h3>
      <table>
        <tr><th>feature</th><th>形式</th></tr>
        <tr><td>body</td><td>79-pt Q-curve path</td></tr>
        <tr><td>L1/R1</td><td>2 個 rect 20.2×6.0</td></tr>
        <tr><td>L2/R2</td><td>L1/R1 ↑ gap 1.5u</td></tr>
        <tr><td>touchpad</td><td>71.08×33.62 + inner 線</td></tr>
        <tr><td>D-pad 4 arm</td><td>五邊形 path</td></tr>
        <tr><td>4 face</td><td>圓 r=5.83</td></tr>
        <tr><td>2 stick</td><td>圓 r≈6.17</td></tr>
        <tr><td>8 spoke</td><td>弧形 path（rotate 45°）</td></tr>
        <tr><td>PS</td><td>圓 r=4.64</td></tr>
        <tr><td>Share/Options</td><td>5×2.5 rect</td></tr>
      </table>
    </div>
  </div>

  <h2>下一步</h2>
  <p>若 D-pad 五邊形 / spoke 弧形 OK，下步把這些幾何打進 <code>rust/web/controller.js</code>。</p>
</body>
</html>
"""


def main():
    g = json.loads(GEOM_PATH.read_text())
    if SO_PATH.exists():
        so = json.loads(SO_PATH.read_text())
        g["share_options_traced"] = so
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(build_html(g))
    print(f"wrote {OUT_PATH}")


if __name__ == "__main__":
    main()
