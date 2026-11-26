#!/usr/bin/env python3
"""Regenerate figures/weighted-graphs.svg from the measured adjacency matrices.

    g++ -std=c++20 -O3 matgen.cpp -o matgen   # dumps the fixture matrices
    ./matgen 65 1 > mat65.txt && ./matgen 66 1 > mat66.txt
    python3 gen-weighted.py

matgen replays the experiment's RNG (weights 1..16, half plus 2**40) and
Algorithm 5's naming/filtering, then writes one line per edge:
`x y class gone` with class 1 = light, 2 = heavy; gone = certificate removes it.
"""
import html
from pathlib import Path

HERE = Path(__file__).resolve().parent
PITCH, CELL, X0 = 6.0, 5.6, 46.0
LIGHT, HEAVY, GONE, FG, MUTED = "#2166ac", "#d73027", "#e4e4e4", "#222222", "#666666"
WIDTH = 566


def load(path):
    head, edges = None, []
    for line in path.read_text().splitlines():
        f = line.split()
        if head is None:
            assert f[0] == "n"
            head = int(f[1])
        else:
            x, y, cls, gone = map(int, f)
            edges.append((x, y, cls, gone))
    assert len(edges) == head * (head - 1) // 2
    return head, edges


def panel(out, top, n, edges):
    gt = top + 56.0                       # grid top
    gb = gt + n * PITCH                   # grid bottom
    right = X0 + n * PITCH + CELL
    over = len(edges) > 32 * n
    removed = sum(1 for *_ , g in edges if g)
    cx = (34 + right) / 2
    t = f"{n} pivots — sampling {'on' if over else 'off'}"
    s = (f"{len(edges):,} > 32·{n} = {32*n:,}; {removed:,} certified out" if over
         else f"{len(edges):,} edges = 32·{n}; no trial drawn")
    for y, txt, size, fill in ((top + 24, t, 13, FG), (top + 38, s, 11, MUTED)):
        out.append(f'<text x="{cx:.1f}" y="{y}" text-anchor="middle" '
                   f'font-size="{size}" fill="{fill}">{html.escape(txt)}</text>')
    for c in (0, n - 1):
        out.append(f'<text x="{X0 + c*PITCH - 0.4:.1f}" y="{gb + 14:.0f}" '
                   f'text-anchor="middle" font-size="10" fill="{MUTED}">{c}</text>')
        out.append(f'<text x="34" y="{gt + c*PITCH + 9.6:.1f}" text-anchor="end" '
                   f'font-size="10" fill="{MUTED}">{c}</text>')
    for x, y, cls, gone in edges:
        fill = GONE if gone else (HEAVY if cls == 2 else LIGHT)
        for cx_, cy_ in ((x, y), (y, x)):   # symmetric: both triangles
            out.append(f'<rect x="{X0 + cx_*PITCH:.1f}" y="{gt + cy_*PITCH:.1f}" '
                       f'width="{CELL}" height="{CELL}" fill="{fill}"/>')
    return gb + 14                          # last x-label line


def legend(out, y):
    for i, (fill, label) in enumerate(((LIGHT, "weight < 2⁴⁰"),
                                       (HEAVY, "weight ≥ 2⁴⁰"),
                                       (GONE, "certified removable"))):
        x = 40 + i * 161.5
        out.append(f'<rect x="{x:.1f}" y="{y}" width="14" height="14" fill="{fill}"/>')
        out.append(f'<text x="{x + 20:.1f}" y="{y + 12}" font-size="12" '
                   f'fill="{FG}">{html.escape(label)}</text>')
    return y + 30


def main():
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {{H}}" '
           f'width="{WIDTH}" height="{{H}}" shape-rendering="crispEdges" '
           f'font-family="sans-serif">',
           f'<rect width="{WIDTH}" height="{{H}}" fill="#ffffff"/>']
    y = panel(out, 0, *load(HERE / "mat65.txt"))
    y = panel(out, y + 30, *load(HERE / "mat66.txt"))
    height = int(legend(out, y + 30))
    out.append("</svg>")
    text = "\n".join(out).replace("{H}", str(height)) + "\n"
    (HERE / "weighted-graphs.svg").write_text(text)
    print(f"weighted-graphs.svg: {WIDTH}x{height}, {len(out) - 2} elements")


if __name__ == "__main__":
    main()
