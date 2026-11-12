# Text der AlbumCraft-Oberfläche auf Deutsch.

language-name = Deutsch

## Shared buttons and words

button-ok = OK
button-cancel = Abbrechen
button-close = Schließen
button-remove = Entfernen
button-save = Speichern
add-folder = Ordner hinzufügen…
scan-again = Erneut durchsuchen
removed-root = { $path } wurde aus der Bibliothek entfernt. Die Fotos und Ihre Notizen dazu sind unverändert.

## Settings dialog

settings-title = Einstellungen
settings-saved-note = Änderungen werden sofort gespeichert.

settings-section-interface = Oberfläche
settings-section-folders = Ordner
settings-section-editing = Bearbeitung
settings-section-slideshow = Diashow
settings-section-export = Export
settings-section-maps = Karten
settings-section-storage = Speicher
settings-section-ai = KI-Funktionen

## Settings > Interface

settings-language = Sprache
settings-language-system = Wie das System ({ $language })
settings-language-note = Weitere Sprachen können als Übersetzungsdateien hinzugefügt werden; siehe i18n/ im Quellcode.
settings-theme = Design
theme-system = Wie das System
theme-light = Hell
theme-dark = Dunkel

## Dialogs (src/dialogs.rs)

dialog-new-album = Neues Album
dialog-rename-album = Album umbenennen
dialog-tags = Stichwörter und Personen
dialog-gallery = Web-Galerie
dialog-folder-note = Ordnerbeschreibung
dialog-movie = Film
dialog-about = Über AlbumCraft
dialog-online-maps = Karten
dialog-print-review = Prüfung
dialog-trash = In den Papierkorb legen

new-album-name = Name des Albums:
new-album-adds = { $n ->
    [one] Das ausgewählte Foto wird hinzugefügt.
   *[other] Die { $n } ausgewählten Fotos werden hinzugefügt.
}
new-album-create = Erstellen
rename-album-rename = Umbenennen

tags-intro = { $n ->
    [one] Wird zum ausgewählten Foto hinzugefügt. Trennen Sie die Wörter mit Kommas.
   *[other] Wird zu den { $n } ausgewählten Fotos hinzugefügt. Trennen Sie die Wörter mit Kommas.
}
tags-tags = Stichwörter:
tags-people = Personen:
tags-add = Hinzufügen

gallery-intro = { $n ->
    [one] Eine Webseite mit einem Foto und Ihren Bearbeitungen, in jedem Browser lesbar oder auf einem Webhoster zu veröffentlichen.
   *[other] Eine Webseite mit { $n } Fotos und Ihren Bearbeitungen, in jedem Browser lesbar oder auf einem Webhoster zu veröffentlichen.
}
gallery-title = Titel:
gallery-style = Stil:
gallery-size = Größe der Fotos:
gallery-size-medium = Mittel (1280 Pixel, kleine Dateien)
gallery-size-large = Groß (2048 Pixel, für große Bildschirme)
gallery-captions = Bildunterschriften anzeigen
gallery-make = Ordner wählen und erstellen…

folder-note-intro = Eine kurze Beschreibung, die unter dem Ordnernamen angezeigt wird:


# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] Ein Video mit einem Foto wie in der Diashow: { $seconds } s
   *[other] Ein Video mit { $n } Fotos wie in der Diashow: je { $seconds } s
}{ $fade ->
    [yes] , mit Überblendungen
   *[no] {""}
}{ $motion ->
    [yes] , mit Schwenk und Zoom
   *[no] {""}
}.
movie-length = Dauer: etwa { $seconds } s. Zeiten und Effekte ändern Sie in der Diashow-Leiste oder in den Einstellungen.
movie-music = Musik
movie-other-music = Andere Musik…
movie-add-music = Musik hinzufügen…
movie-no-ffmpeg = Zum Erstellen von Filmen wird die freie Software FFmpeg benötigt, die nicht gefunden wurde.
movie-install-ffmpeg = Installieren Sie sie von ffmpeg.org (oder über die Paketverwaltung Ihres Systems) und öffnen Sie dieses Fenster dann erneut.
movie-make = Datei wählen und erstellen…
movie-preview = Vorschau als Diashow

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] Das abgelehnte Foto ({ $size }) in den Papierkorb legen?
   *[other] Die { $count } abgelehnten Fotos ({ $size }) in den Papierkorb legen?
}
trash-explain = Die Dateien werden aus ihren Ordnern entfernt. Sie können sie im Papierkorb des Systems wiederherstellen, solange er nicht geleert wurde.
trash-move = In den Papierkorb legen

print-review-remove = Diese Fotos entfernen
print-review-print = Trotzdem drucken

about-text = Ein Fotoorganizer für den Computer: Durchsuchen Sie Ihre Fotoordner, erstellen Sie Alben, wählen Sie Fotos aus, lehnen Sie sie ab und bewerten Sie sie, korrigieren und bearbeiten Sie sie und starten Sie Diashows.
about-edits = Bearbeitungen verändern Ihre Originaldateien nie; sie werden angewendet, wenn Sie ein Foto ansehen oder exportieren.
about-credits = Integrierte Karte: Natural Earth (gemeinfrei). Städteliste: GeoNames (CC BY 4.0). Online-Karten: © OpenStreetMap-Mitwirkende (ODbL).

online-maps-question = Detaillierte Karten aus dem Internet anzeigen?
online-maps-what = AlbumCraft kann zeigen, wo Ihre Fotos aufgenommen wurden. Mit Online-Karten lädt es Kartenbilder von OpenStreetMap herunter und kann nach Adressen suchen, wenn Sie einen Ort angeben.
online-maps-privacy = Ihre Fotos und deren Orte werden nie gesendet. Die Kartenserver sehen Ihre Internetadresse, die Bereiche, die Sie ansehen, und die Orte, nach denen Sie suchen.
online-maps-offline = Ohne Online-Karten verwendet AlbumCraft seine integrierte Weltkarte und Städteliste und verbindet sich nicht mit dem Internet.
online-maps-later = Sie können das später auf der Karte oder in den Einstellungen (Hauptmenü) unter Karten ändern.
online-maps-use = Online-Karten verwenden
online-maps-builtin = Nur integrierte Karte

## Notices at the bottom of the window

notice-undo = Rückgängig
notice-show = Im Ordner anzeigen
notice-close = Diese Meldung schließen
notice-no-file-manager = Der Dateimanager konnte nicht geöffnet werden: { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } Foto
   *[other] { $n } Fotos
}

settings-bar = Leiste unten in der Bibliothek
settings-bar-compact = Kompakt: eine einzelne Zeile, sichtbar bei ausgewählten Fotos
settings-bar-tray = Ablage: Miniaturen der ausgewählten Fotos und große Schaltflächen
settings-bar-note = In der kompakten Leiste verlieren die Schaltflächen ihre Beschriftung, wenn das Fenster schmal ist. Passen die Schaltflächen nicht hinein, lässt sich die Leiste seitlich scrollen.
settings-thumb-size = Größe der Fotos in der Bibliothek

settings-folders-intro = Die Bibliothek zeigt die Fotos dieser Ordner und ihrer Unterordner dort, wo sie liegen: Nichts wird kopiert oder verschoben. Um Fotos von einer Speicherkarte oder Kamera zu kopieren, verwenden Sie Importieren.
settings-folders-none = Noch keine Ordner: Stattdessen werden Beispielfotos angezeigt.
settings-folders-remove-hint = Diesen Ordner nicht mehr anzeigen (seine Fotos und Ihre Notizen dazu bleiben erhalten)
settings-folders-rescan-hint = Jetzt nach neuen, geänderten und gelöschten Fotos suchen
settings-excluded-title = Left out folders
settings-excluded-intro = Folders inside a library folder that are not shown, with everything under them (for example Screenshots).
settings-excluded-none = Nothing is left out.
settings-excluded-add = Leave out a folder
settings-excluded-remove-hint = Show this folder again

settings-edit-opens = Die Registerkarte Bearbeiten öffnet sich mit
settings-edit-basic = Einfach: Korrekturen mit einem Klick und Werkzeuge
settings-edit-advanced = Erweitert: die Regler
settings-info-panel = Infofeld
settings-info-show = Infofeld im Editor anzeigen (I)

