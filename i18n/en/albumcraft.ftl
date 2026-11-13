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
tags-proposed = Proposed:
tags-proposed-hint = Add this tag (right-click: it does not fit)
tags-not-this = Not “{ $tag }”: don't propose it for this

gallery-intro = { $n ->
    [one] A web page with one picture and your edits, viewable in any browser or uploadable to a web host.
   *[other] A web page with { $n } pictures and your edits, viewable in any browser or uploadable to a web host.
}
gallery-title = Title:
gallery-theme = Theme:
gallery-layout = Layout:
gallery-size = Picture size:
gallery-size-medium = Medium (1280 pixels, small files)
gallery-size-large = Large (2048 pixels, for big screens)
gallery-captions = Show captions
gallery-make = Choose folder and make…



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
settings-excluded-title = Left out folders
settings-excluded-intro = Folders inside a library folder that are not shown, with everything under them (for example Screenshots).
settings-excluded-none = Nothing is left out.
settings-excluded-add = Leave out a folder
settings-excluded-remove-hint = Show this folder again

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
settings-ai-intro = Finding faces, searching by describing a picture, proposing tags. Everything is computed on this computer: no picture, face or search text is sent anywhere. The networks these features need are downloaded only when you ask, from AlbumCraft's own releases.
settings-ai-unavailable = This build of AlbumCraft cannot run networks: these features are not available in it.
settings-ai-networks = Networks
# $size is like "12.5 MB"
settings-ai-pack-size = { $size }  ·  licence: { $licence }
settings-ai-remove = Remove
settings-ai-remove-hint = Remove the networks from this computer. What was found in your pictures is kept.
settings-ai-downloaded = Downloaded
settings-ai-downloading = Downloading… (see the bar at the bottom)
settings-ai-download = Download
settings-ai-remove-failed = The networks could not be removed: { $error }
settings-ai-search-uses = Smart search uses
settings-ai-search-auto = The pack for the interface's language
settings-ai-search-auto-hint = The multilingual pack when the interface is not in English, else the English one
settings-ai-download-task = Downloading { $pack }
settings-ai-analysis = Analysis of the library
settings-ai-analysis-text = Each picture is analysed once, in the background, while AlbumCraft has nothing else to do. It stops when AlbumCraft closes and carries on at the next start.
# $done and $total are numbers with thousands separators; $n is the total
settings-ai-progress = { $n ->
    [one] { $done } of { $total } picture analysed
   *[other] { $done } of { $total } pictures analysed
}
settings-ai-progress-paused = { $n ->
    [one] { $done } of { $total } picture analysed (paused)
   *[other] { $done } of { $total } pictures analysed (paused)
}
settings-ai-all-analysed = Every picture is analysed.
settings-ai-nothing = Nothing to analyse: no network is downloaded.
settings-ai-pause = Pause the analysis

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

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = All pictures
nav-picks = Picks
nav-places = Places
nav-places-hint = Your pictures on a map
nav-albums = Albums
nav-people = People
nav-folders = Folders
people-hint = Tag people from the selection bar.
scanning = Looking for pictures…

bar-import = Import
bar-import-hint = Copy pictures from a memory card, a camera or a folder into your library
bar-folders = Folders
bar-folders-hint = The folders shown in the library, where they are (nothing is copied)
bar-size-hint = Picture size
bar-picks-hint = Show only picks
bar-hide-rejects = Hide rejected pictures

sort-name = File name
sort-taken = Date taken
sort-rating = Rating
sort-hint = Order of the pictures: { $order }

filter-any = Any
filter-any-no-rejects = Any, no rejects
filter-min = { $n }+
filter-min-no-rejects = { $n }+, no rejects
filter-hint = Filter by rating; hide rejected pictures
filter-any-rating = Any rating
filter-or-more = { $rating } or more

album-rename = Rename…
album-delete = Delete album
album-show-map = Show on map
album-slideshow = Play as slideshow
album-deleted = Deleted the album "{ $name }". Its pictures are untouched.

empty-scanning = Looking for pictures…
empty-no-match = Nothing matches your search or filter.
empty-album = This album is empty. Select pictures and add them from the selection bar.
empty-person = Nobody tagged yet.
empty-picks = No picks yet. Hover a picture and click its flag, or press P.
empty-unreachable = No pictures were found in your library's folders. If they are on a disk that is not connected, connect it and scan again.
empty-library = Your library is empty. Import pictures, or add a folder you already have.
empty-folders-hint = The folders shown in the library
empty-import = Import pictures
empty-import-hint = Copy pictures from a memory card, a camera or a folder
empty-add = Add a folder
empty-add-hint = Show the pictures of a folder where they are, without copying them

