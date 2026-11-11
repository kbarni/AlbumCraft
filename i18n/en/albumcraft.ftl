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