settings-timing = Zeiten
settings-next-after = Nächstes Foto nach
settings-note-paused = Mit der Leertaste pausieren Sie die Diashow, mit ← und → navigieren Sie.
settings-note-manual = Jedes Foto bleibt stehen, bis Sie mit → oder den Schaltflächen zum nächsten wechseln. Der Vollbild-Betrachter (F11) schaltet trotzdem von selbst weiter.
settings-effects = Effekte
settings-fade = Überblendung zwischen den Fotos
settings-motion = Langsamer Schwenk und Zoom
settings-motion-subtle = Dezent
settings-motion-medium = Mittel
settings-captions = Bildunterschriften anzeigen
settings-viewer-note = Der Vollbild-Betrachter (F11 im Editor) startet immer ohne Überblendung und ohne Schwenk und Zoom.
settings-music = Musik
settings-music-play = Während der Diashows Musik abspielen
settings-music-another = Andere wählen…
settings-music-choose = Musik wählen…
settings-movies-note = Aus Fotos erstellte Filme verwenden ebenfalls diese Einstellungen.

settings-export-note = Womit die Exportansicht startet.
settings-export-size = Größe der Fotos
settings-export-quality = JPEG-Qualität
settings-export-info = Angaben in den Kopien
settings-export-gps = Aufnahmeort (GPS) einbeziehen
settings-export-gps-hint = Schalten Sie das aus, bevor Sie Fotos weitergeben, wenn andere den Ort nicht sehen sollen.
settings-export-exif = Kameradaten (EXIF) einbeziehen
settings-export-exif-hint = Kamera, Objektiv, Belichtung und Aufnahmedatum, aus der Originaldatei übernommen. Der Ort folgt der Einstellung oben.
settings-creator = Urheber
settings-creator-hint = Ihr Name
settings-copyright = Copyright
settings-copyright-hint = © Ihr Name
settings-credit-note = In jede Kopie geschrieben (EXIF), anstelle der Angaben des Originals. Per E-Mail gesendete Kopien folgen ebenfalls diesen Einstellungen. Signatur oder Logo wählen Sie in der Exportansicht.

settings-maps-online = Online-Karten
settings-maps-use = Online-Karten verwenden (OpenStreetMap)
settings-maps-note = Detaillierte Karten und Adresssuche. Ihre Fotos und deren Orte werden nie gesendet; die Kartenserver sehen Ihre Internetadresse, die Bereiche, die Sie ansehen, und die Orte, nach denen Sie suchen. Ausgeschaltet: nur die Weltkarte und die Städteliste des Programms, keine Verbindung zum Internet.
settings-maps-tiles = Heruntergeladene Kartenkacheln
settings-maps-tiles-after = Kartenkacheln werden bei Bedarf erneut heruntergeladen.

settings-notes = Ihre Notizen
settings-notes-note = Notizen, Bildunterschriften, Stichwörter, Bearbeitungen und Alben werden in einer einzigen Datei gespeichert; Ihre Fotodateien werden nie verändert.
settings-thumbs = Miniaturen
settings-thumbs-after = Miniaturen werden bei Bedarf aus den Fotos neu erstellt (eine Zeit lang langsamer).
cache-on-disk = { $size } auf der Festplatte
cache-measuring = Wird gemessen…
cache-empty = Leeren

settings-ai-soon = Noch nicht verfügbar. Geplant:
settings-ai-faces = Gesichter
settings-ai-faces-text = Gesichter erkennen und nach Person gruppieren; die Gruppen benennen Sie selbst. Ein Download von etwa 10 bis 40 MB.
settings-ai-search = Intelligente Suche
settings-ai-search-text = Fotos finden, indem Sie sie beschreiben („Schnee“, „Geburtstagskuchen“). Leicht (Englisch, etwa 100 MB) oder mehrsprachig (etwa 400 MB).
settings-ai-tags = Stichwortvorschläge
settings-ai-tags-text = Stichwörter, die anhand des Fotoinhalts vorgeschlagen werden.
settings-ai-local = Alles wird auf diesem Computer berechnet; nichts wird irgendwohin gesendet.

export-size-original = Originalgröße
export-size-very-large = Sehr groß (2560 Pixel)
export-size-large = Groß (1600 Pixel)
export-size-small = Klein, für E-Mail (800 Pixel)

## Export view (src/export_view.rs)

filter-pictures = Fotos
view-back-editor = Editor
view-back-library = Bibliothek
view-back-hint = Zurück (Esc)

export-title = Export
export-count-all = { $n ->
    [one] { $n } Foto
   *[other] { $n } Fotos
}
export-count-some = { $n } von { $total } Fotos
export-select-first = Wählen Sie zuerst Fotos aus.
export-logo-unreadable = Das Logo { $path } ist nicht lesbar.
export-task = { $n ->
    [one] Export von { $n } Foto
   *[other] Export von { $n } Fotos
}
export-save-into = Speichern in
export-choose-folder = Ordner wählen…
export-choose-folder-hint = Ordner für die Kopien wählen
export-safe = Nichts wird überschrieben; die Originale bleiben unverändert.
export-go = { $n ->
    [one] { $n } Foto exportieren
   *[other] { $n } Fotos exportieren
}
export-dialog-title = Fotos exportieren nach…

export-size = Größe
export-size-other = Andere:
export-size-pixels = { " " }Pixel
export-size-note = Die längste Seite. Fotos werden nie vergrößert.
export-quality = JPEG-Qualität

export-signature = Signatur
export-signature-on = Signatur oder Logo hinzufügen
export-sig-text = Text
export-sig-logo = Bild (Logo)
export-logo-other = Anderes Bild…
export-logo-choose = Bild wählen…
export-logo-dialog = Bild für Logo oder Signatur
export-logo-note = Ein PNG mit transparentem Hintergrund liefert das beste Ergebnis.
export-sig-text-hint = Ihr Name oder © Ihr Name
export-sig-position = Position
export-sig-size = Größe
export-sig-size-hint = Im Verhältnis zur kürzeren Seite des Fotos
export-sig-opacity = Deckkraft
export-sig-colour = Farbe
export-sig-shadow = Schatten
export-sig-shadow-hint = Ein leichter Schein, damit der Text auf jedem Foto lesbar bleibt

export-info = Angaben in den Dateien
export-gps = Aufnahmeort (GPS)
export-gps-hint = Schalten Sie das aus, bevor Sie Fotos weitergeben, wenn andere den Ort nicht sehen sollen.
export-exif = Kameradaten (EXIF)
export-exif-hint = Kamera, Objektiv, Belichtung und Aufnahmedatum, aus der Originaldatei übernommen.
export-creator = Urheber
export-creator-hint = Ihr Name
export-copyright = Copyright
export-copyright-hint = © Ihr Name
export-credit-note = In jede Kopie geschrieben. Signatur, Urheber und Copyright werden für das nächste Mal gemerkt (auch in den Einstellungen unter Export).

export-left-out = Ausgelassen: Klicken, um es mit zu exportieren
export-leave-out = Klicken, um dieses Foto auszulassen

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = Alle Fotos
nav-picks = Favoriten
nav-places = Orte
nav-places-hint = Ihre Fotos auf einer Karte
nav-albums = Alben
nav-people = Personen
nav-folders = Ordner
people-hint = Personen legen Sie über die Auswahlleiste fest.
scanning = Fotos werden gesucht…

bar-import = Importieren
bar-import-hint = Fotos von einer Speicherkarte, Kamera oder aus einem Ordner in Ihre Bibliothek kopieren
bar-folders = Ordner
bar-folders-hint = Die in der Bibliothek angezeigten Ordner, dort wo sie liegen (nichts wird kopiert)
bar-size-hint = Größe der Fotos
bar-picks-hint = Nur Favoriten anzeigen
bar-hide-rejects = Abgelehnte Fotos ausblenden

sort-name = Dateiname
sort-taken = Aufnahmedatum
sort-rating = Bewertung
sort-hint = Reihenfolge der Fotos: { $order }

filter-any = Alle
filter-any-no-rejects = Alle, ohne abgelehnte
filter-min = { $n }+
filter-min-no-rejects = { $n }+, ohne abgelehnte
filter-hint = Nach Bewertung filtern; abgelehnte Fotos ausblenden
filter-any-rating = Alle Bewertungen
filter-or-more = { $rating } oder mehr