folder-play-hint = Play this folder as a slideshow
folder-edit-hint = Edit caption and description
folder-map-hint = Show this folder on the map
tile-pick-hint = Pick (P)
tile-reject-hint = Reject (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = No rating
rating-stars = { $n ->
    [one] { $n } star
   *[other] { $n } stars
}
flag-pick = Pick
flag-unpick = Unpick
flag-reject = Reject
flag-unreject = Unreject
flag-rating = Rating
album-new = New album…
menu-copy-edits = Copy edits
menu-paste-edits = Paste edits
menu-paste-n = { $n ->
    [one] Paste { $n } edit
   *[other] Paste { $n } edits
}
copy-edits-select-one = Select one picture to copy its edits.
menu-import = Import pictures…
menu-add-folder = Add folder to library…
menu-folders = Library folders…
menu-scan = Scan folders again
menu-select-all = Select all
menu-select-none = Select none
menu-rotate-left = Rotate left
menu-rotate-right = Rotate right
menu-trash-rejects = Move rejected pictures to the trash…
menu-trash-rejects-hint = Remove the files of the rejected pictures from their folders, after a confirmation
menu-contact-sheet = Contact sheet…
menu-contact-sheet-hint = Pages of small pictures with a title and their file names, to print or save as PDF
menu-undo-edits = Undo all edits
menu-settings = Settings…
menu-about = About AlbumCraft…
menu-exit = Exit
edits-removed = { $n ->
    [one] Removed the edits of { $n } picture.
   *[other] Removed the edits of { $n } pictures.
}

status-library = Library: { $folders ->
    [one] { $folders } folder
   *[other] { $folders } folders
}, { $pictures ->
    [one] { $pictures } picture
   *[other] { $pictures } pictures
}, { $size } on disk
status-importing = Importing { $done } of { $total }…
status-stop = Stop
status-stop-hint = Stop after the current picture (the ones copied stay)
task-running = { $label }…
task-stopping = { $label }: stopping…
task-cancel-hint = Stop after the current picture
status-samples = Showing sample pictures. Use Import or Folders to bring in your own.
status-analysis = Analysing pictures { $done } / { $total }
status-analysis-paused = Analysis of the pictures paused ({ $done } / { $total })
status-analysis-pause = Pause
status-analysis-pause-hint = Stop analysing for now. What is done is kept; it also stops when AlbumCraft closes and carries on at the next start.
status-analysis-resume = Resume
status-analysis-resume-hint = Carry on analysing the pictures in the background

selected-count = { $n ->
    [one] { $n } picture
   *[other] { $count } pictures
}
# $n only matters in other languages (agreement of "selected")
selected-all = { $n ->
    [one] { $pictures } selected
   *[other] { $pictures } selected
}
selected-set = { $name } · { $pictures }
fallback-folder = Folder
fallback-album = Album
tray-nothing = Nothing selected
tray-click = Click pictures to select them

output-web = Web page
output-web-tip = Make a web gallery of these pictures
output-mail = Email
output-mail-tip = Small copies, attached to a new message in your mail program
output-print = Print
output-print-tip = Print these pictures (Ctrl+P)
output-export = Export
output-export-tip = Save edited copies to a folder
output-collage = Collage
output-collage-tip = Make a collage or a contact sheet
output-movie = Movie
output-movie-tip = Make a movie

bar-clear-hint = Clear the selection (Esc)
bar-pick-hint = Pick / unpick (P)
bar-reject-hint = Reject / unreject (X)
bar-rotate-left = Rotate left (Ctrl+[)
bar-rotate-right = Rotate right (Ctrl+])
bar-tags = Tags and people
bar-location = Set location: where these pictures were taken, on a map
bar-to-album = To album
bar-clear = Clear

ctx-open = Open in editor
ctx-slideshow = View slideshow from here
ctx-more-like-this = More like this
ctx-more-like-this-hint = Show the pictures that look like this one
ctx-add-to-album = Add to album
ctx-remove-from-album = Remove from album
ctx-tags = Tags and people…
ctx-location = Set location…
ctx-export = Export…
ctx-show-file = Show in file manager
ctx-open-default = Open in default program
ctx-open-with = Open with…
ctx-open-with-menu = Open with
ctx-copy-path = Copy path
ctx-rename = Rename…
ctx-move = Move to folder…
ctx-trash = Move to the trash…
notice-copied-path = { $n ->
    [one] Copied the path.
   *[other] Copied { $n } paths.
}
notice-open-failed = Could not open the program: { $error }
folder-select = Select this folder
folder-texts = Caption and description…
folder-open = Open in file manager
folder-refresh = Refresh thumbnails
folder-refresh-hint = Make this folder's thumbnails again from the files
folder-refreshing = Making the thumbnails of { $name } again.
folder-move = Move to…
folder-remove = Remove from library
folder-remove-hint = The files and your notes on them are not touched.
removed-folder = Took { $path } out of the library. Its files and your notes on them are untouched.
dialog-folder-texts = Caption and description
folder-caption-label = Caption (one line, shown under the folder's name):
folder-description-label = Description (shown above the folder's pictures):
dialog-rename-picture = Rename picture
dialog-rename-folder = Rename folder
rename-label = New name:
rename-keeps-ext = The extension .{ $ext } stays.
rename-button = Rename
dialog-move = Move to folder
move-pictures-into = { $n ->
    [one] Move { $name } into:
   *[other] Move the { $count } pictures into:
}
move-folder-into = Move the folder { $name } into:
move-filter = Filter folders
move-other = Other folder…
move-none = No other folder of the library matches.
files-renamed = Renamed { $from } to { $to }.
folder-trash = Move folder to the trash…
trash-folder-question = { $n ->
    [one] Move the folder { $name } and everything in it (1 picture, { $size }) to the trash?
   *[other] Move the folder { $name } and everything in it ({ $count } pictures, { $size }) to the trash?
}
dialog-delete-forever = Delete for good?
delete-forever-question = { $n ->
    [one] This item could not go to the trash (its disk has none, for example a network drive). Delete it for good? It can't be brought back.
   *[other] These { $n } items could not go to the trash (their disk has none, for example a network drive). Delete them for good? They can't be brought back.
}
delete-forever-button = Delete for good
delete-forever-keep = Keep them
task-move-across = { $n ->
    [one] Moving { $n } item to another disk
   *[other] Moving { $n } items to another disk
}
task-trash-folder = Moving { $name } to the trash
task-delete = { $n ->
    [one] Deleting { $n } item
   *[other] Deleting { $n } items
}
moved-across = { $n ->
    [one] Moved { $n } item to another disk.
   *[other] Moved { $n } items to another disk.
}
fail-move = Could not move the files: unexpected error.
fail-delete = Could not delete the files: unexpected error.
files-moved = { $n ->
    [one] Moved { $name } to { $folder }.
   *[other] Moved { $count } pictures to { $folder }.
}
files-more-problems = …and { $n } more.
trash-question-chosen = { $n ->
    [one] Move this picture ({ $size }) to the trash?
   *[other] Move these { $count } pictures ({ $size }) to the trash?
}

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Vivid
look-vivid-tip = Rich, saturated slide-film colours with deep blacks
look-soft-film = Soft film
look-soft-film-tip = Gentle portrait film: soft contrast, warm and kind to skin
look-polaroid = Instant
look-polaroid-tip = Faded instant-camera print: lifted blacks, warm highlights, cool shadows
look-cross-process = Cross process
look-cross-process-tip = Yellow highlights, blue-green shadows and punchy contrast
look-lomo = Lomo
look-lomo-tip = Saturated toy-camera look with dark corners
look-matte = Matte
look-matte-tip = Flat, faded blacks and muted colours
look-teal-orange = Teal & orange
look-teal-orange-tip = Cinema grade: teal shadows, warm skin and highlights
look-high-key = High key
look-high-key-tip = Bright and airy, with open shadows
look-cool = Cool
look-cool-tip = A cooler, bluer cast
look-warm = Warm
look-warm-tip = A warmer, golden cast
look-bw = B & W
look-bw-tip = Classic black and white
look-dramatic-bw = Dramatic B & W
look-dramatic-bw-tip = Black and white through a red filter: dark skies, strong local contrast
look-sepia = Sepia
look-sepia-tip = Old-photo brown tone
look-soft-focus = Soft focus
look-soft-focus-tip = Sharp centre, dreamy soft edges
look-glow = Glow
look-glow-tip = A soft luminous glow
look-film-grain = Film grain
look-film-grain-tip = Film-like grain
look-focal-bw = Focal B & W
look-focal-bw-tip = Colour in the middle, black and white around it
effect-sharpen = Sharpen
effect-sepia = Sepia
effect-bw = B & W
effect-warmify = Warmify
effect-film-grain = Film Grain
effect-tint = Tint
effect-saturation = Saturation
effect-soft-focus = Soft Focus
effect-glow = Glow
effect-filtered-bw = Filtered B & W
effect-focal-bw = Focal B & W
effect-graduated-tint = Graduated Tint
op-rotate-right = Rotate right
op-rotate-left = Rotate left
op-crop = Crop
op-straighten = Straighten { $angle }°
op-perspective-v = Perspective, vertical { $v }
op-perspective-h = Perspective, horizontal { $h }
op-perspective-both = Perspective { $v } / { $h }
op-redeye = Redeye
op-lucky = Auto fix (lucky)
op-auto-contrast = Auto Contrast
op-auto-color = Auto Color
op-tuning = Tuning
op-look = { $look } { $amount } %
op-adjust = Adjust
op-adjust-parts = Adjust: { $parts }
adj-wb = white balance
adj-exposure = exposure
adj-highlights = highlights
adj-shadows = shadows
adj-blacks = blacks
adj-contrast = contrast
adj-saturation = saturation
adj-vibrance = vibrance
adj-clarity = clarity
adj-sharpen = sharpen
adj-noise = noise reduction
decor-vignette-light = Light vignette
decor-vignette = Vignette
decor-instant = Instant-print frame
decor-border = Border
decor-shadow = Drop shadow
decor-rounded = Rounded corners
decor-text = Text "{ $text }"
wb-as-shot = As shot
wb-daylight = Daylight
wb-cloudy = Cloudy
wb-shade = Shade
wb-tungsten = Tungsten
wb-fluorescent = Fluorescent
wb-flash = Flash
wb-custom = Custom
wb-auto = Auto
gallery-theme-paper = Paper
gallery-theme-night = Night
gallery-theme-museum = Museum
gallery-theme-darkroom = Darkroom
gallery-theme-instant = Instant photos
gallery-theme-ocean = Ocean
gallery-theme-vintage = Vintage
gallery-theme-blossom = Blossom
gallery-layout-grid = Square grid
gallery-layout-masonry = Columns
gallery-layout-rows = Rows
crop-free = Free
crop-original = Original shape
crop-square = Square
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (10 × 15 print)
crop-16-9 = 16 × 9 (wide screen)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Double-click to reset
edit-loading = The picture is still loading.
edit-straightened = Straightened by { $angle }°.
edit-already-level = This picture already looks level.
edit-no-horizon = No clear horizon or vertical lines found to straighten by.
edit-histogram = Histogram and camera info
edit-unknown-camera = Unknown camera
edit-35mm = (35mm equivalent: { $mm }mm)
edit-focal = Focal Length: { $mm }mm
edit-iso = ISO: { $iso }
edit-no-camera = No camera information in this picture
edit-clip-shadows = shadows { $pct } %
edit-clip-highlights = highlights { $pct } %
edit-clipped = Clipped: { $parts }
edit-clipped-hint = Share of the picture that is pure black or pure white in at least one colour
edit-back = Back to Library
edit-info = Info
edit-info-hint = Properties, location and tags (I)
edit-side-by-side-hint = Show the pictures side by side
edit-stacked-hint = Show one picture above the other
edit-focus-hint = Change the picture the focus is on (where your edits go)
edit-compare = Compare
edit-compare-hint = This picture next to another one, side by side
edit-original = Original
edit-original-hint = Hold to see the picture without your edits (or hold O)
edit-slideshow = Slideshow
edit-fullscreen-hint = Full screen (F11): from this picture on, one after the other, with music
edit-next = Next picture
edit-previous = Previous picture
edit-position = { $n } of { $total }
edit-tab-edit = Edit
edit-tab-presets = Presets
edit-tab-effects = Effects
edit-undo = Undo
edit-undo-hint = Take back the last change (Ctrl+Z)
edit-redo = Redo
edit-list = Edits on this picture:
edit-list-none = (none — this is the original)
edit-step-hint =
    { $label }
    Click to go back to this step (the later ones are removed; Undo brings them back)
edit-copy-hint = Copy these edits to paste them on other pictures (Ctrl+Shift+C)
edit-paste-n = Paste { $n }
edit-paste-hint = Add the copied edits to this picture (Ctrl+Shift+V)
edit-basic = Basic
edit-advanced = Advanced
edit-wb-help = Click something in the picture that should be neutral gray or white.
edit-crop-help = Drag the box's edges or corners, drag inside it to move it, or draw a new box. Enter applies.
edit-crop-shape = Shape
edit-crop-orientation = Orientation
edit-landscape = Landscape
edit-landscape-hint = Wider than tall
edit-portrait = Portrait
edit-portrait-hint = Taller than wide
edit-apply = Apply
edit-remove-crop = Remove crop
edit-remove-crop-hint = Show the whole picture again
edit-redeye-help = Drag a box around the red eyes. Only red pupils inside the box are darkened.
edit-straighten-title = Straighten
edit-straighten-help = Move the slider until the horizon lines up with the grid.
edit-oneclick = One-click fixes
edit-lucky = I'm feeling lucky
edit-lucky-hint = Exposure, white balance, black point and a little vibrance, all at once (then fine-tune them in Advanced)
edit-auto-exposure = Auto exposure
edit-auto-exposure-hint = Brighten or darken to a balanced exposure
edit-auto-colour = Auto colour
edit-auto-colour-hint = Auto white balance: remove a colour cast
edit-auto-contrast = Auto contrast
edit-auto-contrast-hint = Stretch the darkest and brightest tones
edit-auto-straighten = Auto straighten
edit-auto-straighten-hint = Level a tilted horizon or verticals
edit-tools = Tools
edit-pick-neutral = Pick neutral
edit-pick-neutral-hint = Click something gray to set the white balance
edit-sec-wb = WHITE BALANCE
edit-sec-light = LIGHT
edit-sec-colour = COLOUR
edit-sec-detail = DETAIL
edit-sec-geometry = GEOMETRY
edit-wb-pick = Pick
edit-wb-pick-hint = Click something gray in the picture
edit-s-temperature = Temperature
edit-s-temperature-tip = Bluer (left) or warmer (right)
edit-s-tint = Tint
edit-s-tint-tip = Greener (left) or more magenta (right)
edit-s-exposure = Exposure
edit-s-exposure-tip = Overall brightness, in stops
edit-s-highlights = Highlights
edit-s-highlights-tip = Bright areas: left recovers detail, right brightens
edit-s-shadows = Shadows
edit-s-shadows-tip = Dark areas: right opens them up
edit-s-blacks = Blacks
edit-s-blacks-tip = Black point: left deepens, right lifts
edit-s-contrast = Contrast
edit-s-contrast-tip = Difference between dark and bright
edit-s-saturation = Saturation
edit-s-saturation-tip = All colours
edit-s-vibrance = Vibrance
edit-s-vibrance-tip = Mostly the dull colours; spares skin and already vivid ones
edit-s-clarity = Clarity
edit-s-clarity-tip = Local contrast in the midtones
edit-s-sharpen = Sharpen
edit-s-sharpen-tip = Crisper fine detail
edit-s-noise = Noise
edit-s-noise-tip = Noise reduction: smooths grain while keeping edges
edit-reset-all = Reset all
edit-crop-btn = Crop…
edit-crop-btn-hint = Choose what part of the picture to keep, or change the crop
edit-s-straighten-tip = Turn the picture a little to level the horizon (zooms in to hide the corners)
edit-s-vertical = Vertical
edit-s-vertical-tip = Perspective: right widens the top, to stand up buildings that lean back when shot from below
edit-s-horizontal = Horizontal
edit-s-horizontal-tip = Perspective: right makes the right side taller, for a wall seen at an angle
edit-intensity = Intensity
edit-remove-preset = Remove preset
edit-presets-help = Looks for the whole picture. Click one, then set its intensity.
edit-effects-add = Add on or around the picture
edit-add-instant = Instant frame
edit-add-text = Text
edit-your-text = Your text
edit-effects-note = Effects go on top of every other edit. Add one to adjust it here.
edit-p-amount = Amount
edit-p-size = Size
edit-p-width = Width
edit-p-bottom = Bottom
edit-p-colour = Colour
edit-p-darkness = Darkness
edit-p-background = Background
edit-p-radius = Radius
edit-vignette-note = Left of the middle lightens the corners instead.
edit-halo = Soft halo (readable on any picture)
edit-remove-effect = Remove this effect
edit-caption-hint = Make a caption!
edit-print-hint = Print this picture (Ctrl+P)
edit-album = Album
edit-zoom-hint = Zoom (100 % = the photo's real pixels)
edit-zoom-slider-hint = Zoom: left = fit, middle = 1:1, right = 400 % (mouse wheel works too; drag the picture to zoom to 100 % and move it)
edit-one-to-one-hint = Original size (100 %): one photo pixel per screen pixel
edit-fit = Fit
edit-fit-hint = Fit the whole picture in the window (double-click a zoomed picture does the same)
edit-people = People:
edit-person-remove = Remove person
edit-person-add = add person
edit-compare-none = No other picture in this folder
edit-compare-selected = Selected
edit-compare-swap-hint = Click to edit this picture instead
edit-view-loading = Loading…
edit-sharpening = Sharpening…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = This picture can't be read.
edit-full-unreadable = The full picture can't be read.
edit-full-loading = Loading full picture…
edit-draw-box = Drag on the picture to draw a box
edit-click-neutral = Click something that should be gray or white
edit-wb-title = White balance

## Info panel, search, slideshow bar

info-hide-hint = Hide this panel (I)
info-properties = Properties
info-camera = Camera
info-lens = Lens
info-taken = Taken on
info-file-date = { $date } (file date)
info-size = Size
info-megapixels = { $w } × { $h } ({ $mp } MP)
info-file = File
info-focal = Focal length
info-focal-35 = { $f } mm (35 mm: { $e } mm)
info-aperture = Aperture
info-exposure = Exposure
info-folder = Folder
info-sample = Built-in sample picture
info-edits = Edits
info-no-exif = No camera information (EXIF) in this picture.
info-exif = Extended EXIF
info-flash = Flash
info-bias = Exposure bias
info-program = Program
info-metering = Metering
info-distance = Distance
info-colour-space = Colour space
info-gps = GPS position
info-digitized = Digitized
info-modified = Modified
info-software = Software
info-preview = Camera preview
info-preview-hint = { $w } × { $h } pixels, stored in the file
info-all-exif = All EXIF data ({ $n } entries)
info-location = Location
info-set-in-app = Set in AlbumCraft
info-from-gps = From the camera's GPS
info-modify = Modify…
info-modify-hint = Move the pin, rename the place or remove it
info-show-map-hint = Open Places on this picture
info-no-location = No location yet.
info-set-location-hint = Find the place or click it on a map
info-tags = Tags
info-no-tags = No tags yet.
info-remove-tag = Remove this tag
info-add-tag = Add a tag…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = E
coord-w = W
info-orient-2 = Mirrored
info-orient-3 = Upside down
info-orient-4 = Mirrored, upside down
info-orient-5 = Mirrored, turned left
info-orient-6 = Turned right (camera held upright)
info-orient-7 = Mirrored, turned right
info-orient-8 = Turned left (camera held upright)
search-remove-hint = Remove from the search
search-hint = Search tags, places, captions…
search-and = and…
search-clear-hint = Clear the search
search-for = Search for “{ $text }”
search-kind-tags = Tags
search-like-chip = Like { $name }
search-looks-like-title = Looks like
search-similar-title = Pictures like this one
search-still-analysing = (analysis still running: some pictures are not ranked yet)
search-show-more = Show more
search-show-more-hint = { $n ->
    [one] One more picture matches less well
   *[other] { $count } more pictures match less well
}
search-like-no-pack = To find similar pictures, download a search pack in Settings › AI features.
search-failed-because = Smart search failed: { $error }
search-failed = the search stopped unexpectedly
search-no-folder = AlbumCraft has no folder for its data
search-pack-failed = Smart search could not start: { $error }
search-not-analysed = This picture is not analysed yet.
music-dialog-title = Music for the slideshow
filter-music = Music
ss-previous = Previous (←)
ss-play-pause = Play / pause (Space)
ss-next = Next (→)
ss-seconds = Seconds per picture
ss-interval-off = Off: pictures change only with → or the buttons (Settings, Slideshow)
ss-fade = Fade
ss-motion = Motion
ss-other-music-hint = Choose another music file
ss-exit-full = Exit full screen (F11 or Esc)
ss-exit = Exit (Esc)

## Messages of the window (src/app.rs)

save-failed-prefix = Your library could not be saved:
save-failed =
    { $prefix } { $error }
    Your latest changes are only in this window for now; AlbumCraft keeps trying.
save-ok-again = Your library is saved again.
lib-unreadable-kept =
    Your library file could not be read ({ $error }).

    It was kept as:
    { $backup }

    AlbumCraft started with an empty library. Your pictures themselves are not affected.
lib-unreadable-nosave =
    Your library file could not be read ({ $error }).

    AlbumCraft started with an empty library and won't save any changes this time, so that the file is not overwritten.
dialog-library = Library
crash-title = AlbumCraft closed unexpectedly
crash-text =
    AlbumCraft stopped because of an error the last time it ran. Changes made in the last few seconds before that may be missing.

    Details were written to:
    { $log }

    Please include that file when reporting the problem.
picasa-imported =
    { $n ->
        [one] Brought in stars, captions and edits from the .picasa.ini file of { $n } folder.
       *[other] Brought in stars, captions and edits from the .picasa.ini files of { $n } folders.
    }
copy-no-edits = This picture has no edits to copy.
copied-edits =
    { $n ->
        [one] Copied { $n } edit. Select pictures and paste them (Ctrl+Shift+V).
       *[other] Copied { $n } edits. Select pictures and paste them (Ctrl+Shift+V).
    }
paste-first = Copy the edits of a picture first (Ctrl+Shift+C).
pasted-edits =
    { $n ->
        [one] Pasted the edits onto { $n } picture.
       *[other] Pasted the edits onto { $n } pictures.
    }
no-pictures-to-show = There are no pictures to show.
music-error = No music: { $error }
choose-folder-dialog = Choose a folder of pictures
added-to-album =
    { $n ->
        [one] Added { $n } picture to "{ $name }".
       *[other] Added { $n } pictures to "{ $name }".
    }
trash-samples = Sample pictures can't be moved to the trash.
trash-no-rejects = No picture is rejected. Press X to reject one.
task-trash =
    { $n ->
        [one] Moving { $n } picture to the trash
       *[other] Moving { $n } pictures to the trash
    }
task-email =
    { $n ->
        [one] Preparing { $n } picture for an email
       *[other] Preparing { $n } pictures for an email
    }
default-title = My pictures
gallery-folder-dialog = Make the web gallery in…
task-gallery =
    { $n ->
        [one] Making the web gallery of { $n } picture
       *[other] Making the web gallery of { $n } pictures
    }
movie-save-dialog = Save the movie as…
filter-mp4 = MP4 video
task-movie = Making the movie
scan-truncated = Only the first { $n } pictures are shown: remove some folders from the library to see the rest.
selection-none = No Selection
selection-one = { $name }     { $date }     { $w }x{ $h } pixels     { $size }
selection-several = { $pictures }     { $a }      { $size } on disk
selection-range = { $pictures }     { $a } to { $b }     { $size } on disk

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Photo pile
cstyle-pile-tip = Scattered pictures, like prints thrown on a table
cstyle-mosaic = Mosaic
cstyle-mosaic-tip = Pictures of different sizes fill the page
cstyle-frame = Centre stage
cstyle-frame-tip = A mosaic around one big picture in the middle
cstyle-grid = Grid
cstyle-grid-tip = Regular rows and columns
cstyle-film = Film strip
cstyle-film-tip = Pictures on strips of film
cstyle-clothesline = Clothesline
cstyle-clothesline-tip = Instant pictures pegged on lines
cstyle-fan = Fan
cstyle-fan-tip = Spread out like a hand of cards
cstyle-shape = Shape
cstyle-shape-tip = A heart, circle or star made of pictures
cstyle-blend = Multiple exposure
cstyle-blend-tip = The pictures shown through each other (best with 2 to 4)
cstyle-sheet = Contact sheet
cstyle-sheet-tip = Pages of small pictures with a title and their file names
shape-heart = Heart
shape-circle = Circle
shape-star = Star
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = Letter
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Square
fmt-wide = Screen (16:9)
grad-vertical = Top to bottom
grad-diagonal = Diagonal
grad-radial = From the centre
collage-need-sheet = Select the pictures for the contact sheet first.
collage-need-two = Select at least two pictures to make a collage.
collage-unreadable = That picture could not be read.
collage-new-layout = New layout
collage-new-layout-hint = Other positions, turns and sizes
collage-shuffle = Shuffle
collage-shuffle-hint = Put the pictures in a random order
collage-spacing = Space between pictures
border-none = None
border-white = White
border-instant = Instant
collage-frame-text = Text on the frames
cap-none = None
cap-caption = Caption
collage-shadows = Shadows
bg-gradient = Gradient
bg-picture = Picture
collage-to = to
collage-use-selected = Use the selected picture
collage-use-selected-hint = Click a picture of the collage first
collage-choose-file = Choose a file…
collage-no-bg = No picture chosen yet
collage-bg-dialog = Background picture
collage-page = Page
collage-size = Picture size
collage-px = { $px } pixels
collage-centre = Put in the middle
collage-put-back = Put back
collage-put-back-hint = Undo the moves of this picture
collage-leave-out = Leave out
collage-leave-out-hint = Take this picture out of the collage (Delete)
collage-title = Title
collage-no-title = No title
collage-no-subtitle = No second line
collage-header = Title on the first page
collage-under = Under each picture
collage-date = Date
collage-pictures = Pictures
collage-page-numbers = Page numbers
collage-sending = Sending page { $page } of { $total } to the printer…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }
collage-no-printer = No printer is available
collage-save-pdf = Save as PDF…
collage-save-picture = Save picture…
collage-save-picture-hint = Save the collage as a JPEG or PNG picture
collage-file-sheet = Contact sheet
collage-file-collage = Collage
collage-save-pdf-title = Save as PDF
collage-save-picture-title = Save the collage as a picture
filter-pdf = PDF document
filter-jpeg = JPEG picture
filter-png = PNG picture
collage-saving-sheet = Saving the contact sheet…
collage-saving = Saving the collage…
task-save-collage = Saving the collage
collage-printed =
    { $n ->
        [one] Sent { $n } page to { $printer }.
       *[other] Sent { $n } pages to { $printer }.
    }
collage-print-failed = Printing didn't finish: { $error }
collage-hint-swap = Drag a picture onto another to swap them. Click one to select it (Delete leaves it out).
collage-hint-free = Drag a picture to move it, scroll over it to turn it. Click one to select it (Delete leaves it out).
collage-page-of = Page { $page } of { $total }

## Places map (src/map_view.rs)

map-search-still = Still looking for GPS positions in your pictures: try again in a moment.
map-none-located =
    { $n ->
        [one] This picture has no location yet. Select it and click { $icon } (Set location) in the selection bar to place it.
       *[other] None of these { $n } pictures has a location yet. Select them and click { $icon } (Set location) in the selection bar to place them.
    }
map-location-set =
    { $n ->
        [one] Location set for { $n } picture.
       *[other] Location set for { $n } pictures.
    }
map-location-removed =
    { $n ->
        [one] Location removed from { $n } picture.
       *[other] Location removed from { $n } pictures.
    }
map-nothing-online = Nothing found online.
map-set-location = Set location
map-sub-scope =
    { $n } of { $total ->
        [one] { $total } picture
       *[other] { $total } pictures
    } with a location
map-sub-indexing = { $n } with a location · looking for GPS positions…
map-sub-all =
    { $n ->
        [one] { $n } picture with a location
       *[other] { $n } pictures with a location
    }
map-title-scope = Places · { $title }
map-show-whole = Show the whole library
map-online = Online map
map-builtin = Built-in map
map-online-tip = Showing OpenStreetMap. Click to use the built-in map only (no internet connections).
map-builtin-tip = Showing the built-in world map. Click to show detailed maps from OpenStreetMap (downloads map pictures from the internet).
map-show-all = Show all
map-show-all-hint = Zoom to show every picture on the map
map-search-online-hint = Search a place or address
map-search-builtin-hint = Search a city or coordinates
map-no-city-builtin = No city by that name in the built-in list.
map-no-city-hint = No city by that name. Try coordinates, like 44.84, -0.58.
map-cities = CITIES
map-cities-builtin = CITIES (BUILT-IN LIST)
map-searching = Searching online…
map-places-osm = PLACES AND ADDRESSES (OPENSTREETMAP)
map-search-addresses = Search addresses online   (Enter)
map-where = Where were these taken?
map-where-help = Search for a place above, or click on the map to drop a pin. You can drag the pin afterwards.
map-name-hint = Name of the place (optional)
map-drag-pin = Drag the pin to adjust it.
map-drag-pin-offline = Drag the pin to adjust it. Online maps show streets.
map-set-n =
    { $n ->
        [one] Set location for { $n } picture
       *[other] Set location for { $n } pictures
    }
map-remove-location = Remove location
map-remove-location-hint = These pictures will have no location, even if the camera stored one (the files are not changed)
map-close-hint = Close (Esc)
map-change-location = Change location…
map-select-in-library = Select in library
map-select-in-library-hint = Select these pictures in the library
map-show-hint = Show on the map
map-preparing = Preparing the map…
map-hover = { $pictures } · { $place }
map-credit-builtin = Built-in map: Natural Earth · GeoNames
map-problem = { $problem }. Showing the built-in map.
map-empty-indexing = Looking for GPS positions in your pictures…
map-empty =
    No pictures with a location yet.

    Pictures from phones and GPS cameras show up here by themselves. To place others, select them and click { $icon } (Set location) in the selection bar.

## Import (src/import_view.rs)

import-subtitle = Copy pictures into your library
import-from = From
import-choose = Choose…
import-refresh-hint = Look for cards and drives again, and read the folder again
import-hide-dups = Hide the { $n } already in the library
import-dups-hint = Pictures with the same content as one in your library, even under another name
import-uncheck-all = Uncheck all
import-check-all = Check all
import-copy-into = Copy into
import-another-folder = Another folder…
import-name-hint = New folder name, e.g. Summer in Spain
import-checked = { $chosen } of { $shown } checked. Nothing is overwritten; the originals are not changed.
import-go =
    { $n ->
        [one] Import { $n } picture
       *[other] Import { $n } pictures
    }
import-dest-dialog = Put the new folder in…
import-from-dialog = Import pictures from…
import-pick-dest = Choose where to put the new folder.
import-running = An import is still running: wait for it to finish (see the status bar).
import-started =
    { $n ->
        [one] Importing { $n } picture into { $path }…
       *[other] Importing { $n } pictures into { $path }…
    }
import-listing-in = Looking for pictures in { $from }…
import-none-in = No pictures in { $path }.
import-choose-another = Choose another folder…
import-heading = Import from a memory card, a camera or a folder
import-copy-note = The pictures are copied into a new folder of your library. The originals are not changed.
import-no-cards = No memory card or camera found. Plug one in and click { $icon } above.
import-instead = To show pictures where they already are, without copying them:
import-add-instead = add a folder to the library instead
import-all-dups =
    { $n ->
        [one] The picture is already in your library.
       *[other] All { $n } pictures are already in your library.
    }
import-show-anyway = Show them anyway
import-truncated = Only the first { $n } pictures are listed.
import-group-hint = Check or uncheck this whole group
import-in-library = In the library
import-click-preview = Click a picture to see it here
import-this = Import this picture
import-this-hint = Space
import-already = already in your library

## Print (src/print_view.rs)

print-select-first = Select the pictures you want to print first.
print-page-layout = Page Layout
print-layout-full = Full Page
print-shrink = Shrink to Fit
print-shrink-hint = The whole picture is printed, with white space where the shapes differ
print-crop = Crop to Fit
print-crop-hint = The picture fills the print size; edges that don't fit are cut off
print-options = Border and Text Options
print-border = Border
print-no-border = No border
print-thin-line = Thin black line
print-white-border = White border
print-text-under = Text under each picture
print-no-text = No text
print-printer = Printer
print-no-printer-found = No printer found
print-setup = Printer Setup…
print-setup-hint = Paper size, orientation and the printer's own settings
print-paper = Paper
print-default = Printer default
print-no-answer = No printer answered — showing an A4 page
print-paper-describe = { $dpi } dpi, { $w } x { $h } in ({ $orientation })
print-copies = Copies per picture
print-copies-n =
    { $n ->
        [one] { $n } copy
       *[other] { $n } copies
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } print
       *[other] { $prints } prints
    }
print-cancel = Cancel  { $icon }
print-go = Print  { $icon }
print-review = Review
print-review-hint = Check that each picture has enough pixels for its print size
print-none-left = There are no pictures left to print.
print-job-printed =
    { $n ->
        [one] Sent { $n } page to { $printer }.
       *[other] Sent { $n } pages to { $printer }.
    }
print-preview = Preview
print-ready = All of your pictures are ready to print.
print-too-small = Some of your pictures are too small to print well.  You can remove these pictures, print them anyway, or cancel and change the print size.
print-quality-best = Best quality ({ $ppi } pixels/inch)
print-quality-good = Good quality ({ $ppi } pixels/inch)
print-quality-bad = Bad quality ({ $ppi } pixels/inch)

## Results of background work (src/backend.rs)

backend-no-data-dir = No folder for application data was found.
backend-cache-fail = The cache could not be emptied ({ $error })
fail-export = Could not export: unexpected error.
fail-trash = Could not move the pictures to the trash: unexpected error.
fail-email = Could not prepare the email: unexpected error.
fail-gallery = Could not make the web gallery: unexpected error.
fail-movie = Could not make the movie: unexpected error
fail-collage = Could not save the collage: unexpected error.
fail-download = The download stopped: unexpected error.
analysis-no-data-dir = No folder for application data was found.
analysis-stopped = The analysis of the pictures was stopped: it failed { $n } times in a row ({ $error }). It will be tried again the next time AlbumCraft starts.
pack-nothing-downloaded = Nothing was downloaded: { $error }.
pack-downloaded = { $pack } is downloaded ({ $size }).
pack-download-stopped = The download of { $pack } was stopped. Starting it again continues where it was.
pack-download-failed = { $pack } could not be downloaded. { $error }
export-nothing = Nothing was exported: { $error }.
export-result =
    Exported { $ok } of { $total ->
        [one] { $total } picture
       *[other] { $total } pictures
    } to { $path }
export-stopped = The export was stopped before the end.
export-failed-list = Not exported:
export-more = … and { $n } more
gallery-ready =
    The web gallery is ready:
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } picture could not be read and was left out.
       *[other] { $n } pictures could not be read and were left out.
    }
