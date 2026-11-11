# AlbumCraft interface text, English (the reference: every other language has the same keys).
# Syntax: https://projectfluent.org/fluent/guide/  Plurals: { $n -> [one] … *[other] … }

# The language's own name, as shown in Settings > Interface > Language.
language-name = English

## Shared buttons and words

button-ok = OK
button-cancel = Cancel
button-close = Close
button-remove = Remove
button-save = Save
add-folder = Add folder…
scan-again = Scan again
removed-root = Removed { $path } from the library. The pictures and your notes on them are untouched.

## Settings dialog

settings-title = Settings
settings-saved-note = Changes are saved as you make them.

settings-section-interface = Interface
settings-section-folders = Folders
settings-section-editing = Editing
settings-section-slideshow = Slideshow
settings-section-export = Export
settings-section-maps = Maps
settings-section-storage = Storage
settings-section-ai = AI features

## Settings > Interface

settings-language = Language
settings-language-system = Same as system ({ $language })
settings-language-note = Other languages can be added as translation files; see i18n/ in the source code.
settings-theme = Theme
theme-system = Same as system
theme-light = Light
theme-dark = Dark

## Dialogs (src/dialogs.rs)

dialog-new-album = New album
dialog-rename-album = Rename album
dialog-tags = Tags and people
dialog-gallery = Web gallery
dialog-folder-note = Folder description
dialog-folder-manager = Library folders
dialog-movie = Movie
dialog-about = About AlbumCraft
dialog-online-maps = Maps
dialog-print-review = Review
dialog-trash = Move to the trash

new-album-name = Album name:
new-album-adds = { $n ->
    [one] The selected picture will be added.
   *[other] The { $n } selected pictures will be added.
}
new-album-create = Create
rename-album-rename = Rename

tags-intro = { $n ->
    [one] Add to the selected picture. Separate words with commas.
   *[other] Add to the { $n } selected pictures. Separate words with commas.
}
tags-tags = Tags:
tags-people = People:
tags-add = Add

gallery-intro = { $n ->
    [one] A web page with one picture and your edits, viewable in any browser or uploadable to a web host.
   *[other] A web page with { $n } pictures and your edits, viewable in any browser or uploadable to a web host.
}
gallery-title = Title:
gallery-style = Style:
gallery-size = Picture size:
gallery-size-medium = Medium (1280 pixels, small files)
gallery-size-large = Large (2048 pixels, for big screens)
gallery-captions = Show captions
gallery-make = Choose folder and make…

folder-note-intro = A short description shown under the folder name:

folders-intro = The library shows the pictures in these folders and their subfolders, where they are: nothing is copied or moved.
folders-import-hint = To copy pictures from a memory card or a camera, use Import.
folders-none = (none yet — sample pictures are shown instead)

# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] A video of one picture as in the slideshow: { $seconds } s
   *[other] A video of { $n } pictures as in the slideshow: { $seconds } s each
}{ $fade ->
    [yes] , crossfades
   *[no] {""}
}{ $motion ->
    [yes] , pan and zoom
   *[no] {""}
}.
movie-length = About { $seconds } s long. Change the timing and effects in the slideshow's control bar or in Settings.
movie-music = Music
movie-other-music = Other music…
movie-add-music = Add music…
movie-no-ffmpeg = Making movies needs the free FFmpeg program, which wasn't found.
movie-install-ffmpeg = Install it from ffmpeg.org (or your system's package manager), then open this window again.
movie-make = Choose file and make…
movie-preview = Preview as slideshow

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] Move the rejected picture ({ $size }) to the trash?
   *[other] Move the { $count } rejected pictures ({ $size }) to the trash?
}
trash-explain = The files are removed from their folders. You can get them back from the system's trash until it is emptied.
trash-move = Move to the trash

print-review-remove = Remove These Pictures
print-review-print = Print Anyway

about-text = A desktop photo organizer: browse your picture folders, make albums, pick, reject and rate pictures, fix and touch up photos, and play slideshows.
about-edits = Edits never change your original files; they are applied when you look at or export a picture.
about-credits = Built-in map: Natural Earth (public domain). City list: GeoNames (CC BY 4.0). Online maps: © OpenStreetMap contributors (ODbL).

online-maps-question = Show detailed maps from the internet?
online-maps-what = AlbumCraft can show where your pictures were taken. With online maps, it downloads street maps from OpenStreetMap and can look up addresses when you set a location.
online-maps-privacy = Your pictures and their locations are never sent. The map servers see your internet address, the areas you look at and the places you search for.
online-maps-offline = Without online maps, AlbumCraft uses its built-in world map and list of cities, and makes no internet connections.
online-maps-later = You can change this later on the map, or in Settings (main menu): Maps.
online-maps-use = Use online maps
online-maps-builtin = Built-in map only

## Notices at the bottom of the window

notice-undo = Undo
notice-show = Show in folder
notice-close = Close this message
notice-no-file-manager = Could not open the file manager: { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } picture
   *[other] { $n } pictures
}

settings-bar = Bar at the bottom of the library
settings-bar-compact = Compact: one line, shown while pictures are selected
settings-bar-tray = Tray: thumbnails of the selected pictures and large buttons
settings-bar-note = In the compact bar, the buttons lose their names when the window is narrow. When the buttons don't fit, the bar scrolls sideways.
settings-thumb-size = Picture size in the library

settings-folders-intro = The library shows the pictures in these folders and their subfolders, where they are: nothing is copied or moved. To copy pictures from a memory card or a camera, use Import.
settings-folders-none = No folders yet: sample pictures are shown instead.
settings-folders-remove-hint = Stop showing this folder (its pictures and your notes on them are kept)
settings-folders-rescan-hint = Look for new, changed and removed pictures now