album-rename = Umbenennen…
album-delete = Album löschen
album-show-map = Auf der Karte anzeigen
album-slideshow = Als Diashow abspielen
album-deleted = Das Album „{ $name }“ wurde gelöscht. Seine Fotos sind unverändert.

empty-scanning = Fotos werden gesucht…
empty-no-match = Kein Foto passt zu Ihrer Suche oder Ihrem Filter.
empty-album = Dieses Album ist leer. Wählen Sie Fotos aus und fügen Sie sie über die Auswahlleiste hinzu.
empty-person = Noch niemand ist markiert.
empty-picks = Noch keine Favoriten. Fahren Sie mit der Maus über ein Foto und klicken Sie auf die Flagge, oder drücken Sie P.
empty-unreachable = In den Ordnern Ihrer Bibliothek wurden keine Fotos gefunden. Liegen sie auf einem nicht angeschlossenen Laufwerk, schließen Sie es an und durchsuchen Sie erneut.
empty-library = Ihre Bibliothek ist leer. Importieren Sie Fotos oder fügen Sie einen vorhandenen Ordner hinzu.
empty-folders-hint = Die in der Bibliothek angezeigten Ordner
empty-import = Fotos importieren
empty-import-hint = Fotos von einer Speicherkarte, Kamera oder aus einem Ordner kopieren
empty-add = Ordner hinzufügen
empty-add-hint = Die Fotos eines Ordners dort anzeigen, wo sie liegen, ohne sie zu kopieren