gallery-stopped = The web gallery was not made: stopped.
gallery-failed = Could not make the web gallery: { $error }
movie-ready =
    The movie is ready:
    { $path }
movie-stopped = The movie was not made: stopped.
movie-failed = Could not make the movie: { $error }
collage-saved-many =
    Saved { $n } pictures, the first one is:
    { $path }
collage-saved =
    Saved:
    { $path }
collage-save-failed = Could not save the collage: { $error }
import-unexpected = The import stopped unexpectedly.
import-summary =
    { $n ->
        [one] Imported { $n } picture into { $path }.
       *[other] Imported { $n } pictures into { $path }.
    }
import-already-there =
    { $n ->
        [one] { $n } was already there.
       *[other] { $n } were already there.
    }
import-was-stopped = The import was stopped before the end.
import-failed-count = { $n } could not be copied:

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec
locale-months-long = January, February, March, April, May, June, July, August, September, October, November, December
# how a date reads; $month is the short name
locale-date = { $month } { $day }, { $year }
locale-decimal = .
locale-group = ,
locale-unit-kb = KB
locale-unit-mb = MB
locale-unit-gb = GB

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Type a name for the new folder.
import-name-chars = A folder name can't contain / \ : * ? " < > |
import-name-long = That folder name is too long.
import-drive = Drive { $letter }:
import-mkdir-fail = The folder { $path } could not be made: { $error }
import-too-many = too many files with this name
import-appeared = a file with this name appeared during the copy
export-logo-bad = the logo { $path } can't be read ({ $error })
export-unreadable = the picture could not be read
gallery-too-many = too many galleries with this name here
gallery-none-read = none of the pictures could be read
movie-no-pictures = no pictures
movie-ffmpeg-start = FFmpeg could not be started: { $error }
movie-ffmpeg-input = FFmpeg has no input
movie-ffmpeg-stopped = FFmpeg stopped ({ $error })
movie-ffmpeg-failed =
    FFmpeg failed:
    { $tail }
