# AlbumCraft app icon

`albumcraft.svg` is the source: a gecko (the AlbumCraft mascot) drawn by hand as SVG for this
project, in the style of the other *Craft* app icons (rounded tile, bright background, cream
animal with black linework). It is original artwork, not derived from any other icon, and is
covered by the project's license.

Everything else here is exported from it with Inkscape:

```sh
for n in 16 24 32 48 64 128 256 512; do
  inkscape albumcraft.svg --export-type=png --export-filename=hicolor/${n}x${n}/apps/albumcraft.png -w $n -h $n
done
cp albumcraft.svg hicolor/scalable/apps/albumcraft.svg
inkscape albumcraft.svg --export-type=png --export-filename=albumcraft-1024.png -w 1024 -h 1024
convert hicolor/{16x16,24x24,32x32,48x48,64x64,256x256}/apps/albumcraft.png albumcraft.ico
```

The app embeds the 64 px (sidebar) and 256 px (window icon) PNGs; `hicolor/` follows the
freedesktop icon-theme layout for Linux packages, `albumcraft.ico` is for Windows.