folder-play-hint = Diesen Ordner als Diashow abspielen
folder-edit-hint = Beschreibung ändern
folder-map-hint = Diesen Ordner auf der Karte anzeigen
tile-pick-hint = Als Favorit markieren (P)
tile-reject-hint = Ablehnen (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = Keine Bewertung
rating-stars = { $n ->
    [one] { $n } Stern
   *[other] { $n } Sterne
}
flag-pick = Als Favorit markieren
flag-unpick = Favorit entfernen
flag-reject = Ablehnen
flag-unreject = Ablehnung aufheben
flag-rating = Bewertung
album-new = Neues Album…
menu-copy-edits = Bearbeitungen kopieren
menu-paste-edits = Bearbeitungen einfügen
menu-paste-n = { $n ->
    [one] { $n } Bearbeitung einfügen
   *[other] { $n } Bearbeitungen einfügen
}
copy-edits-select-one = Wählen Sie genau ein Foto aus, um seine Bearbeitungen zu kopieren.
menu-import = Fotos importieren…
menu-add-folder = Ordner zur Bibliothek hinzufügen…
menu-folders = Ordner der Bibliothek…
menu-scan = Ordner erneut durchsuchen
menu-select-all = Alles auswählen
menu-select-none = Auswahl aufheben
menu-rotate-left = Nach links drehen
menu-rotate-right = Nach rechts drehen
menu-trash-rejects = Abgelehnte Fotos in den Papierkorb legen…
menu-trash-rejects-hint = Die Dateien abgelehnter Fotos nach Bestätigung aus ihren Ordnern entfernen
menu-contact-sheet = Kontaktbogen…
menu-contact-sheet-hint = Seiten mit kleinen Fotos, Titel und Dateinamen, zum Drucken oder Speichern als PDF
menu-undo-edits = Alle Bearbeitungen rückgängig machen
menu-settings = Einstellungen…
menu-about = Über AlbumCraft…
menu-exit = Beenden
edits-removed = { $n ->
    [one] Die Bearbeitungen von { $n } Foto wurden entfernt.
   *[other] Die Bearbeitungen von { $n } Fotos wurden entfernt.
}

status-library = Bibliothek: { $folders ->
    [one] { $folders } Ordner
   *[other] { $folders } Ordner
}, { $pictures ->
    [one] { $pictures } Foto
   *[other] { $pictures } Fotos
}, { $size } auf der Festplatte
status-importing = Import: { $done } von { $total }…
status-stop = Anhalten
status-stop-hint = Nach dem aktuellen Foto anhalten (bereits kopierte bleiben)
task-running = { $label }…
task-stopping = { $label }: wird angehalten…
task-cancel-hint = Nach dem aktuellen Foto anhalten
status-samples = Beispielfotos werden angezeigt. Mit Importieren oder Ordner fügen Sie Ihre eigenen hinzu.

selected-count = { $n ->
    [one] { $n } Foto
   *[other] { $count } Fotos
}
selected-all = { $n ->
    [one] { $pictures } ausgewählt
   *[other] { $pictures } ausgewählt
}
selected-set = { $name } · { $pictures }
fallback-folder = Ordner
fallback-album = Album
tray-nothing = Keine Auswahl
tray-click = Klicken Sie auf Fotos, um sie auszuwählen

output-web = Webseite
output-web-tip = Eine Web-Galerie mit diesen Fotos erstellen
output-mail = E-Mail
output-mail-tip = Kleine Kopien, angehängt an eine neue Nachricht in Ihrem E-Mail-Programm
output-print = Drucken
output-print-tip = Diese Fotos drucken (Strg+P)
output-export = Exportieren
output-export-tip = Bearbeitete Kopien in einem Ordner speichern
output-collage = Collage
output-collage-tip = Eine Collage oder einen Kontaktbogen erstellen
output-movie = Film
output-movie-tip = Einen Film erstellen

bar-clear-hint = Auswahl aufheben (Esc)
bar-pick-hint = Als Favorit markieren / Favorit entfernen (P)
bar-reject-hint = Ablehnen / Ablehnung aufheben (X)
bar-rotate-left = Nach links drehen (Strg+[)
bar-rotate-right = Nach rechts drehen (Strg+])
bar-tags = Stichwörter und Personen
bar-location = Ort festlegen: wo diese Fotos aufgenommen wurden, auf einer Karte
bar-to-album = In ein Album
bar-clear = Aufheben

ctx-open = Im Editor öffnen
ctx-slideshow = Diashow ab hier abspielen
ctx-add-to-album = Zu einem Album hinzufügen
ctx-remove-from-album = Aus dem Album entfernen
ctx-tags = Stichwörter und Personen…
ctx-location = Ort festlegen…
ctx-export = Exportieren…
ctx-show-file = Im Dateimanager anzeigen

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Lebendig
look-vivid-tip = Satte, kräftige Farben wie beim Diafilm, mit tiefem Schwarz
look-soft-film = Weicher Film
look-soft-film-tip = Zarter Porträtfilm: weicher, warmer Kontrast, schmeichelhaft für die Haut
look-polaroid = Sofortbild
look-polaroid-tip = Verblasster Sofortbildabzug: angehobenes Schwarz, warme Lichter, kühle Schatten
look-cross-process = Cross-Entwicklung
look-cross-process-tip = Gelbe Lichter, blaugrüne Schatten und starker Kontrast
look-lomo = Lomo
look-lomo-tip = Gesättigter Spielzeugkamera-Look mit dunklen Ecken
look-matte = Matt
look-matte-tip = Flaches, verwaschenes Schwarz, gedämpfte Farben
look-teal-orange = Blaugrün und Orange
look-teal-orange-tip = Kino-Farbgebung: blaugrüne Schatten, warme Haut und Lichter
look-high-key = High Key
look-high-key-tip = Hell und luftig, mit offenen Schatten
look-cool = Kühl
look-cool-tip = Ein kühlerer, bläulicherer Farbstich
look-warm = Warm
look-warm-tip = Ein wärmerer, goldener Farbstich
look-bw = S/W
look-bw-tip = Klassisches Schwarzweiß
look-dramatic-bw = Dramatisches S/W
look-dramatic-bw-tip = Schwarzweiß mit Rotfilter: dunkle Himmel, starker lokaler Kontrast
look-sepia = Sepia
look-sepia-tip = Brauner Ton alter Fotos
look-soft-focus = Weichzeichner
look-soft-focus-tip = Scharfe Mitte, weiche, verträumte Ränder
look-glow = Leuchten
look-glow-tip = Ein sanfter Lichtschein
look-film-grain = Filmkorn
look-film-grain-tip = Körnung wie bei Film
look-focal-bw = Fokus-S/W
look-focal-bw-tip = Farbe in der Mitte, ringsum Schwarzweiß
effect-sharpen = Schärfen
effect-sepia = Sepia
effect-bw = S/W
effect-warmify = Wärmer machen
effect-film-grain = Filmkorn
effect-tint = Tönung
effect-saturation = Sättigung
effect-soft-focus = Weichzeichner
effect-glow = Leuchten
effect-filtered-bw = Gefiltertes S/W
effect-focal-bw = Fokus-S/W
effect-graduated-tint = Verlaufstönung
op-rotate-right = Nach rechts drehen
op-rotate-left = Nach links drehen
op-crop = Zuschneiden
op-straighten = Ausrichten { $angle }°
op-perspective-v = Perspektive, vertikal { $v }
op-perspective-h = Perspektive, horizontal { $h }
op-perspective-both = Perspektive { $v } / { $h }
op-redeye = Rote Augen
op-lucky = Auto-Korrektur (Glückstreffer)
op-auto-contrast = Auto-Kontrast
op-auto-color = Auto-Farbe
op-tuning = Feinabstimmung
op-look = { $look } { $amount } %
op-adjust = Anpassung
op-adjust-parts = Anpassung: { $parts }
adj-wb = Weißabgleich
adj-exposure = Belichtung
adj-highlights = Lichter
adj-shadows = Schatten
adj-blacks = Schwarz
adj-contrast = Kontrast
adj-saturation = Sättigung
adj-vibrance = Dynamik
adj-clarity = Klarheit
adj-sharpen = Schärfe
adj-noise = Rauschminderung
decor-vignette-light = Helle Vignette
decor-vignette = Vignette
decor-instant = Sofortbildrahmen
decor-border = Rand
decor-shadow = Schlagschatten
decor-rounded = Abgerundete Ecken
decor-text = Text „{ $text }“
wb-as-shot = Wie aufgenommen
wb-daylight = Tageslicht
wb-cloudy = Bewölkt
wb-shade = Schatten
wb-tungsten = Kunstlicht
wb-fluorescent = Leuchtstofflampe
wb-flash = Blitz
wb-custom = Benutzerdefiniert
wb-auto = Auto
gallery-style-light = Helles Raster
gallery-style-dark = Dunkles Raster
gallery-style-masonry = Mosaik (ganze Fotos)
crop-free = Frei
crop-original = Originalformat
crop-square = Quadrat
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (Abzug 10 × 15)
crop-16-9 = 16 × 9 (Breitbild)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Doppelklick zum Zurücksetzen
edit-loading = Das Foto wird noch geladen.
edit-straightened = Um { $angle }° ausgerichtet.
edit-already-level = Dieses Foto scheint bereits gerade zu sein.
edit-no-horizon = Kein klarer Horizont und keine klare senkrechte Linie zum Ausrichten gefunden.
edit-histogram = Histogramm und Kamerainfos
edit-unknown-camera = Unbekannte Kamera
edit-35mm = (Kleinbild-äquivalent: { $mm } mm)
edit-focal = Brennweite: { $mm } mm
edit-iso = ISO: { $iso }
edit-no-camera = Keine Kamerainformationen in diesem Foto
edit-clip-shadows = Schatten { $pct } %
edit-clip-highlights = Lichter { $pct } %
edit-clipped = Beschneidung: { $parts }
edit-clipped-hint = Anteil des Fotos, der in mindestens einer Farbe reines Schwarz oder reines Weiß ist
edit-back = Zurück zur Bibliothek
edit-info = Infos
edit-info-hint = Eigenschaften, Ort und Stichwörter (I)
edit-side-by-side-hint = Die Fotos nebeneinander anzeigen
edit-stacked-hint = Die Fotos übereinander anzeigen
edit-focus-hint = Wechseln, welches Foto bearbeitet wird (dort landen Ihre Änderungen)
edit-compare = Vergleichen
edit-compare-hint = Dieses Foto neben einem anderen, nebeneinander
edit-original = Original
edit-original-hint = Gedrückt halten, um das Foto ohne Ihre Bearbeitungen zu sehen (oder O gedrückt halten)
edit-slideshow = Diashow
edit-fullscreen-hint = Vollbild (F11): ab diesem Foto eines nach dem anderen, mit Musik
edit-next = Nächstes Foto
edit-previous = Vorheriges Foto
edit-position = { $n } von { $total }
edit-tab-edit = Bearbeiten
edit-tab-presets = Voreinstellungen
edit-tab-effects = Effekte
edit-undo = Rückgängig
edit-undo-hint = Letzte Änderung rückgängig machen (Strg+Z)
edit-redo = Wiederholen
edit-list = Bearbeitungen dieses Fotos:
edit-list-none = (keine – es ist das Original)
edit-step-hint =
    { $label }
    Klicken, um zu diesem Schritt zurückzukehren (die folgenden werden entfernt; Rückgängig holt sie zurück)
edit-copy-hint = Diese Bearbeitungen kopieren, um sie in andere Fotos einzufügen (Strg+Umschalt+C)
edit-paste-n = { $n } einfügen
edit-paste-hint = Die kopierten Bearbeitungen zu diesem Foto hinzufügen (Strg+Umschalt+V)
edit-basic = Einfach
edit-advanced = Erweitert
edit-wb-help = Klicken Sie auf eine Stelle im Foto, die neutral grau oder weiß sein sollte.
edit-crop-help = Ziehen Sie die Kanten oder Ecken des Rahmens, ziehen Sie innen, um ihn zu verschieben, oder zeichnen Sie einen neuen Rahmen. Die Eingabetaste wendet an.
edit-crop-shape = Format
edit-crop-orientation = Ausrichtung
edit-landscape = Querformat
edit-landscape-hint = Breiter als hoch
edit-portrait = Hochformat
edit-portrait-hint = Höher als breit
edit-apply = Anwenden
edit-remove-crop = Zuschnitt entfernen
edit-remove-crop-hint = Wieder das ganze Foto anzeigen
edit-redeye-help = Ziehen Sie einen Rahmen um die roten Augen. Nur die roten Pupillen innerhalb des Rahmens werden abgedunkelt.
edit-straighten-title = Ausrichten
edit-straighten-help = Bewegen Sie den Regler, bis der Horizont am Raster ausgerichtet ist.
edit-oneclick = Korrekturen mit einem Klick
edit-lucky = Auf gut Glück
edit-lucky-hint = Belichtung, Weißabgleich, Schwarzpunkt und etwas Dynamik auf einmal (danach unter Erweitert verfeinern)
edit-auto-exposure = Auto-Belichtung
edit-auto-exposure-hint = Aufhellen oder abdunkeln für eine ausgewogene Belichtung
edit-auto-colour = Auto-Farbe
edit-auto-colour-hint = Automatischer Weißabgleich: einen Farbstich entfernen
edit-auto-contrast = Auto-Kontrast
edit-auto-contrast-hint = Die dunkelsten und hellsten Töne strecken
edit-auto-straighten = Auto-Ausrichtung
edit-auto-straighten-hint = Einen schiefen Horizont oder schiefe senkrechte Linien ausrichten
edit-tools = Werkzeuge
edit-pick-neutral = Neutralpunkt
edit-pick-neutral-hint = Auf eine graue Stelle klicken, um den Weißabgleich einzustellen
edit-sec-wb = WEISSABGLEICH
edit-sec-light = LICHT
edit-sec-colour = FARBE
edit-sec-detail = DETAIL
edit-sec-geometry = GEOMETRIE
edit-wb-pick = Wählen
edit-wb-pick-hint = Auf eine graue Stelle im Foto klicken
edit-s-temperature = Temperatur
edit-s-temperature-tip = Bläulicher (links) oder wärmer (rechts)
edit-s-tint = Tönung
edit-s-tint-tip = Grüner (links) oder magentafarbener (rechts)
edit-s-exposure = Belichtung
edit-s-exposure-tip = Gesamthelligkeit, in Blendenstufen
edit-s-highlights = Lichter
edit-s-highlights-tip = Helle Bereiche: links holt Zeichnung zurück, rechts hellt auf
edit-s-shadows = Schatten
edit-s-shadows-tip = Dunkle Bereiche: rechts öffnet sie
edit-s-blacks = Schwarz
edit-s-blacks-tip = Schwarzpunkt: links vertieft, rechts hebt an
edit-s-contrast = Kontrast
edit-s-contrast-tip = Abstand zwischen Dunkel und Hell
edit-s-saturation = Sättigung
edit-s-saturation-tip = Alle Farben
edit-s-vibrance = Dynamik
edit-s-vibrance-tip = Vor allem matte Farben; schont Haut und bereits kräftige Farben
edit-s-clarity = Klarheit
edit-s-clarity-tip = Lokaler Kontrast in den Mitteltönen
edit-s-sharpen = Schärfe
edit-s-sharpen-tip = Feine Details schärfer
edit-s-noise = Rauschen
edit-s-noise-tip = Rauschminderung: glättet das Korn und behält die Kanten
edit-reset-all = Alles zurücksetzen
edit-crop-btn = Zuschneiden…
edit-crop-btn-hint = Den Teil des Fotos wählen, der bleibt, oder den Zuschnitt ändern
edit-s-straighten-tip = Das Foto leicht drehen, um den Horizont auszurichten (zoomt, um die Ecken zu verbergen)
edit-s-vertical = Vertikal
edit-s-vertical-tip = Perspektive: rechts verbreitert die Oberseite, um stürzende Linien bei Aufnahmen von unten auszugleichen
edit-s-horizontal = Horizontal
edit-s-horizontal-tip = Perspektive: rechts vergrößert die rechte Seite, für eine schräg gesehene Wand
edit-intensity = Stärke
edit-remove-preset = Voreinstellung entfernen
edit-presets-help = Looks für das ganze Foto. Klicken Sie auf einen und stellen Sie dann seine Stärke ein.
edit-effects-add = Auf dem Foto oder um das Foto hinzufügen
edit-add-instant = Sofortbildrahmen
edit-add-text = Text
edit-your-text = Ihr Text
edit-effects-note = Effekte werden über allen anderen Bearbeitungen angewendet. Fügen Sie einen hinzu, um ihn hier einzustellen.
edit-p-amount = Stärke
edit-p-size = Größe
edit-p-width = Breite
edit-p-bottom = Unten
edit-p-colour = Farbe
edit-p-darkness = Dunkelheit
edit-p-background = Hintergrund
edit-p-radius = Radius
edit-vignette-note = Links von der Mitte werden stattdessen die Ecken aufgehellt.
edit-halo = Leichter Schein (auf jedem Foto lesbar)
edit-remove-effect = Diesen Effekt entfernen
edit-caption-hint = Fügen Sie eine Bildunterschrift hinzu!
edit-print-hint = Dieses Foto drucken (Strg+P)
edit-album = Album
edit-zoom-hint = Zoom (100 % = die tatsächlichen Pixel des Fotos)
edit-zoom-slider-hint = Zoom: links = einpassen, Mitte = 1:1, rechts = 400 % (das Mausrad funktioniert auch; ziehen Sie das Foto, um bei 100 % zu zoomen und es zu verschieben)
edit-one-to-one-hint = Originalgröße (100 %): ein Pixel des Fotos pro Bildschirmpixel
edit-fit = Einpassen
edit-fit-hint = Das ganze Foto ins Fenster einpassen (ein Doppelklick auf ein gezoomtes Foto tut dasselbe)
edit-people = Personen:
edit-person-remove = Person entfernen
edit-person-add = Person hinzufügen
edit-compare-none = Kein weiteres Foto in diesem Ordner
edit-compare-selected = Ausgewählt
edit-compare-swap-hint = Klicken, um stattdessen dieses Foto zu bearbeiten
edit-view-loading = Wird geladen…
edit-sharpening = Schärfen läuft…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = Dieses Foto ist nicht lesbar.
edit-full-unreadable = Das vollständige Foto ist nicht lesbar.
edit-full-loading = Das vollständige Foto wird geladen…
edit-draw-box = Ziehen Sie über das Foto, um einen Rahmen aufzuziehen
edit-click-neutral = Klicken Sie auf eine Stelle, die grau oder weiß sein sollte
edit-wb-title = Weißabgleich

## Info panel, search, slideshow bar

info-hide-hint = Dieses Feld ausblenden (I)
info-properties = Eigenschaften
info-camera = Kamera
info-lens = Objektiv
info-taken = Aufgenommen am
info-file-date = { $date } (Dateidatum)
info-size = Größe
info-megapixels = { $w } × { $h } ({ $mp } MP)
info-file = Datei
info-focal = Brennweite
info-focal-35 = { $f } mm (Kleinbild: { $e } mm)
info-aperture = Blende
info-exposure = Belichtung
info-folder = Ordner
info-sample = Integriertes Beispielfoto
info-edits = Bearbeitungen
info-no-exif = Keine Kamerainformationen (EXIF) in diesem Foto.
info-exif = Erweiterte EXIF-Daten
info-flash = Blitz
info-bias = Belichtungskorrektur
info-program = Programm
info-metering = Messung
info-distance = Entfernung
info-colour-space = Farbraum
info-gps = GPS-Position
info-digitized = Digitalisiert
info-modified = Geändert
info-software = Software
info-preview = Kameravorschau
info-preview-hint = { $w } × { $h } Pixel, in der Datei gespeichert
info-all-exif = Alle EXIF-Daten ({ $n } Einträge)
info-location = Ort
info-set-in-app = In AlbumCraft festgelegt
info-from-gps = Vom GPS der Kamera
info-modify = Ändern…
info-modify-hint = Die Nadel verschieben, den Ort umbenennen oder entfernen
info-show-map-hint = Orte mit diesem Foto öffnen
info-no-location = Noch kein Ort.
info-set-location-hint = Den Ort suchen oder auf einer Karte anklicken
info-tags = Stichwörter
info-no-tags = Noch keine Stichwörter.
info-remove-tag = Dieses Stichwort entfernen
info-add-tag = Stichwort hinzufügen…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = O
coord-w = W
info-orient-2 = Gespiegelt
info-orient-3 = Auf dem Kopf
info-orient-4 = Gespiegelt, auf dem Kopf
info-orient-5 = Gespiegelt, nach links gedreht
info-orient-6 = Nach rechts gedreht (Kamera aufrecht gehalten)
info-orient-7 = Gespiegelt, nach rechts gedreht
info-orient-8 = Nach links gedreht (Kamera aufrecht gehalten)
search-remove-hint = Aus der Suche entfernen
search-hint = Stichwörter, Orte, Bildunterschriften suchen…
search-and = und…
search-clear-hint = Suche löschen
search-for = Nach „{ $text }“ suchen
search-kind-tags = Stichwörter
music-dialog-title = Musik für die Diashow
filter-music = Musik
ss-previous = Zurück (←)
ss-play-pause = Wiedergabe / Pause (Leertaste)
ss-next = Weiter (→)
ss-seconds = Sekunden pro Foto
ss-interval-off = Aus: Die Fotos wechseln nur mit → oder den Schaltflächen (Einstellungen, Diashow)
ss-fade = Überblendung
ss-motion = Bewegung
ss-other-music-hint = Eine andere Musikdatei wählen
ss-exit-full = Vollbild beenden (F11 oder Esc)
ss-exit = Beenden (Esc)

## Messages of the window (src/app.rs)

save-failed-prefix = Ihre Bibliothek konnte nicht gespeichert werden:
save-failed =
    { $prefix } { $error }
    Ihre letzten Änderungen sind vorerst nur in diesem Fenster; AlbumCraft versucht es weiter.
save-ok-again = Ihre Bibliothek wird wieder gespeichert.
lib-unreadable-kept =
    Die Datei Ihrer Bibliothek konnte nicht gelesen werden ({ $error }).

    Sie wurde aufbewahrt unter:
    { $backup }

    AlbumCraft wurde mit einer leeren Bibliothek gestartet. Ihre Fotos selbst sind nicht betroffen.
lib-unreadable-nosave =
    Die Datei Ihrer Bibliothek konnte nicht gelesen werden ({ $error }).

    AlbumCraft wurde mit einer leeren Bibliothek gestartet und speichert diesmal keine Änderungen, damit die Datei nicht überschrieben wird.
dialog-library = Bibliothek
crash-title = AlbumCraft wurde unerwartet beendet
crash-text =
    AlbumCraft wurde beim letzten Mal wegen eines Fehlers beendet. Änderungen aus den allerletzten Sekunden fehlen möglicherweise.

    Die Einzelheiten wurden geschrieben nach:
    { $log }

    Bitte fügen Sie diese Datei bei, wenn Sie das Problem melden.
picasa-imported =
    { $n ->
        [one] Notizen, Bildunterschriften und Bearbeitungen aus der .picasa.ini-Datei von { $n } Ordner importiert.
       *[other] Notizen, Bildunterschriften und Bearbeitungen aus den .picasa.ini-Dateien von { $n } Ordnern importiert.
    }
copy-no-edits = Dieses Foto hat keine Bearbeitungen zum Kopieren.
copied-edits =
    { $n ->
        [one] { $n } Bearbeitung kopiert. Wählen Sie Fotos aus und fügen Sie sie ein (Strg+Umschalt+V).
       *[other] { $n } Bearbeitungen kopiert. Wählen Sie Fotos aus und fügen Sie sie ein (Strg+Umschalt+V).
    }
paste-first = Kopieren Sie zuerst die Bearbeitungen eines Fotos (Strg+Umschalt+C).
pasted-edits =
    { $n ->
        [one] Bearbeitungen in { $n } Foto eingefügt.
       *[other] Bearbeitungen in { $n } Fotos eingefügt.
    }
no-pictures-to-show = Es gibt keine Fotos zum Anzeigen.
music-error = Keine Musik: { $error }
choose-folder-dialog = Wählen Sie einen Fotoordner
added-to-album =
    { $n ->
        [one] { $n } Foto zu „{ $name }“ hinzugefügt.
       *[other] { $n } Fotos zu „{ $name }“ hinzugefügt.
    }
trash-samples = Beispielfotos können nicht in den Papierkorb gelegt werden.
trash-no-rejects = Es ist kein Foto abgelehnt. Drücken Sie X, um eines abzulehnen.
task-trash =
    { $n ->
        [one] { $n } Foto wird in den Papierkorb gelegt
       *[other] { $n } Fotos werden in den Papierkorb gelegt
    }
task-email =
    { $n ->
        [one] { $n } Foto wird für eine E-Mail vorbereitet
       *[other] { $n } Fotos werden für eine E-Mail vorbereitet
    }
default-title = Meine Fotos
gallery-folder-dialog = Web-Galerie erstellen in…
task-gallery =
    { $n ->
        [one] Web-Galerie mit { $n } Foto wird erstellt
       *[other] Web-Galerie mit { $n } Fotos wird erstellt
    }
movie-save-dialog = Film speichern unter…
filter-mp4 = MP4-Video
task-movie = Film wird erstellt
scan-truncated = Nur die ersten { $n } Fotos werden angezeigt: Entfernen Sie Ordner aus der Bibliothek, um die übrigen zu sehen.
selection-none = Keine Auswahl
selection-one = { $name }     { $date }     { $w }x{ $h } Pixel     { $size }
selection-several = { $pictures }     { $a }      { $size } auf der Festplatte
selection-range = { $pictures }     vom { $a } bis { $b }     { $size } auf der Festplatte

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Fotohaufen
cstyle-pile-tip = Verstreute Fotos, wie auf einen Tisch geworfene Abzüge
cstyle-mosaic = Mosaik
cstyle-mosaic-tip = Fotos in verschiedenen Größen füllen die Seite
cstyle-frame = Foto in der Mitte
cstyle-frame-tip = Ein Mosaik um ein großes Foto in der Mitte
cstyle-grid = Raster
cstyle-grid-tip = Gleichmäßige Zeilen und Spalten
cstyle-film = Filmstreifen
cstyle-film-tip = Fotos auf Filmstreifen
cstyle-clothesline = Wäscheleine
cstyle-clothesline-tip = Sofortbilder, an Leinen geklammert
cstyle-fan = Fächer
cstyle-fan-tip = Ausgebreitet wie ein Kartenspiel
cstyle-shape = Form
cstyle-shape-tip = Ein Herz, ein Kreis oder ein Stern aus Fotos
cstyle-blend = Mehrfachbelichtung
cstyle-blend-tip = Die Fotos scheinen durcheinander hindurch (ideal mit 2 bis 4)
cstyle-sheet = Kontaktbogen
cstyle-sheet-tip = Seiten mit kleinen Fotos, Titel und Dateinamen
shape-heart = Herz
shape-circle = Kreis
shape-star = Stern
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = US Letter
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Quadrat
fmt-wide = Bildschirm (16:9)
grad-vertical = Von oben nach unten
grad-diagonal = Diagonal
grad-radial = Von der Mitte aus
collage-need-sheet = Wählen Sie zuerst die Fotos für den Kontaktbogen aus.
collage-need-two = Wählen Sie mindestens zwei Fotos aus, um eine Collage zu erstellen.
collage-unreadable = Dieses Foto konnte nicht gelesen werden.
collage-new-layout = Neue Anordnung
collage-new-layout-hint = Andere Positionen, Drehungen und Größen
collage-shuffle = Mischen
collage-shuffle-hint = Die Fotos in zufällige Reihenfolge bringen
collage-spacing = Abstand zwischen den Fotos
border-none = Keiner
border-white = Weiß
border-instant = Sofortbild
collage-frame-text = Text auf den Rahmen
cap-none = Keine
cap-caption = Bildunterschrift
collage-shadows = Schatten
bg-gradient = Verlauf
bg-picture = Bild
collage-to = bis
collage-use-selected = Ausgewähltes Foto verwenden
collage-use-selected-hint = Klicken Sie zuerst auf ein Foto der Collage
collage-choose-file = Datei wählen…
collage-no-bg = Noch kein Bild gewählt
collage-bg-dialog = Hintergrundbild
collage-page = Seite
collage-size = Bildgröße
collage-px = { $px } Pixel
collage-centre = In die Mitte setzen
collage-put-back = Zurücksetzen
collage-put-back-hint = Die Verschiebungen dieses Fotos rückgängig machen
collage-leave-out = Auslassen
collage-leave-out-hint = Dieses Foto aus der Collage nehmen (Entf)
collage-title = Titel
collage-no-title = Ohne Titel
collage-no-subtitle = Keine zweite Zeile
collage-header = Titel auf der ersten Seite
collage-under = Unter jedem Foto
collage-date = Datum
collage-pictures = Fotos
collage-page-numbers = Seitenzahlen
collage-sending = Seite { $page } von { $total } wird an den Drucker gesendet…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } Seite
       *[other] { $pages } Seiten
    }
