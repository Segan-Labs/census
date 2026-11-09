#!/usr/bin/env python3
"""Turn the Eclipse hero art (~/Pictures/Seganism/seganism-banner-1920x1080.png, read-only)
into the text logos in crates/census-render/assets/.

    python3 tools/eclipse_to_ascii.py SRC.png crates/census-render/assets/eclipse.txt        # 28 rows of art, ~19 after trimming
    python3 tools/eclipse_to_ascii.py SRC.png crates/census-render/assets/eclipse-small.txt small
"""
import sys
from PIL import Image

RAMP = " .:-=+*#%@"


def big(src, rows=28):
    img = Image.open(src).convert("L")
    # The crescent and the start of its orbit: x 690..1280, y 120..890 of the 1920x1080 banner.
    img = img.crop((690, 120, 1280, 890))
    cols = round(img.width / img.height * rows * 2.0 * 1.15)
    img = img.resize((cols, rows), Image.BOX)
    lines = []
    for y in range(rows):
        row = ""
        for x in range(cols):
            v = img.getpixel((x, y)) / 255
            v = min(1.0, max(0.0, v - 0.09) * 4.0)  # the art is dim on near-black: lift it, drop the star dust
            row += RAMP[int(v * (len(RAMP) - 1) + 0.5)]
        lines.append(row.rstrip())
    return lines


def pool(lines):
    """Half the size: each 2x2 block becomes its densest character."""
    width = max(len(l) for l in lines)
    lines = [l.ljust(width) for l in lines] + [" " * width] * (len(lines) % 2)
    out = []
    for y in range(0, len(lines), 2):
        row = ""
        for x in range(0, width, 2):
            block = (lines[y] + " ")[x:x + 2] + (lines[y + 1] + " ")[x:x + 2]
            row += RAMP[max(RAMP.index(c) for c in block)]
        out.append(row.rstrip())
    return out


def trim(lines):
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip(" .:"):  # a faint tail of dots goes too
        lines.pop()
    indent = min(len(l) - len(l.lstrip()) for l in lines if l.strip())
    return [l[indent:] for l in lines]


src, out = sys.argv[1], sys.argv[2]
lines = trim(big(src))
if len(sys.argv) > 3 and sys.argv[3] == "small":
    lines = trim(pool(lines))
open(out, "w").write("\n".join(lines) + "\n")