root-already = That folder is already in the library.
root-inside = That folder is already in the library, as part of { $outer }.
root-excluded = That folder is inside { $folder }, which is left out of the library.
exclude-is-root = That is a library folder: remove it instead.
exclude-outside = That folder is not inside a library folder.
exclude-already = That folder is already left out.
name-empty = Type a name.
name-bad-char = A name can't contain { $char }.
name-dot = A name can't end with a dot.
name-long = That name is too long.
move-no-parent = { $path } has no folder.
move-no-name = { $path } has no name.
move-sample = Sample pictures can't be moved or renamed.
move-missing = { $path } is not there any more.
move-into-itself = A folder can't be moved into itself.
move-no-folder = The folder { $path } doesn't exist.
move-taken = { $name } already exists there.
move-failed = Could not move { $path }: { $error }
move-outside = { $path } is not in the library: choose a folder inside a library folder.
move-stopped = Stopped: { $path } was not moved.
move-original-left = { $to } was copied, but the original { $path } could not be removed ({ $error }): both are there now.
deleted-for-good = { $n ->
    [one] Deleted { $n } item for good.
   *[other] Deleted { $n } items for good.
}
deleted-stopped = Stopped before the end.
online-tile-bad = a map tile could not be read: { $error }
online-map-down = Online map not reachable ({ $error })
online-search-down = The online search is not reachable ({ $error }).
online-search-odd = Unexpected answer from the online search: { $error }
collage-nothing = there is nothing to save
collage-extension = save as .pdf, .png or .jpg
collage-page-fail = a page could not be drawn
geo-near = Near { $name }
mail-no-pictures = no pictures to attach
mail-copies-fail = Could not make the copies for the email: { $error }
mail-stopped = The email was not prepared: stopped.
mail-none-read = Could not make the copies for the email: none of the pictures could be read.
mail-left-out =
    { $n ->
        [one] { $n } picture could not be read and was left out.
       *[other] { $n } pictures could not be read and were left out.
    }