collage-no-printer = Kein Drucker verfügbar
collage-save-pdf = Als PDF speichern…
collage-save-picture = Bild speichern…
collage-save-picture-hint = Die Collage als JPEG- oder PNG-Bild speichern
collage-file-sheet = Kontaktbogen
collage-file-collage = Collage
collage-save-pdf-title = Als PDF speichern
collage-save-picture-title = Collage als Bild speichern
filter-pdf = PDF-Dokument
filter-jpeg = JPEG-Bild
filter-png = PNG-Bild
collage-saving-sheet = Kontaktbogen wird gespeichert…
collage-saving = Collage wird gespeichert…
task-save-collage = Collage wird gespeichert
collage-printed =
    { $n ->
        [one] { $n } Seite an { $printer } gesendet.
       *[other] { $n } Seiten an { $printer } gesendet.
    }
collage-print-failed = Der Druck wurde nicht abgeschlossen: { $error }
collage-hint-swap = Ziehen Sie ein Foto auf ein anderes, um sie zu tauschen. Klicken Sie auf ein Foto, um es auszuwählen (Entf lässt es aus).
collage-hint-free = Ziehen Sie ein Foto, um es zu verschieben, scrollen Sie darüber, um es zu drehen. Klicken Sie auf ein Foto, um es auszuwählen (Entf lässt es aus).
collage-page-of = Seite { $page } von { $total }

