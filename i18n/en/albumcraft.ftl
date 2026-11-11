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
bar-appearance = Appearance: { $mode } (click to change)
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
folder-edit-hint = Edit description
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
ctx-add-to-album = Add to album
ctx-remove-from-album = Remove from album
ctx-tags = Tags and people…
ctx-location = Set location…
ctx-export = Export…
ctx-show-file = Show in file manager

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
gallery-style-light = Light grid
gallery-style-dark = Dark grid
gallery-style-masonry = Masonry (whole pictures)
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
