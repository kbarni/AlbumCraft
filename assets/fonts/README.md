# Icon font

`MaterialSymbolsOutlined-subset.ttf` holds only the icons listed in `src/icons.rs`, cut from
Google's Material Symbols Outlined variable font (weight 400, no fill, grade 0, optical size 24).
License: Apache 2.0 (`LICENSE-MaterialSymbols`).

To add an icon:

1. Find it at <https://fonts.google.com/icons> (Material Symbols, Outlined).
2. Add a line to `src/icons.rs` with its name and codepoint.
3. Download the full font and its codepoint list from
   <https://github.com/google/material-design-icons/tree/master/variablefont>
   (`MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf` and `.codepoints`), then run, from the
   repository root (needs `pip install fonttools`):

   ```sh
   python3 assets/fonts/subset_icons.py <font.ttf> <font.codepoints>
   ```

   The script checks every codepoint against the official list. `cargo test glyph` checks that
   each icon is in the font.