## Places map (src/map_view.rs)

map-search-still = Die GPS-Positionen Ihrer Fotos werden noch gesucht: Versuchen Sie es gleich noch einmal.
map-none-located =
    { $n ->
        [one] Dieses Foto hat noch keinen Ort. Wählen Sie es aus und klicken Sie in der Auswahlleiste auf { $icon } (Ort festlegen), um es zu platzieren.
       *[other] Keines dieser { $n } Fotos hat bisher einen Ort. Wählen Sie sie aus und klicken Sie in der Auswahlleiste auf { $icon } (Ort festlegen), um sie zu platzieren.
    }
map-location-set =
    { $n ->
        [one] Ort für { $n } Foto festgelegt.
       *[other] Ort für { $n } Fotos festgelegt.
    }
map-location-removed =
    { $n ->
        [one] Ort von { $n } Foto entfernt.
       *[other] Ort von { $n } Fotos entfernt.
    }
map-nothing-online = Online nichts gefunden.
map-set-location = Ort festlegen
map-sub-scope =
    { $n } von { $total ->
        [one] { $total } Foto
       *[other] { $total } Fotos
    } mit Ort
map-sub-indexing = { $n } mit Ort · GPS-Positionen werden gesucht…
map-sub-all =
    { $n ->
        [one] { $n } Foto mit Ort
       *[other] { $n } Fotos mit Ort
    }