settings-edit-opens = The Edit tab opens on
settings-edit-basic = Basic: one-click fixes and tools
settings-edit-advanced = Advanced: the sliders
settings-info-panel = Info panel
settings-info-show = Show the info panel in the editor (I)

settings-timing = Timing
settings-next-after = Next picture after
settings-note-paused = Space pauses a slideshow, ← and → step through it.
settings-note-manual = Each picture stays until you move on with → or the buttons. The full-screen viewer (F11) still moves on by itself.
settings-effects = Effects
settings-fade = Crossfade between pictures
settings-motion = Slow pan and zoom
settings-motion-subtle = Subtle
settings-motion-medium = Medium
settings-captions = Show captions
settings-viewer-note = The full-screen viewer (F11 in the editor) always starts without crossfade and pan and zoom.
settings-music = Music
settings-music-play = Play music during slideshows
settings-music-another = Choose another…
settings-music-choose = Choose music…
settings-movies-note = Movies made from pictures use these settings too.

settings-export-note = What the Export view starts with.
settings-export-size = Picture size
settings-export-quality = JPEG quality
settings-export-info = Information in the copies
settings-export-gps = Include where the pictures were taken (GPS)
settings-export-gps-hint = Turn off before sharing pictures if you don't want others to see the location.
settings-export-exif = Include the camera information (EXIF)
settings-export-exif-hint = Camera, lens, exposure and the date taken, copied from the original file. The location follows the setting above.
settings-creator = Creator
settings-creator-hint = Your name
settings-copyright = Copyright
settings-copyright-hint = © Your Name
settings-credit-note = Written into each copy (EXIF), in place of the original's. Emailed copies follow these settings too. The signature or logo is chosen in the Export view.

settings-maps-online = Online maps
settings-maps-use = Use online maps (OpenStreetMap)
settings-maps-note = Detailed street maps and address search. Your pictures and their locations are never sent; the map servers see your internet address, the areas you look at and the places you search for. Off: only the built-in world map and city list, and no internet connections.
settings-maps-tiles = Downloaded map tiles
settings-maps-tiles-after = Map tiles are downloaded again when needed.

settings-notes = Your notes
settings-notes-note = Ratings, captions, tags, edits and albums are kept in one file; your picture files are never changed.
settings-thumbs = Thumbnails
settings-thumbs-after = Thumbnails are made again from the pictures when needed (slower for a while).
cache-on-disk = { $size } on disk
cache-measuring = Measuring…
cache-empty = Empty

settings-ai-soon = Not available yet. Planned:
settings-ai-faces = Faces
settings-ai-faces-text = find faces and group them by person; you name the groups. A download of about 10 to 40 MB.
settings-ai-search = Smart search
settings-ai-search-text = find pictures by describing them ("snow", "birthday cake"). Light (English, about 100 MB) or Multilingual (about 400 MB).
settings-ai-tags = Tag suggestions
settings-ai-tags-text = tags proposed from what is in the picture.
settings-ai-local = Everything will be computed on this computer; nothing is sent anywhere.

export-size-original = Original size
export-size-very-large = Very large (2560 pixels)
export-size-large = Large (1600 pixels)
export-size-small = Small, for email (800 pixels)

## Export view (src/export_view.rs)

filter-pictures = Pictures
view-back-editor = Editor
view-back-library = Library
view-back-hint = Go back (Esc)

export-title = Export
export-count-all = { $n ->
    [one] { $n } picture
   *[other] { $n } pictures
}
export-count-some = { $n } of { $total } pictures
export-select-first = Select some pictures first.
export-logo-unreadable = The logo { $path } can't be read.
export-task = { $n ->
    [one] Exporting { $n } picture
   *[other] Exporting { $n } pictures
}
export-save-into = Save into
export-choose-folder = Choose a folder…
export-choose-folder-hint = Choose the folder for the copies
export-safe = Nothing is overwritten; the originals are not changed.
export-go = { $n ->
    [one] Export { $n } picture
   *[other] Export { $n } pictures
}
export-dialog-title = Export pictures to…

export-size = Size
export-size-other = Other:
export-size-pixels = { " " }pixels
export-size-note = The longest side. Pictures are never made larger than they are.
export-quality = JPEG quality

export-signature = Signature
export-signature-on = Add a signature or a logo
export-sig-text = Text
export-sig-logo = Picture (logo)
export-logo-other = Other picture…
export-logo-choose = Choose a picture…
export-logo-dialog = Logo or signature picture
export-logo-note = A PNG with a transparent background works best.
export-sig-text-hint = Your name, or © Your Name
export-sig-position = Position
export-sig-size = Size
export-sig-size-hint = Compared with the picture's shorter side
export-sig-opacity = Opacity
export-sig-colour = Colour
export-sig-shadow = Shadow
export-sig-shadow-hint = A soft halo so the text reads on any picture

export-info = Information in the files
export-gps = Where the pictures were taken (GPS)
export-gps-hint = Turn off before sharing pictures if you don't want others to see the location.
export-exif = Camera information (EXIF)
export-exif-hint = Camera, lens, exposure and the date taken, copied from the original file.
export-creator = Creator
export-creator-hint = Your name
export-copyright = Copyright
export-copyright-hint = © Your Name
export-credit-note = Written into each copy. The signature, creator and copyright are kept for next time (also in Settings, Export).

export-left-out = Left out: click to export it too
export-leave-out = Click to leave this picture out
