#!/usr/bin/env python3
"""Draw the Eclipse logo assets from geometry, so the art can be re-tuned instead of hand-edited.

    python3 tools/eclipse-art.py            # writes crates/census-render/assets/eclipse*.txt
    python3 tools/eclipse-art.py --print    # prints the big art and its tones instead

Cells are about 2:1 tall, so one unit is one row or two columns. A lit sphere sits up-left of a dark
disc; the crescent is the sphere outside the disc, brightest where the sphere is deepest, with a short
glow bleeding into the disc. The disc is faint dots, a tilted dotted orbit with a small body crosses the
tail of the crescent, a few stars sit outside, and a faint label sits in the empty lower-left corner.
Tones: b = bold accent (the core), a = accent, d = dim, f = the terminal foreground (the orbit).
"""
import math
import random
import sys
from pathlib import Path

RAMP = " .:-=+*#%@"


def draw(cols, rows, moon, r_moon, offset, r_sun, depth_w, glow_k, orbit, stars, label, dot_grid, back=True):
    mx, my = moon
    sx, sy = mx - offset[0], my - offset[1]
    art = [[" "] * cols for _ in range(rows)]
    tone = [["d"] * cols for _ in range(rows)]
    bright = [[0.0] * cols for _ in range(rows)]

    for r in range(rows):
        for c in range(cols):
            x, y = (c + 0.5) / 2, r + 0.5
            dm, ds = math.hypot(x - mx, y - my), math.hypot(x - sx, y - sy)
            depth = max(0.0, r_sun - ds)
            b = min(1.0, depth / depth_w) ** 0.6
            if dm <= r_moon:
                b *= math.exp(-(r_moon - dm) / glow_k)
            bright[r][c] = b
            if b >= 0.06:
                art[r][c] = RAMP[min(9, int(b * 9.4))]
                tone[r][c] = "b" if b >= 0.74 else "a" if b >= 0.2 else "d"
            elif dm <= r_moon:
                gx, gy, phase = dot_grid
                if r % gy == 0 and (c + (r // gy % 2) * phase) % gx == 0:
                    art[r][c] = "."

    # Orbit: an ellipse round the disc, the lower half in front, the upper half hidden behind the disc
    # and under the bright part of the crescent. Its left end runs off the frame, like the hero's.
    cx, cy, a, b_, theta, body_t = orbit
    for i in range(4800):
        t = 2 * math.pi * i / 4800
        ex, ey = a * math.cos(t), b_ * math.sin(t)
        x = cx + ex * math.cos(theta) - ey * math.sin(theta)
        y = cy + ex * math.sin(theta) + ey * math.cos(theta)
        c, r = int(x * 2), int(y)
        if not (0 <= c < cols and 0 <= r < rows) or x < 0:
            continue
        if math.sin(t) <= 0 and (not back or math.hypot((c + 0.5) / 2 - mx, r + 0.5 - my) <= r_moon):
            continue
        if bright[r][c] >= 0.45:
            continue
        art[r][c], tone[r][c] = "·", "f"
    bx = cx + a * math.cos(body_t) * math.cos(theta) - b_ * math.sin(body_t) * math.sin(theta)
    by = cy + a * math.cos(body_t) * math.sin(theta) + b_ * math.sin(body_t) * math.cos(theta)
    c, r = int(bx * 2), int(by)
    if 0 <= c < cols and 0 <= r < rows:
        art[r][c], tone[r][c] = "o", "b"

    # Stars: a fixed scatter, only in empty sky away from everything else.
    rng = random.Random(stars[1])
    placed = 0
    for _ in range(20000):
        if placed >= stars[0]:
            break
        c, r = rng.randrange(cols), rng.randrange(int(my + r_moon) - 1)
        x, y = (c + 0.5) / 2, r + 0.5
        if math.hypot(x - mx, y - my) <= r_moon + 1.5 or math.hypot(x - sx, y - sy) <= r_sun + 2.0:
            continue
        if any(art[rr][cc] != " " for rr in range(max(0, r - 1), min(rows, r + 2)) for cc in range(max(0, c - 2), min(cols, c + 3))):
            continue
        art[r][c] = "+" if placed % 3 == 0 else "."
        placed += 1

    if label:
        text, (lc, lr) = label
        if all(art[lr][lc + i] == " " for i in range(len(text))):
            for i, ch in enumerate(text):
                art[lr][lc + i], tone[lr][lc + i] = ch, "d"

    lines = ["".join(row).rstrip() for row in art]
    tones = ["".join(row) for row in tone]
    while lines and not lines[-1]:
        lines.pop()
        tones.pop()
    while lines and not lines[0]:
        lines.pop(0)
        tones.pop(0)
    tones = [t[: len(l)] for t, l in zip(tones, lines)]
    return "\n".join(lines) + "\n", "\n".join(tones) + "\n"


def big():
    return draw(
        cols=46, rows=26,
        moon=(13.3, 12.3), r_moon=9.4, offset=(3.4, 2.4), r_sun=9.8,
        depth_w=2.8, glow_k=1.2,
        orbit=(10.4, 13.6, 13.6, 3.2, math.radians(-24), math.radians(-12)),
        stars=(6, 7), label=("T+12:04:09", (0, 20)), dot_grid=(4, 2, 2),
    )


def small():
    return draw(
        cols=22, rows=10,
        moon=(6.4, 5.2), r_moon=4.2, offset=(1.5, 1.0), r_sun=4.4,
        depth_w=1.3, glow_k=0.6,
        orbit=(5.2, 5.6, 6.4, 1.4, math.radians(-24), math.radians(30)),
        stars=(1, 3), label=None, dot_grid=(4, 2, 2), back=False,
    )


def main():
    assets = Path(__file__).resolve().parent.parent / "crates/census-render/assets"
    art, tones = big()
    sart, stones = small()
    if "--print" in sys.argv:
        print(art, end="")
        print(tones, end="")
        print(sart, end="")
        return
    (assets / "eclipse.txt").write_text(art)
    (assets / "eclipse-tones.txt").write_text(tones)
    (assets / "eclipse-small.txt").write_text(sart)
    (assets / "eclipse-small-tones.txt").write_text(stones)


main()