map-title-scope = Orte · { $title }
map-show-whole = Ganze Bibliothek anzeigen
map-online = Online-Karte
map-builtin = Integrierte Karte
map-online-tip = OpenStreetMap wird angezeigt. Klicken, um nur die integrierte Karte zu verwenden (keine Verbindung zum Internet).
map-builtin-tip = Die integrierte Weltkarte wird angezeigt. Klicken, um die detaillierten OpenStreetMap-Karten anzuzeigen (lädt Kartenbilder aus dem Internet).
map-show-all = Alle anzeigen
map-show-all-hint = Zoomen, sodass alle Fotos auf der Karte zu sehen sind
map-search-online-hint = Einen Ort oder eine Adresse suchen
map-search-builtin-hint = Eine Stadt oder Koordinaten suchen
map-no-city-builtin = Keine Stadt dieses Namens in der integrierten Liste.
map-no-city-hint = Keine Stadt dieses Namens. Versuchen Sie Koordinaten, z. B. 44,84, -0,58.
map-cities = STÄDTE
map-cities-builtin = STÄDTE (INTEGRIERTE LISTE)
map-searching = Online-Suche…
map-places-osm = ORTE UND ADRESSEN (OPENSTREETMAP)
map-search-addresses = Online nach Adressen suchen   (Eingabetaste)
map-where = Wo wurden sie aufgenommen?
map-where-help = Suchen Sie oben einen Ort oder klicken Sie auf die Karte, um eine Nadel zu setzen. Danach können Sie sie ziehen.
map-name-hint = Name des Ortes (optional)
map-drag-pin = Ziehen Sie die Nadel, um sie anzupassen.
map-drag-pin-offline = Ziehen Sie die Nadel, um sie anzupassen. Online-Karten zeigen die Straßen.
map-set-n =
    { $n ->
        [one] Ort für { $n } Foto festlegen
       *[other] Ort für { $n } Fotos festlegen
    }
map-remove-location = Ort entfernen
map-remove-location-hint = Diese Fotos haben dann keinen Ort, auch wenn die Kamera einen gespeichert hat (die Dateien werden nicht verändert)
map-close-hint = Schließen (Esc)
map-change-location = Ort ändern…
map-select-in-library = In der Bibliothek auswählen
map-select-in-library-hint = Diese Fotos in der Bibliothek auswählen
map-show-hint = Auf der Karte anzeigen
map-preparing = Karte wird vorbereitet…
map-hover = { $pictures } · { $place }
map-credit-builtin = Integrierte Karte: Natural Earth · GeoNames
map-problem = { $problem }. Die integrierte Karte wird angezeigt.
map-empty-indexing = GPS-Positionen Ihrer Fotos werden gesucht…
map-empty =
    Noch keine Fotos mit Ort.

    Fotos von Smartphones und GPS-Kameras erscheinen hier von selbst. Um die anderen zu platzieren, wählen Sie sie aus und klicken in der Auswahlleiste auf { $icon } (Ort festlegen).

## Import (src/import_view.rs)

import-subtitle = Fotos in Ihre Bibliothek kopieren
import-from = Von
import-choose = Wählen…
import-refresh-hint = Erneut nach Karten und Laufwerken suchen und den Ordner neu lesen
import-hide-dups = Die { $n } bereits in der Bibliothek ausblenden
import-dups-hint = Fotos mit demselben Inhalt wie ein Foto Ihrer Bibliothek, auch unter anderem Namen
import-uncheck-all = Alle abwählen
import-check-all = Alle anwählen
import-copy-into = Kopieren nach
import-another-folder = Ein anderer Ordner…
import-name-hint = Name des neuen Ordners, z. B. Sommer in Spanien
import-checked = { $chosen } von { $shown } angewählt. Nichts wird überschrieben; die Originale bleiben unverändert.
import-go =
    { $n ->
        [one] { $n } Foto importieren
       *[other] { $n } Fotos importieren
    }
import-dest-dialog = Den neuen Ordner ablegen in…
import-from-dialog = Fotos importieren von…
import-pick-dest = Wählen Sie, wo der neue Ordner liegen soll.
import-running = Ein Import läuft noch: Warten Sie, bis er beendet ist (siehe Statusleiste).
import-started =
    { $n ->
        [one] { $n } Foto wird nach { $path } importiert…
       *[other] { $n } Fotos werden nach { $path } importiert…
    }
import-listing-in = Fotos in { $from } werden gesucht…
import-none-in = Keine Fotos in { $path }.
import-choose-another = Anderen Ordner wählen…
import-heading = Von einer Speicherkarte, Kamera oder einem Ordner importieren
import-copy-note = Die Fotos werden in einen neuen Ordner Ihrer Bibliothek kopiert. Die Originale bleiben unverändert.
import-no-cards = Keine Speicherkarte und keine Kamera gefunden. Schließen Sie eine an und klicken Sie oben auf { $icon }.
import-instead = Um Fotos dort anzuzeigen, wo sie bereits liegen, ohne sie zu kopieren:
import-add-instead = fügen Sie stattdessen einen Ordner zur Bibliothek hinzu
import-all-dups =
    { $n ->
        [one] Das Foto ist bereits in Ihrer Bibliothek.
       *[other] Alle { $n } Fotos sind bereits in Ihrer Bibliothek.
    }
import-show-anyway = Trotzdem anzeigen
import-truncated = Nur die ersten { $n } Fotos werden aufgelistet.
import-group-hint = Diese ganze Gruppe an- oder abwählen
import-in-library = In der Bibliothek
import-click-preview = Klicken Sie auf ein Foto, um es hier zu sehen
import-this = Dieses Foto importieren
import-this-hint = Leertaste
import-already = bereits in Ihrer Bibliothek

## Print (src/print_view.rs)

print-select-first = Wählen Sie zuerst die Fotos aus, die Sie drucken möchten.
print-page-layout = Seitenlayout
print-layout-full = Ganze Seite
print-shrink = Verkleinern, um einzupassen
print-shrink-hint = Das ganze Foto wird gedruckt, mit weißen Rändern, wo die Formate abweichen
print-crop = Beschneiden, um einzupassen
print-crop-hint = Das Foto füllt das Druckformat; überstehende Ränder werden abgeschnitten
print-options = Rand- und Textoptionen
print-border = Rand
print-no-border = Kein Rand
print-thin-line = Dünne schwarze Linie
print-white-border = Weißer Rand
print-text-under = Text unter jedem Foto
print-no-text = Kein Text
print-printer = Drucker
print-no-printer-found = Kein Drucker gefunden
print-setup = Druckereinstellungen…
print-setup-hint = Papierformat, Ausrichtung und druckerspezifische Einstellungen
print-paper = Papier
print-default = Standard des Druckers
print-no-answer = Kein Drucker hat geantwortet – es wird eine A4-Seite angezeigt
print-paper-describe = { $dpi } dpi, { $w } x { $h } Zoll ({ $orientation })
print-copies = Kopien pro Foto
print-copies-n =
    { $n ->
        [one] { $n } Kopie
       *[other] { $n } Kopien
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } Abzug
       *[other] { $prints } Abzüge
    }