mail-opened =
    A new message with { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    } is open in your mail program.{ $left_out }
mail-asked =
    Your mail program was asked for a new message with { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    }. If they are not attached, attach them from:
    { $path }{ $left_out }
mail-open-fail =
    Could not open your mail program ({ $error }).
    The small copies of { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    } are in:
    { $path }
    Attach them to your message.{ $left_out }
mail-too-many = too many pictures
mail-no-mapi = no mail program (mapi32.dll: { $error })
mail-no-attach = the mail program does not take attachments (no MAPISendMailW)
mail-refused = the mail program refused the message (MAPI error { $code })
mail-open-mail = could not open Mail: { $error }
mail-open-mail-short = could not open Mail
mail-no-xdg = xdg-email (from xdg-utils) is not installed
mail-xdg-fail = could not start xdg-email: { $error }
gallery-html-lang = en
gallery-count = { $n ->
    [one] { $n } picture
   *[other] { $n } pictures
}
gallery-close = Close
gallery-prev = Previous
gallery-next = Next
gallery-zoom = Zoom
sheet-count = { $n ->
    [one] { $n } picture
   *[other] { $n } pictures
}

# Tag proposals: the built-in vocabulary (core vocabulary.rs). Short tag names, capitalised.
tag-beach = Beach
tag-sea = Sea
tag-coast = Coast
tag-island = Island
tag-harbour = Harbour
tag-lake = Lake
tag-river = River
tag-waterfall = Waterfall
tag-mountain = Mountains
tag-hills = Hills
tag-valley = Valley
tag-forest = Forest
tag-countryside = Countryside
tag-field = Fields
tag-vineyard = Vineyard
tag-desert = Desert
tag-glacier = Glacier
tag-volcano = Volcano
tag-cave = Cave
tag-park = Park
tag-garden = Garden
tag-city = City
tag-skyline = Skyline
tag-street = Street
tag-old-town = Old town
tag-village = Village
tag-market = Market
tag-square = Town square
tag-home = At home
tag-kitchen = Kitchen
tag-restaurant = Restaurant
tag-cafe = Café
tag-bar = Bar
tag-hotel = Hotel
tag-office = Office
tag-school = School
tag-museum = Museum
tag-stadium = Stadium
tag-airport = Airport
tag-station = Train station
tag-road = Road
tag-campsite = Camping
tag-farm = Farm
tag-zoo = Zoo
tag-aquarium = Aquarium
tag-underwater = Underwater
tag-sky = Sky
tag-architecture = Architecture
tag-church = Church
tag-cathedral = Cathedral
tag-temple = Temple
tag-mosque = Mosque
tag-castle = Castle
tag-palace = Palace
tag-ruins = Ruins
tag-monument = Monument
tag-statue = Statue
tag-tower = Tower
tag-lighthouse = Lighthouse
tag-bridge = Bridge
tag-house = House
tag-interior = Interior
tag-stairs = Stairs
tag-window = Window
tag-door = Door
tag-snow = Snow
tag-ice = Ice
tag-rain = Rain
tag-fog = Fog
tag-storm = Storm
tag-rainbow = Rainbow
tag-clouds = Clouds
tag-sunset = Sunset
tag-sunrise = Sunrise
tag-night = Night
tag-northern-lights = Northern lights
tag-stars = Starry sky
tag-moon = Moon
tag-autumn = Autumn
tag-spring = Spring
tag-summer = Summer
tag-winter = Winter
tag-flowers = Flowers
tag-tree = Trees
tag-leaves = Leaves
tag-mushroom = Mushrooms
tag-rocks = Rocks
tag-sand = Sand
tag-portrait = Portrait
tag-selfie = Selfie
tag-group = Group photo
tag-family = Family
tag-couple = Couple
tag-baby = Baby
tag-children = Children
tag-friends = Friends
tag-crowd = Crowd
tag-wedding = Wedding
tag-birthday = Birthday
tag-christmas = Christmas
tag-easter = Easter
tag-halloween = Halloween
tag-new-year = New Year
tag-fireworks = Fireworks
tag-party = Party
tag-concert = Concert
tag-festival = Festival
tag-parade = Parade
tag-graduation = Graduation
tag-picnic = Picnic
tag-barbecue = Barbecue
tag-dinner = Dinner
tag-ceremony = Ceremony
tag-performance = Show
tag-dance = Dance
tag-travel = Travel
tag-hiking = Hiking
tag-climbing = Climbing
tag-skiing = Skiing
tag-snowboard = Snowboarding
tag-sailing = Sailing
tag-surfing = Surfing
tag-swimming = Swimming
tag-diving = Diving
tag-kayak = Kayaking
tag-fishing = Fishing
tag-cycling = Cycling
tag-running = Running
tag-football = Football
tag-tennis = Tennis
tag-golf = Golf
tag-basketball = Basketball
tag-horse-riding = Horse riding
tag-sport = Sport
tag-camping = Camping trip
tag-gardening = Gardening
tag-cooking = Cooking
tag-reading = Reading
tag-music = Music
tag-games = Games
tag-sightseeing = Sightseeing
tag-road-trip = Road trip
tag-animals = Animals
tag-dog = Dog
tag-cat = Cat
tag-bird = Bird
tag-horse = Horse
tag-cow = Cows
tag-sheep = Sheep
tag-goat = Goat
tag-pig = Pig
tag-chicken = Chickens
tag-duck = Ducks
tag-fish = Fish
tag-insect = Insects
tag-butterfly = Butterfly
tag-rabbit = Rabbit
tag-deer = Deer
tag-wildlife = Wildlife
tag-whale = Whale
tag-seal = Seal
tag-elephant = Elephant
tag-food = Food
tag-meal = Meal
tag-breakfast = Breakfast
tag-cake = Cake
tag-dessert = Dessert
tag-bread = Bread
tag-fruit = Fruit
tag-vegetables = Vegetables
tag-seafood = Seafood
tag-pizza = Pizza
tag-coffee = Coffee
tag-wine = Wine
tag-beer = Beer
tag-drinks = Drinks
tag-car = Car
tag-vintage-car = Vintage car
tag-motorbike = Motorbike
tag-bicycle = Bicycle
tag-bus = Bus
tag-train = Train
tag-tram = Tram
tag-plane = Aeroplane
tag-boat = Boat
tag-ship = Ship
tag-truck = Truck
tag-tractor = Tractor
tag-toys = Toys
tag-books = Books
tag-art = Art
tag-street-art = Street art
tag-sculpture = Sculpture
tag-instrument = Musical instrument
tag-computer = Computer
tag-phone = Phone
tag-clock = Clock
tag-black-and-white = Black and white
tag-macro = Close-up
tag-aerial = Aerial view
tag-panorama = Panorama
tag-landscape = Landscape
tag-silhouette = Silhouette
tag-reflection = Reflection
tag-long-exposure = Long exposure
tag-golden-hour = Golden hour
tag-backlight = Backlight
tag-minimalist = Minimalist
tag-abstract = Abstract
tag-vintage = Vintage
tag-lights = City lights
tag-screenshot = Screenshot
tag-document = Document
tag-receipt = Receipt
tag-whiteboard = Whiteboard
tag-map = Map
tag-sign = Sign
tag-ticket = Ticket
