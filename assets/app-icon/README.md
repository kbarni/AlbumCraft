# AlbumCraft app icon

`albumcraft-1024.png` is the source: a gecko (the AlbumCraft mascot) on a rounded green tile,
1024 × 1024 with a transparent background. It is original artwork made for this project and is
covered by the project's license.

Everything else here is resized from it with ImageMagick:

```sh
for n in 16 24 32 48 64 128 256 512; do
  convert albumcraft-1024.png -filter Lanczos -resize ${n}x${n} -strip PNG32:hicolor/${n}x${n}/apps/albumcraft.png
done
convert hicolor/{16x16,24x24,32x32,48x48,64x64,256x256}/apps/albumcraft.png albumcraft.ico
```

The app embeds the 64 px (sidebar) and 256 px (window icon) PNGs; `hicolor/` follows the
freedesktop icon-theme layout for Linux packages, `albumcraft.ico` is for Windows and the macOS
`.icns` is made from `albumcraft-1024.png` by the release workflow.