print-cancel = Abbrechen  { $icon }
print-go = Drucken  { $icon }
print-review = Prüfen
print-review-hint = Prüfen, ob jedes Foto genug Pixel für seine Druckgröße hat
print-none-left = Es sind keine Fotos mehr zu drucken.
print-job-printed =
    { $n ->
        [one] { $n } Seite an { $printer } gesendet.
       *[other] { $n } Seiten an { $printer } gesendet.
    }
print-preview = Vorschau
print-ready = Alle Ihre Fotos sind druckbereit.
print-too-small = Einige Ihrer Fotos sind zu klein für einen guten Druck.  Sie können diese Fotos entfernen, trotzdem drucken oder abbrechen und das Druckformat ändern.
print-quality-best = Beste Qualität ({ $ppi } Pixel/Zoll)
print-quality-good = Gute Qualität ({ $ppi } Pixel/Zoll)
print-quality-bad = Schlechte Qualität ({ $ppi } Pixel/Zoll)

## Results of background work (src/backend.rs)

backend-no-data-dir = Es wurde kein Ordner für die Programmdaten gefunden.
backend-cache-fail = Der Cache konnte nicht geleert werden ({ $error })
fail-export = Export nicht möglich: unerwarteter Fehler.
fail-trash = Die Fotos konnten nicht in den Papierkorb gelegt werden: unerwarteter Fehler.
fail-email = Die E-Mail konnte nicht vorbereitet werden: unerwarteter Fehler.
fail-gallery = Die Web-Galerie konnte nicht erstellt werden: unerwarteter Fehler.
fail-movie = Der Film konnte nicht erstellt werden: unerwarteter Fehler
fail-collage = Die Collage konnte nicht gespeichert werden: unerwarteter Fehler.
export-nothing = Es wurde nichts exportiert: { $error }.
export-result =
    { $ok } von { $total ->
        [one] { $total } Foto
       *[other] { $total } Fotos
    } exportiert nach { $path }
export-stopped = Der Export wurde vor dem Ende angehalten.
export-failed-list = Nicht exportiert:
export-more = … und { $n } weitere
gallery-ready =
    Die Web-Galerie ist fertig:
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } Foto konnte nicht gelesen werden und wurde ausgelassen.
       *[other] { $n } Fotos konnten nicht gelesen werden und wurden ausgelassen.
    }
gallery-stopped = Die Web-Galerie wurde nicht erstellt: Abbruch angefordert.
gallery-failed = Die Web-Galerie konnte nicht erstellt werden: { $error }
movie-ready =
    Der Film ist fertig:
    { $path }
movie-stopped = Der Film wurde nicht erstellt: Abbruch angefordert.
movie-failed = Der Film konnte nicht erstellt werden: { $error }
collage-saved-many =
    { $n } Bilder gespeichert, das erste ist:
    { $path }
collage-saved =
    Gespeichert:
    { $path }
collage-save-failed = Die Collage konnte nicht gespeichert werden: { $error }
import-unexpected = Der Import wurde unerwartet beendet.
import-summary =
    { $n ->
        [one] { $n } Foto nach { $path } importiert.
       *[other] { $n } Fotos nach { $path } importiert.
    }
import-already-there =
    { $n ->
        [one] { $n } war schon vorhanden.
       *[other] { $n } waren schon vorhanden.
    }
import-was-stopped = Der Import wurde vor dem Ende angehalten.
import-failed-count = { $n } konnten nicht kopiert werden:

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = Jan., Feb., März, Apr., Mai, Juni, Juli, Aug., Sept., Okt., Nov., Dez.
locale-months-long = Januar, Februar, März, April, Mai, Juni, Juli, August, September, Oktober, November, Dezember
locale-date = { $day }. { $month } { $year }
locale-decimal = ,
locale-group = .
locale-unit-kb = KB
locale-unit-mb = MB
locale-unit-gb = GB

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Geben Sie einen Namen für den neuen Ordner ein.
import-name-chars = Ein Ordnername darf keines der Zeichen / \ : * ? " < > | enthalten.
import-name-long = Dieser Ordnername ist zu lang.
import-drive = Laufwerk { $letter }:
import-mkdir-fail = Der Ordner { $path } konnte nicht erstellt werden: { $error }
import-too-many = zu viele Dateien tragen diesen Namen
import-appeared = während des Kopierens ist eine Datei mit diesem Namen aufgetaucht
export-logo-bad = das Logo { $path } ist nicht lesbar ({ $error })
export-unreadable = das Foto konnte nicht gelesen werden
gallery-too-many = zu viele Galerien tragen hier diesen Namen
gallery-none-read = keines der Fotos konnte gelesen werden
movie-no-pictures = keine Fotos
movie-ffmpeg-start = FFmpeg konnte nicht gestartet werden: { $error }
movie-ffmpeg-input = FFmpeg hat keinen Eingang
movie-ffmpeg-stopped = FFmpeg wurde beendet ({ $error })
movie-ffmpeg-failed =
    FFmpeg ist fehlgeschlagen:
    { $tail }
root-already = Dieser Ordner ist bereits in der Bibliothek.
root-inside = Dieser Ordner ist bereits in der Bibliothek, in { $outer }.
root-excluded = That folder is inside { $folder }, which is left out of the library.
exclude-is-root = That is a library folder: remove it instead.
exclude-outside = That folder is not inside a library folder.
exclude-already = That folder is already left out.
online-tile-bad = eine Kartenkachel konnte nicht gelesen werden: { $error }
online-map-down = Online-Karte nicht erreichbar ({ $error })
online-search-down = Die Online-Suche ist nicht erreichbar ({ $error }).
online-search-odd = Unerwartete Antwort der Online-Suche: { $error }
collage-nothing = es gibt nichts zu speichern
collage-extension = speichern Sie als .pdf, .png oder .jpg
collage-page-fail = eine Seite konnte nicht gezeichnet werden
geo-near = In der Nähe von { $name }
mail-no-pictures = keine Fotos zum Anhängen
mail-copies-fail = Die Kopien für die E-Mail konnten nicht erstellt werden: { $error }
mail-stopped = Die E-Mail wurde nicht vorbereitet: Abbruch angefordert.
mail-none-read = Die Kopien für die E-Mail konnten nicht erstellt werden: Keines der Fotos konnte gelesen werden.
mail-left-out =
    { $n ->
        [one] { $n } Foto konnte nicht gelesen werden und wurde ausgelassen.
       *[other] { $n } Fotos konnten nicht gelesen werden und wurden ausgelassen.
    }
mail-opened =
    Eine neue Nachricht mit { $n ->
        [one] { $n } Foto
       *[other] { $n } Fotos
    } ist in Ihrem E-Mail-Programm geöffnet.{ $left_out }
mail-asked =
    Ihr E-Mail-Programm wurde um eine neue Nachricht mit { $n ->
        [one] { $n } Foto
       *[other] { $n } Fotos
    } gebeten. Sind sie nicht angehängt, fügen Sie sie aus diesem Ordner an:
    { $path }{ $left_out }
mail-open-fail =
    Ihr E-Mail-Programm konnte nicht geöffnet werden ({ $error }).
    Die kleinen Kopien von { $n ->
        [one] { $n } Foto
       *[other] { $n } Fotos
    } liegen in:
    { $path }
    Hängen Sie sie an Ihre Nachricht an.{ $left_out }
mail-too-many = zu viele Fotos
mail-no-mapi = kein E-Mail-Programm (mapi32.dll: { $error })
mail-no-attach = das E-Mail-Programm akzeptiert keine Anhänge (kein MAPISendMailW)
mail-refused = das E-Mail-Programm hat die Nachricht abgelehnt (MAPI-Fehler { $code })
mail-open-mail = Mail konnte nicht geöffnet werden: { $error }
mail-open-mail-short = Mail konnte nicht geöffnet werden
mail-no-xdg = xdg-email (aus xdg-utils) ist nicht installiert
mail-xdg-fail = xdg-email konnte nicht gestartet werden: { $error }
gallery-html-lang = de
gallery-count = { $n ->
    [one] { $n } Foto
   *[other] { $n } Fotos
}
gallery-close = Schließen
gallery-prev = Zurück
gallery-next = Weiter
sheet-count = { $n ->
    [one] { $n } Foto
   *[other] { $n } Fotos
}
