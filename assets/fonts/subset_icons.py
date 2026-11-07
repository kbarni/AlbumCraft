#!/usr/bin/env python3
"""Build assets/fonts/MaterialSymbolsOutlined-subset.ttf: only the icons listed in src/icons.rs.

The full font and its codepoint list come from
https://github.com/google/material-design-icons/tree/master/variablefont
("MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf" and ".codepoints"; Apache 2.0).

    python3 assets/fonts/subset_icons.py <full font .ttf> <.codepoints file>

Needs fontTools (pip install fonttools). Run from the repository root. The variable font is
fixed at weight 400, no fill, grade 0, optical size 24, then cut down to the listed icons.
"""

import re
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/fonts/MaterialSymbolsOutlined-subset.ttf"


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    font_path, codepoints_path = sys.argv[1], sys.argv[2]
    official = {}
    for line in Path(codepoints_path).read_text().splitlines():
        if line.strip():
            name, cp = line.split()
            official.setdefault(name, int(cp, 16))

    src = (ROOT / "src/icons.rs").read_text()
    wanted = re.findall(r'^\s*\w+ = "(\w+)" "\\u\{([0-9a-fA-F]+)\}";', src, re.M)
    if not wanted:
        sys.exit("no icons found in src/icons.rs")
    bad = [f"{n}: {cp} (official: {official.get(n, 'unknown name')})"
           for n, cp in wanted if official.get(n) != int(cp, 16)]
    if bad:
        sys.exit("wrong codepoints in src/icons.rs:\n  " + "\n  ".join(bad))

    font = TTFont(font_path)
    font = instancer.instantiateVariableFont(font, {"FILL": 0, "GRAD": 0, "opsz": 24, "wght": 400})
    options = subset.Options()
    options.layout_features = []  # no ligatures: icons are used by codepoint
    options.name_IDs = ["*"]
    options.notdef_outline = True
    sub = subset.Subsetter(options)
    sub.populate(unicodes=[int(cp, 16) for _, cp in wanted])
    sub.subset(font)
    font.save(OUT)
    print(f"{len(wanted)} icons -> {OUT.relative_to(ROOT)} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
