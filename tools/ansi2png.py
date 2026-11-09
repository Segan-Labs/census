#!/usr/bin/env python3
"""Render ANSI text (stdin) to a PNG: a headless terminal for README and design screenshots.

    census --color always --width 100 --height 28 | python3 tools/ansi2png.py out.png [cols] [rows]

Handles the SGR codes census prints (reset, bold, dim, the 8 + 8 palette colours, default colours).
"""
import re
import sys
from PIL import Image, ImageDraw, ImageFont

FONT = "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Regular.ttf"
FONT_BOLD = "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Bold.ttf"
SIZE = 18
PAD = 28
BG = (14, 14, 18)
FG = (222, 222, 228)
# A common dark terminal palette; census only asks for the terminal's own colours, so any theme works.
PALETTE = [
    (30, 30, 36), (224, 108, 117), (152, 195, 121), (229, 192, 123),
    (97, 175, 239), (179, 146, 240), (86, 182, 194), (200, 200, 208),
    (92, 99, 112), (240, 128, 137), (172, 215, 141), (240, 210, 143),
    (117, 195, 255), (199, 166, 255), (106, 202, 214), (255, 255, 255),
]


def mix(a, b, t):
    return tuple(round(a[i] * (1 - t) + b[i] * t) for i in range(3))


def parse(text):
    """Yield rows of (char, fg or None, bold, dim)."""
    fg, bold, dim = None, False, False
    rows = []
    for line in text.split("\n"):
        row = []
        pos = 0
        for m in re.finditer(r"\x1b\[([0-9;]*)m", line):
            for ch in line[pos:m.start()]:
                row.append((ch, fg, bold, dim))
            pos = m.end()
            for code in (m.group(1) or "0").split(";"):
                c = int(code or 0)
                if c == 0:
                    fg, bold, dim = None, False, False
                elif c == 1:
                    bold = True
                elif c == 2:
                    dim = True
                elif c == 22:
                    bold = dim = False
                elif 30 <= c <= 37:
                    fg = PALETTE[c - 30]
                elif 90 <= c <= 97:
                    fg = PALETTE[c - 90 + 8]
                elif c == 39:
                    fg = None
        for ch in line[pos:]:
            row.append((ch, fg, bold, dim))
        rows.append(row)
    while rows and not rows[-1]:
        rows.pop()
    return rows


def main():
    out = sys.argv[1]
    rows = parse(sys.stdin.read())
    cols = int(sys.argv[2]) if len(sys.argv) > 2 else max(len(r) for r in rows)
    nrows = int(sys.argv[3]) if len(sys.argv) > 3 else len(rows)
    font, bold_font = ImageFont.truetype(FONT, SIZE), ImageFont.truetype(FONT_BOLD, SIZE)
    cw = round(font.getlength("M"))
    ch = round(SIZE * 1.45)
    img = Image.new("RGB", (cols * cw + PAD * 2, nrows * ch + PAD * 2), BG)
    d = ImageDraw.Draw(img)
    for y, row in enumerate(rows[:nrows]):
        for x, (c, fg, bold, dim) in enumerate(row[:cols]):
            if c == " ":
                continue
            color = fg or FG
            if dim:
                color = mix(color, BG, 0.5)
            d.text((PAD + x * cw, PAD + y * ch), c, font=bold_font if bold else font, fill=color)
    img.save(out)


main()
