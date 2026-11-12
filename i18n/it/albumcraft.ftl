# Testo dell'interfaccia di AlbumCraft, in italiano.

language-name = Italiano

## Shared buttons and words

button-ok = OK
button-cancel = Annulla
button-close = Chiudi
button-remove = Rimuovi
button-save = Salva
add-folder = Aggiungi cartella…
scan-again = Analizza di nuovo
removed-root = { $path } è stata rimossa dalla libreria. Le foto e le tue note su di esse sono intatte.

## Settings dialog

settings-title = Impostazioni
settings-saved-note = Le modifiche vengono salvate man mano.

settings-section-interface = Interfaccia
settings-section-folders = Cartelle
settings-section-editing = Modifica
settings-section-slideshow = Presentazione
settings-section-export = Esportazione
settings-section-maps = Mappe
settings-section-storage = Archiviazione
settings-section-ai = Funzioni IA

## Settings > Interface

settings-language = Lingua
settings-language-system = Come il sistema ({ $language })
settings-language-note = È possibile aggiungere altre lingue come file di traduzione; vedi i18n/ nel codice sorgente.
settings-theme = Tema
theme-system = Come il sistema
theme-light = Chiaro
theme-dark = Scuro

## Dialogs (src/dialogs.rs)

dialog-new-album = Nuovo album
dialog-rename-album = Rinomina album
dialog-tags = Tag e persone
dialog-gallery = Galleria web
dialog-folder-note = Descrizione della cartella
dialog-movie = Filmato
dialog-about = Informazioni su AlbumCraft
dialog-online-maps = Mappe
dialog-print-review = Verifica
dialog-trash = Sposta nel cestino

new-album-name = Nome dell'album:
new-album-adds = { $n ->
    [one] La foto selezionata verrà aggiunta.
   *[other] Le { $n } foto selezionate verranno aggiunte.
}
new-album-create = Crea
rename-album-rename = Rinomina

tags-intro = { $n ->
    [one] Da aggiungere alla foto selezionata. Separa le parole con virgole.
   *[other] Da aggiungere alle { $n } foto selezionate. Separa le parole con virgole.
}
tags-tags = Tag:
tags-people = Persone:
tags-add = Aggiungi

gallery-intro = { $n ->
    [one] Una pagina web con una foto e le tue modifiche, leggibile in qualsiasi browser o da pubblicare su un servizio di hosting.
   *[other] Una pagina web con { $n } foto e le tue modifiche, leggibile in qualsiasi browser o da pubblicare su un servizio di hosting.
}
gallery-title = Titolo:
gallery-style = Stile:
gallery-size = Dimensione delle foto:
gallery-size-medium = Media (1280 pixel, file leggeri)
gallery-size-large = Grande (2048 pixel, per schermi grandi)
gallery-captions = Mostra le didascalie
gallery-make = Scegli la cartella e crea…

folder-note-intro = Una breve descrizione mostrata sotto il nome della cartella:


# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] Un video di una foto come nella presentazione: { $seconds } s
   *[other] Un video di { $n } foto come nella presentazione: { $seconds } s ciascuna
}{ $fade ->
    [yes] , con dissolvenze
   *[no] {""}
}{ $motion ->
    [yes] , con panoramica e zoom
   *[no] {""}
}.
movie-length = Durata: circa { $seconds } s. Modifica i tempi e gli effetti nella barra della presentazione o in Impostazioni.
movie-music = Musica
movie-other-music = Altra musica…
movie-add-music = Aggiungi musica…
movie-no-ffmpeg = Per creare filmati serve il software libero FFmpeg, che non è stato trovato.
movie-install-ffmpeg = Installalo da ffmpeg.org (o con il gestore di pacchetti del sistema), poi riapri questa finestra.
movie-make = Scegli il file e crea…
movie-preview = Anteprima come presentazione

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] Spostare nel cestino la foto scartata ({ $size })?
   *[other] Spostare nel cestino le { $count } foto scartate ({ $size })?
}
trash-explain = I file vengono rimossi dalle loro cartelle. Puoi recuperarli dal cestino del sistema finché non viene svuotato.
trash-move = Sposta nel cestino

print-review-remove = Rimuovi queste foto
print-review-print = Stampa comunque

about-text = Un organizzatore di foto per il computer: sfoglia le tue cartelle di foto, crea album, scegli, scarta e valuta le foto, correggile e ritoccale, e avvia presentazioni.
about-edits = Le modifiche non alterano mai i file originali; vengono applicate quando guardi o esporti una foto.
about-credits = Mappa integrata: Natural Earth (pubblico dominio). Elenco delle città: GeoNames (CC BY 4.0). Mappe online: © collaboratori di OpenStreetMap (ODbL).

online-maps-question = Mostrare mappe dettagliate da Internet?
online-maps-what = AlbumCraft può mostrare dove sono state scattate le tue foto. Con le mappe online scarica le mappe da OpenStreetMap e può cercare indirizzi quando indichi un luogo.
online-maps-privacy = Le tue foto e le loro posizioni non vengono mai inviate. I server delle mappe vedono il tuo indirizzo internet, le zone che guardi e i luoghi che cerchi.
online-maps-offline = Senza le mappe online, AlbumCraft usa la sua mappa del mondo e l'elenco di città integrati e non si connette a Internet.
online-maps-later = Potrai cambiarlo più tardi sulla mappa o in Impostazioni (menu principale): Mappe.
online-maps-use = Usa le mappe online
online-maps-builtin = Solo la mappa integrata

## Notices at the bottom of the window

notice-undo = Annulla
notice-show = Mostra nella cartella
notice-close = Chiudi questo messaggio
notice-no-file-manager = Impossibile aprire il gestore di file: { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } foto
   *[other] { $n } foto
}

settings-bar = Barra in fondo alla libreria
settings-bar-compact = Compatta: una sola riga, mostrata quando ci sono foto selezionate
settings-bar-tray = Vassoio: miniature delle foto selezionate e pulsanti grandi
settings-bar-note = Nella barra compatta i pulsanti perdono il nome quando la finestra è stretta. Quando i pulsanti non entrano, la barra scorre di lato.
settings-thumb-size = Dimensione delle foto nella libreria

settings-folders-intro = La libreria mostra le foto di queste cartelle e delle loro sottocartelle dove si trovano: nulla viene copiato o spostato. Per copiare foto da una scheda di memoria o da una fotocamera, usa Importa.
settings-folders-none = Ancora nessuna cartella: vengono mostrate foto di esempio.
settings-folders-remove-hint = Non mostrare più questa cartella (le sue foto e le tue note su di esse vengono conservate)
settings-folders-rescan-hint = Cerca ora le foto nuove, modificate ed eliminate
settings-excluded-title = Left out folders
settings-excluded-intro = Folders inside a library folder that are not shown, with everything under them (for example Screenshots).
settings-excluded-none = Nothing is left out.
settings-excluded-add = Leave out a folder
settings-excluded-remove-hint = Show this folder again

settings-edit-opens = La scheda Modifica si apre su
settings-edit-basic = Base: correzioni con un clic e strumenti
settings-edit-advanced = Avanzate: i cursori
settings-info-panel = Pannello informazioni
settings-info-show = Mostra il pannello informazioni nell'editor (I)

settings-timing = Tempi
settings-next-after = Foto successiva dopo
settings-note-paused = La barra spaziatrice mette in pausa la presentazione, ← e → permettono di spostarsi.
settings-note-manual = Ogni foto resta visibile finché non passi alla successiva con → o con i pulsanti. Il visualizzatore a schermo intero (F11) avanza comunque da solo.
settings-effects = Effetti
settings-fade = Dissolvenza tra le foto
settings-motion = Panoramica e zoom lenti
settings-motion-subtle = Lieve
settings-motion-medium = Medio
settings-captions = Mostra le didascalie
settings-viewer-note = Il visualizzatore a schermo intero (F11 nell'editor) parte sempre senza dissolvenza né panoramica e zoom.
settings-music = Musica
settings-music-play = Riproduci musica durante le presentazioni
settings-music-another = Scegline un'altra…
settings-music-choose = Scegli la musica…
settings-movies-note = Anche i filmati creati con le foto usano queste impostazioni.

settings-export-note = Con cosa parte la vista Esporta.
settings-export-size = Dimensione delle foto
settings-export-quality = Qualità JPEG
settings-export-info = Informazioni nelle copie
settings-export-gps = Includi il luogo dello scatto (GPS)
settings-export-gps-hint = Disattiva prima di condividere le foto se non vuoi che altri vedano il luogo.
settings-export-exif = Includi i dati della fotocamera (EXIF)
settings-export-exif-hint = Fotocamera, obiettivo, esposizione e data dello scatto, copiati dal file originale. Il luogo segue l'impostazione precedente.
settings-creator = Autore
settings-creator-hint = Il tuo nome
settings-copyright = Copyright
settings-copyright-hint = © Il tuo nome
settings-credit-note = Scritti in ogni copia (EXIF), al posto di quelli dell'originale. Anche le copie inviate per e-mail seguono queste impostazioni. La firma o il logo si sceglie nella vista Esporta.

settings-maps-online = Mappe online
settings-maps-use = Usa le mappe online (OpenStreetMap)
settings-maps-note = Mappe dettagliate e ricerca di indirizzi. Le tue foto e le loro posizioni non vengono mai inviate; i server delle mappe vedono il tuo indirizzo internet, le zone che guardi e i luoghi che cerchi. Disattivate: solo la mappa del mondo e l'elenco di città integrati, e nessuna connessione a Internet.
settings-maps-tiles = Tessere di mappa scaricate
settings-maps-tiles-after = Le tessere di mappa vengono scaricate di nuovo quando serve.

settings-notes = Le tue note
settings-notes-note = Note, didascalie, tag, modifiche e album sono conservati in un unico file; i tuoi file di foto non vengono mai modificati.
settings-thumbs = Miniature
settings-thumbs-after = Le miniature vengono rifatte dalle foto quando serve (più lento per un po').
cache-on-disk = { $size } su disco
cache-measuring = Misurazione in corso…
cache-empty = Svuota

settings-ai-soon = Non ancora disponibile. Previsto:
settings-ai-faces = Volti
settings-ai-faces-text = individuare i volti e raggrupparli per persona; i gruppi li nomini tu. Un download di circa 10-40 MB.
settings-ai-search = Ricerca intelligente
settings-ai-search-text = trovare foto descrivendole («neve», «torta di compleanno»). Leggera (inglese, circa 100 MB) o multilingue (circa 400 MB).
settings-ai-tags = Suggerimenti di tag
settings-ai-tags-text = tag proposti in base al contenuto della foto.
settings-ai-intro = Trovare i volti, cercare descrivendo una foto, proporre tag. Tutto viene calcolato su questo computer: nessuna foto, nessun volto e nessun testo di ricerca viene inviato altrove. Le reti di cui queste funzioni hanno bisogno vengono scaricate solo quando lo chiedi, dalle versioni pubblicate di AlbumCraft.
settings-ai-unavailable = Questa versione di AlbumCraft non può eseguire reti: queste funzioni non sono disponibili.
settings-ai-networks = Reti
settings-ai-pack-size = { $size }  ·  licenza: { $licence }
settings-ai-remove = Rimuovi
settings-ai-remove-hint = Rimuovi le reti da questo computer. Ciò che è stato trovato nelle tue foto viene conservato.
settings-ai-downloaded = Scaricato
settings-ai-downloading = Download in corso… (vedi la barra in basso)
settings-ai-download = Scarica
settings-ai-remove-failed = Impossibile rimuovere le reti: { $error }
settings-ai-search-uses = La ricerca intelligente usa
settings-ai-search-auto = Il pacchetto per la lingua dell'interfaccia
settings-ai-search-auto-hint = Il pacchetto multilingue quando l'interfaccia non è in inglese, altrimenti quello inglese
settings-ai-download-task = Download di { $pack }
settings-ai-analysis = Analisi della libreria
settings-ai-analysis-text = Ogni foto viene analizzata una volta, in background, quando AlbumCraft non ha altro da fare. Si ferma alla chiusura di AlbumCraft e riprende all'avvio successivo.
settings-ai-progress = { $n ->
    [one] { $done } di { $total } foto analizzata
   *[other] { $done } di { $total } foto analizzate
}
settings-ai-progress-paused = { $n ->
    [one] { $done } di { $total } foto analizzata (in pausa)
   *[other] { $done } di { $total } foto analizzate (in pausa)
}
settings-ai-all-analysed = Tutte le foto sono analizzate.
settings-ai-nothing = Niente da analizzare: nessuna rete è scaricata.
settings-ai-pause = Metti in pausa l'analisi

export-size-original = Dimensione originale
export-size-very-large = Molto grande (2560 pixel)
export-size-large = Grande (1600 pixel)
export-size-small = Piccola, per e-mail (800 pixel)

## Export view (src/export_view.rs)

filter-pictures = Foto
view-back-editor = Editor
view-back-library = Libreria
view-back-hint = Indietro (Esc)

export-title = Esporta
export-count-all = { $n ->
    [one] { $n } foto
   *[other] { $n } foto
}
export-count-some = { $n } su { $total } foto
export-select-first = Seleziona prima delle foto.
export-logo-unreadable = Il logo { $path } non è leggibile.
export-task = { $n ->
    [one] Esportazione di { $n } foto
   *[other] Esportazione di { $n } foto
}
export-save-into = Salva in
export-choose-folder = Scegli una cartella…
export-choose-folder-hint = Scegli la cartella per le copie
export-safe = Nulla viene sovrascritto; gli originali non vengono modificati.
export-go = { $n ->
    [one] Esporta { $n } foto
   *[other] Esporta { $n } foto
}
export-dialog-title = Esporta le foto in…

export-size = Dimensione
export-size-other = Altra:
export-size-pixels = { " " }pixel
export-size-note = Il lato più lungo. Le foto non vengono mai ingrandite.
export-quality = Qualità JPEG

export-signature = Firma
export-signature-on = Aggiungi una firma o un logo
export-sig-text = Testo
export-sig-logo = Immagine (logo)
export-logo-other = Altra immagine…
export-logo-choose = Scegli un'immagine…
export-logo-dialog = Immagine del logo o della firma
export-logo-note = Un PNG con sfondo trasparente dà il risultato migliore.
export-sig-text-hint = Il tuo nome, oppure © Il tuo nome
export-sig-position = Posizione
export-sig-size = Dimensione
export-sig-size-hint = In rapporto al lato più corto della foto
export-sig-opacity = Opacità
export-sig-colour = Colore
export-sig-shadow = Ombra
export-sig-shadow-hint = Un leggero alone perché il testo resti leggibile su qualsiasi foto

export-info = Informazioni nei file
export-gps = Luogo dello scatto (GPS)
export-gps-hint = Disattiva prima di condividere le foto se non vuoi che altri vedano il luogo.
export-exif = Dati della fotocamera (EXIF)
export-exif-hint = Fotocamera, obiettivo, esposizione e data dello scatto, copiati dal file originale.
export-creator = Autore
export-creator-hint = Il tuo nome
export-copyright = Copyright
export-copyright-hint = © Il tuo nome
export-credit-note = Scritti in ogni copia. Firma, autore e copyright vengono ricordati per la volta successiva (anche in Impostazioni, Esportazione).

export-left-out = Esclusa: clicca per esportarla comunque
export-leave-out = Clicca per escludere questa foto

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = Tutte le foto
nav-picks = Scelte
nav-places = Luoghi
nav-places-hint = Le tue foto su una mappa
nav-albums = Album
nav-people = Persone
nav-folders = Cartelle
people-hint = Indica le persone dalla barra di selezione.
scanning = Ricerca delle foto…

bar-import = Importa
bar-import-hint = Copia le foto da una scheda di memoria, una fotocamera o una cartella nella tua libreria
bar-folders = Cartelle
bar-folders-hint = Le cartelle mostrate nella libreria, dove si trovano (nulla viene copiato)
bar-size-hint = Dimensione delle foto
bar-picks-hint = Mostra solo le foto scelte
bar-hide-rejects = Nascondi le foto scartate

sort-name = Nome del file
sort-taken = Data dello scatto
sort-rating = Valutazione
sort-hint = Ordine delle foto: { $order }

filter-any = Tutte
filter-any-no-rejects = Tutte, tranne le scartate
filter-min = { $n }+
filter-min-no-rejects = { $n }+, tranne le scartate
filter-hint = Filtra per valutazione; nascondi le foto scartate
filter-any-rating = Tutte le valutazioni
filter-or-more = { $rating } o più

album-rename = Rinomina…
album-delete = Elimina l'album
album-show-map = Mostra sulla mappa
album-slideshow = Riproduci come presentazione
album-deleted = L'album «{ $name }» è stato eliminato. Le sue foto sono intatte.

empty-scanning = Ricerca delle foto…
empty-no-match = Nessuna foto corrisponde alla ricerca o al filtro.
empty-album = Questo album è vuoto. Seleziona delle foto e aggiungile dalla barra di selezione.
empty-person = Nessuno è ancora indicato.
empty-picks = Ancora nessuna foto scelta. Passa il mouse su una foto e clicca sulla sua bandierina, oppure premi P.
empty-unreachable = Nelle cartelle della tua libreria non è stata trovata nessuna foto. Se si trovano su un disco non collegato, collegalo e analizza di nuovo.
empty-library = La tua libreria è vuota. Importa delle foto oppure aggiungi una cartella che hai già.
empty-folders-hint = Le cartelle mostrate nella libreria
empty-import = Importa foto
empty-import-hint = Copia foto da una scheda di memoria, una fotocamera o una cartella
empty-add = Aggiungi una cartella
empty-add-hint = Mostra le foto di una cartella dove si trovano, senza copiarle

folder-play-hint = Riproduci questa cartella come presentazione
folder-edit-hint = Modifica la descrizione
folder-map-hint = Mostra questa cartella sulla mappa
tile-pick-hint = Scegli (P)
tile-reject-hint = Scarta (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = Nessuna valutazione
rating-stars = { $n ->
    [one] { $n } stella
   *[other] { $n } stelle
}
flag-pick = Scegli
flag-unpick = Rimuovi dalle scelte
flag-reject = Scarta
flag-unreject = Non scartare più
flag-rating = Valutazione
album-new = Nuovo album…
menu-copy-edits = Copia le modifiche
menu-paste-edits = Incolla le modifiche
menu-paste-n = { $n ->
    [one] Incolla { $n } modifica
   *[other] Incolla { $n } modifiche
}
copy-edits-select-one = Seleziona una sola foto per copiarne le modifiche.
menu-import = Importa foto…
menu-add-folder = Aggiungi una cartella alla libreria…
menu-folders = Cartelle della libreria…
menu-scan = Analizza di nuovo le cartelle
menu-select-all = Seleziona tutto
menu-select-none = Deseleziona tutto
menu-rotate-left = Ruota a sinistra
menu-rotate-right = Ruota a destra
menu-trash-rejects = Sposta nel cestino le foto scartate…
menu-trash-rejects-hint = Rimuovi dalle loro cartelle i file delle foto scartate, dopo conferma
menu-contact-sheet = Provino a contatto…
menu-contact-sheet-hint = Pagine di piccole foto con un titolo e i nomi dei file, da stampare o salvare in PDF
menu-undo-edits = Annulla tutte le modifiche
menu-settings = Impostazioni…
menu-about = Informazioni su AlbumCraft…
menu-exit = Esci
edits-removed = { $n ->
    [one] Le modifiche di { $n } foto sono state rimosse.
   *[other] Le modifiche di { $n } foto sono state rimosse.
}

status-library = Libreria: { $folders ->
    [one] { $folders } cartella
   *[other] { $folders } cartelle
}, { $pictures ->
    [one] { $pictures } foto
   *[other] { $pictures } foto
}, { $size } su disco
status-importing = Importazione di { $done } su { $total }…
status-stop = Ferma
status-stop-hint = Ferma dopo la foto in corso (quelle già copiate restano)
task-running = { $label }…
task-stopping = { $label }: arresto in corso…
task-cancel-hint = Ferma dopo la foto in corso
status-samples = Vengono mostrate foto di esempio. Usa Importa o Cartelle per aggiungere le tue.
status-analysis = Analisi delle foto { $done } / { $total }
status-analysis-paused = Analisi delle foto in pausa ({ $done } / { $total })
status-analysis-pause = Pausa
status-analysis-pause-hint = Ferma l'analisi per ora. Ciò che è fatto viene conservato; si ferma anche alla chiusura di AlbumCraft e riprende all'avvio successivo.
status-analysis-resume = Riprendi
status-analysis-resume-hint = Continua ad analizzare le foto in background

selected-count = { $n ->
    [one] { $n } foto
   *[other] { $count } foto
}
selected-all = { $n ->
    [one] { $pictures } selezionata
   *[other] { $pictures } selezionate
}
selected-set = { $name } · { $pictures }
fallback-folder = Cartella
fallback-album = Album
tray-nothing = Nessuna selezione
tray-click = Clicca sulle foto per selezionarle

output-web = Pagina web
output-web-tip = Crea una galleria web con queste foto
output-mail = E-mail
output-mail-tip = Piccole copie, allegate a un nuovo messaggio nel tuo programma di posta
output-print = Stampa
output-print-tip = Stampa queste foto (Ctrl+P)
output-export = Esporta
output-export-tip = Salva copie ritoccate in una cartella
output-collage = Collage
output-collage-tip = Crea un collage o un provino a contatto
output-movie = Filmato
output-movie-tip = Crea un filmato

bar-clear-hint = Annulla la selezione (Esc)
bar-pick-hint = Scegli / rimuovi dalle scelte (P)
bar-reject-hint = Scarta / non scartare più (X)
bar-rotate-left = Ruota a sinistra (Ctrl+[)
bar-rotate-right = Ruota a destra (Ctrl+])
bar-tags = Tag e persone
bar-location = Imposta il luogo: dove sono state scattate queste foto, su una mappa
bar-to-album = In un album
bar-clear = Annulla

ctx-open = Apri nell'editor
ctx-slideshow = Riproduci la presentazione da qui
ctx-more-like-this = Altre foto simili
ctx-more-like-this-hint = Mostra le foto che somigliano a questa
ctx-add-to-album = Aggiungi a un album
ctx-remove-from-album = Rimuovi dall'album
ctx-tags = Tag e persone…
ctx-location = Imposta il luogo…
ctx-export = Esporta…
ctx-show-file = Mostra nel gestore di file

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Vivido
look-vivid-tip = Colori ricchi e saturi da pellicola per diapositive, con neri profondi
look-soft-film = Pellicola morbida
look-soft-film-tip = Delicata pellicola da ritratto: contrasto morbido, caldo e gradevole per la pelle
look-polaroid = Istantanea
look-polaroid-tip = Stampa sbiadita da fotocamera istantanea: neri sollevati, luci calde, ombre fredde
look-cross-process = Sviluppo incrociato
look-cross-process-tip = Luci gialle, ombre verde-azzurre e contrasto marcato
look-lomo = Lomo
look-lomo-tip = Resa satura da fotocamera giocattolo, con angoli scuri
look-matte = Opaco
look-matte-tip = Neri piatti e slavati, colori attenuati
look-teal-orange = Verde-azzurro e arancio
look-teal-orange-tip = Color grading da cinema: ombre verde-azzurre, pelle e luci calde
look-high-key = High key
look-high-key-tip = Luminoso e arioso, con ombre aperte
look-cool = Freddo
look-cool-tip = Una dominante più fredda, più blu
look-warm = Caldo
look-warm-tip = Una dominante più calda, dorata
look-bw = B/N
look-bw-tip = Bianco e nero classico
look-dramatic-bw = B/N drammatico
look-dramatic-bw-tip = Bianco e nero con filtro rosso: cieli scuri, forte contrasto locale
look-sepia = Seppia
look-sepia-tip = Tono marrone da foto antica
look-soft-focus = Sfocato morbido
look-soft-focus-tip = Centro nitido, bordi morbidi e sognanti
look-glow = Bagliore
look-glow-tip = Un morbido alone luminoso
look-film-grain = Grana della pellicola
look-film-grain-tip = Grana in stile pellicola
look-focal-bw = B/N focale
look-focal-bw-tip = Colore al centro, bianco e nero intorno
effect-sharpen = Nitidezza
effect-sepia = Seppia
effect-bw = B/N
effect-warmify = Riscalda
effect-film-grain = Grana della pellicola
effect-tint = Tinta
effect-saturation = Saturazione
effect-soft-focus = Sfocato morbido
effect-glow = Bagliore
effect-filtered-bw = B/N filtrato
effect-focal-bw = B/N focale
effect-graduated-tint = Tinta sfumata
op-rotate-right = Ruota a destra
op-rotate-left = Ruota a sinistra
op-crop = Ritaglia
op-straighten = Raddrizza { $angle }°
op-perspective-v = Prospettiva, verticale { $v }
op-perspective-h = Prospettiva, orizzontale { $h }
op-perspective-both = Prospettiva { $v } / { $h }
op-redeye = Occhi rossi
op-lucky = Correzione automatica (fortuna)
op-auto-contrast = Contrasto automatico
op-auto-color = Colore automatico
op-tuning = Regolazione fine
op-look = { $look } { $amount } %
op-adjust = Regolazione
op-adjust-parts = Regolazione: { $parts }
adj-wb = bilanciamento del bianco
adj-exposure = esposizione
adj-highlights = alte luci
adj-shadows = ombre
adj-blacks = neri
adj-contrast = contrasto
adj-saturation = saturazione
adj-vibrance = vividezza
adj-clarity = chiarezza
adj-sharpen = nitidezza
adj-noise = riduzione del rumore
decor-vignette-light = Vignettatura chiara
decor-vignette = Vignettatura
decor-instant = Cornice istantanea
decor-border = Bordo
decor-shadow = Ombra esterna
decor-rounded = Angoli arrotondati
decor-text = Testo «{ $text }»
wb-as-shot = Come scattata
wb-daylight = Luce diurna
wb-cloudy = Nuvoloso
wb-shade = Ombra
wb-tungsten = Tungsteno
wb-fluorescent = Fluorescente
wb-flash = Flash
wb-custom = Personalizzato
wb-auto = Auto
gallery-style-light = Griglia chiara
gallery-style-dark = Griglia scura
gallery-style-masonry = Mosaico (foto intere)
crop-free = Libero
crop-original = Forma originale
crop-square = Quadrato
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (stampa 10 × 15)
crop-16-9 = 16 × 9 (schermo panoramico)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Doppio clic per reimpostare
edit-loading = La foto è ancora in caricamento.
edit-straightened = Raddrizzata di { $angle }°.
edit-already-level = Questa foto sembra già dritta.
edit-no-horizon = Non è stato trovato nessun orizzonte né linea verticale netta da raddrizzare.
edit-histogram = Istogramma e informazioni sulla fotocamera
edit-unknown-camera = Fotocamera sconosciuta
edit-35mm = (equivalente a 35 mm: { $mm } mm)
edit-focal = Lunghezza focale: { $mm } mm
edit-iso = ISO: { $iso }
edit-no-camera = Nessuna informazione sulla fotocamera in questa foto
edit-clip-shadows = ombre { $pct } %
edit-clip-highlights = alte luci { $pct } %
edit-clipped = Clipping: { $parts }
edit-clipped-hint = Parte della foto che è nero puro o bianco puro in almeno un colore
edit-back = Torna alla libreria
edit-info = Info
edit-info-hint = Proprietà, luogo e tag (I)
edit-side-by-side-hint = Mostra le foto una accanto all'altra
edit-stacked-hint = Mostra una foto sopra l'altra
edit-focus-hint = Cambia la foto che stai ritoccando (quella che riceve le tue modifiche)
edit-compare = Confronta
edit-compare-hint = Questa foto accanto a un'altra, una di fianco all'altra
edit-original = Originale
edit-original-hint = Tieni premuto per vedere la foto senza le tue modifiche (o tieni premuto O)
edit-slideshow = Presentazione
edit-fullscreen-hint = Schermo intero (F11): da questa foto, una dopo l'altra, con musica
edit-next = Foto successiva
edit-previous = Foto precedente
edit-position = { $n } di { $total }
edit-tab-edit = Modifica
edit-tab-presets = Preimpostazioni
edit-tab-effects = Effetti
edit-undo = Annulla
edit-undo-hint = Annulla l'ultima modifica (Ctrl+Z)
edit-redo = Ripeti
edit-list = Modifiche di questa foto:
edit-list-none = (nessuna: è l'originale)
edit-step-hint =
    { $label }
    Clicca per tornare a questo passaggio (quelli successivi vengono rimossi; Annulla li ripristina)
edit-copy-hint = Copia queste modifiche per incollarle su altre foto (Ctrl+Maiusc+C)
edit-paste-n = Incolla { $n }
edit-paste-hint = Aggiungi le modifiche copiate a questa foto (Ctrl+Maiusc+V)
edit-basic = Base
edit-advanced = Avanzate
edit-wb-help = Clicca su un elemento della foto che dovrebbe essere grigio neutro o bianco.
edit-crop-help = Trascina i bordi o gli angoli del riquadro, trascina all'interno per spostarlo, oppure disegna un nuovo riquadro. Invio applica.
edit-crop-shape = Forma
edit-crop-orientation = Orientamento
edit-landscape = Orizzontale
edit-landscape-hint = Più larga che alta
edit-portrait = Verticale
edit-portrait-hint = Più alta che larga
edit-apply = Applica
edit-remove-crop = Rimuovi il ritaglio
edit-remove-crop-hint = Mostra di nuovo tutta la foto
edit-redeye-help = Disegna un riquadro intorno agli occhi rossi. Solo le pupille rosse all'interno del riquadro vengono scurite.
edit-straighten-title = Raddrizza
edit-straighten-help = Sposta il cursore finché l'orizzonte non è allineato alla griglia.
edit-oneclick = Correzioni con un clic
edit-lucky = Mi sento fortunato
edit-lucky-hint = Esposizione, bilanciamento del bianco, punto del nero e un po' di vividezza, tutto insieme (poi rifiniscili in Avanzate)
edit-auto-exposure = Esposizione automatica
edit-auto-exposure-hint = Schiarisci o scurisci per ottenere un'esposizione equilibrata
edit-auto-colour = Colore automatico
edit-auto-colour-hint = Bilanciamento del bianco automatico: elimina una dominante di colore
edit-auto-contrast = Contrasto automatico
edit-auto-contrast-hint = Estendi i toni più scuri e più chiari
edit-auto-straighten = Raddrizzamento automatico
edit-auto-straighten-hint = Raddrizza un orizzonte o delle verticali inclinati
edit-tools = Strumenti
edit-pick-neutral = Punto neutro
edit-pick-neutral-hint = Clicca su un elemento grigio per impostare il bilanciamento del bianco
edit-sec-wb = BILANCIAMENTO DEL BIANCO
edit-sec-light = LUCE
edit-sec-colour = COLORE
edit-sec-detail = DETTAGLIO
edit-sec-geometry = GEOMETRIA
edit-wb-pick = Scegli
edit-wb-pick-hint = Clicca su un elemento grigio della foto
edit-s-temperature = Temperatura
edit-s-temperature-tip = Più blu (a sinistra) o più caldo (a destra)
edit-s-tint = Tinta
edit-s-tint-tip = Più verde (a sinistra) o più magenta (a destra)
edit-s-exposure = Esposizione
edit-s-exposure-tip = Luminosità complessiva, in stop
edit-s-highlights = Alte luci
edit-s-highlights-tip = Zone chiare: a sinistra recupera dettaglio, a destra schiarisce
edit-s-shadows = Ombre
edit-s-shadows-tip = Zone scure: a destra le apre
edit-s-blacks = Neri
edit-s-blacks-tip = Punto del nero: a sinistra approfondisce, a destra solleva
edit-s-contrast = Contrasto
edit-s-contrast-tip = Distanza tra scuro e chiaro
edit-s-saturation = Saturazione
edit-s-saturation-tip = Tutti i colori
edit-s-vibrance = Vividezza
edit-s-vibrance-tip = Soprattutto i colori spenti; risparmia la pelle e i colori già vivaci
edit-s-clarity = Chiarezza
edit-s-clarity-tip = Contrasto locale nei toni medi
edit-s-sharpen = Nitidezza
edit-s-sharpen-tip = Dettagli fini più nitidi
edit-s-noise = Rumore
edit-s-noise-tip = Riduzione del rumore: leviga la grana mantenendo i contorni
edit-reset-all = Reimposta tutto
edit-crop-btn = Ritaglia…
edit-crop-btn-hint = Scegli la parte della foto da mantenere, o modifica il ritaglio
edit-s-straighten-tip = Ruota leggermente la foto per raddrizzare l'orizzonte (ingrandisce per nascondere gli angoli)
edit-s-vertical = Verticale
edit-s-vertical-tip = Prospettiva: a destra allarga la parte alta, per raddrizzare gli edifici che sembrano cadere all'indietro quando si fotografa dal basso
edit-s-horizontal = Orizzontale
edit-s-horizontal-tip = Prospettiva: a destra ingrandisce il lato destro, per una parete vista di sbieco
edit-intensity = Intensità
edit-remove-preset = Rimuovi la preimpostazione
edit-presets-help = Look per tutta la foto. Clicca su uno, poi regolane l'intensità.
edit-effects-add = Aggiungi sopra o intorno alla foto
edit-add-instant = Cornice istantanea
edit-add-text = Testo
edit-your-text = Il tuo testo
edit-effects-note = Gli effetti si applicano sopra tutte le altre modifiche. Aggiungine uno per regolarlo qui.
edit-p-amount = Intensità
edit-p-size = Dimensione
edit-p-width = Larghezza
edit-p-bottom = In basso
edit-p-colour = Colore
edit-p-darkness = Oscurità
edit-p-background = Sfondo
edit-p-radius = Raggio
edit-vignette-note = A sinistra del centro vengono invece schiariti gli angoli.
edit-halo = Alone leggero (leggibile su qualsiasi foto)
edit-remove-effect = Rimuovi questo effetto
edit-caption-hint = Aggiungi una didascalia!
edit-print-hint = Stampa questa foto (Ctrl+P)
edit-album = Album
edit-zoom-hint = Zoom (100 % = i pixel reali della foto)
edit-zoom-slider-hint = Zoom: a sinistra = adatta, al centro = 1:1, a destra = 400 % (funziona anche la rotellina; trascina la foto per ingrandire al 100 % e spostarla)
edit-one-to-one-hint = Dimensione originale (100 %): un pixel della foto per pixel dello schermo
edit-fit = Adatta
edit-fit-hint = Adatta tutta la foto alla finestra (un doppio clic su una foto ingrandita fa lo stesso)
edit-people = Persone:
edit-person-remove = Rimuovi la persona
edit-person-add = aggiungi una persona
edit-compare-none = Nessun'altra foto in questa cartella
edit-compare-selected = Selezionata
edit-compare-swap-hint = Clicca per ritoccare invece questa foto
edit-view-loading = Caricamento…
edit-sharpening = Applicazione della nitidezza…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = Questa foto non è leggibile.
edit-full-unreadable = La foto completa non è leggibile.
edit-full-loading = Caricamento della foto completa…
edit-draw-box = Trascina sulla foto per disegnare un riquadro
edit-click-neutral = Clicca su un elemento che dovrebbe essere grigio o bianco
edit-wb-title = Bilanciamento del bianco

## Info panel, search, slideshow bar

info-hide-hint = Nascondi questo pannello (I)
info-properties = Proprietà
info-camera = Fotocamera
info-lens = Obiettivo
info-taken = Scattata il
info-file-date = { $date } (data del file)
info-size = Dimensione
info-megapixels = { $w } × { $h } ({ $mp } Mpx)
info-file = File
info-focal = Lunghezza focale
info-focal-35 = { $f } mm (35 mm: { $e } mm)
info-aperture = Apertura
info-exposure = Esposizione
info-folder = Cartella
info-sample = Foto di esempio integrata
info-edits = Modifiche
info-no-exif = Nessuna informazione sulla fotocamera (EXIF) in questa foto.
info-exif = EXIF dettagliati
info-flash = Flash
info-bias = Compensazione dell'esposizione
info-program = Programma
info-metering = Misurazione
info-distance = Distanza
info-colour-space = Spazio colore
info-gps = Posizione GPS
info-digitized = Digitalizzata
info-modified = Modificata
info-software = Software
info-preview = Anteprima della fotocamera
info-preview-hint = { $w } × { $h } pixel, memorizzata nel file
info-all-exif = Tutti i dati EXIF ({ $n } voci)
info-location = Luogo
info-set-in-app = Impostato in AlbumCraft
info-from-gps = Dal GPS della fotocamera
info-modify = Modifica…
info-modify-hint = Sposta il segnaposto, rinomina il luogo o rimuovilo
info-show-map-hint = Apri Luoghi su questa foto
info-no-location = Ancora nessun luogo.
info-set-location-hint = Trova il luogo o cliccaci sopra su una mappa
info-tags = Tag
info-no-tags = Ancora nessun tag.
info-remove-tag = Rimuovi questo tag
info-add-tag = Aggiungi un tag…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = E
coord-w = O
info-orient-2 = Specchiata
info-orient-3 = Capovolta
info-orient-4 = Specchiata, capovolta
info-orient-5 = Specchiata, ruotata a sinistra
info-orient-6 = Ruotata a destra (fotocamera tenuta dritta)
info-orient-7 = Specchiata, ruotata a destra
info-orient-8 = Ruotata a sinistra (fotocamera tenuta dritta)
search-remove-hint = Rimuovi dalla ricerca
search-hint = Cerca tag, luoghi, didascalie…
search-and = e…
search-clear-hint = Cancella la ricerca
search-for = Cerca «{ $text }»
search-kind-tags = Tag
search-like-chip = Come { $name }
search-looks-like-title = Somiglia a
search-similar-title = Foto simili
search-still-analysing = (analisi in corso: alcune foto non sono ancora classificate)
search-show-more = Mostra altre
search-show-more-hint = { $n ->
    [one] Un'altra foto corrisponde meno bene
   *[other] Altre { $count } foto corrispondono meno bene
}
search-like-no-pack = Per trovare foto simili, scarica un pacchetto di ricerca in Impostazioni › Funzioni IA.
search-failed-because = La ricerca intelligente non è riuscita: { $error }
search-failed = la ricerca si è interrotta in modo imprevisto
search-no-folder = AlbumCraft non ha una cartella per i suoi dati
search-pack-failed = La ricerca intelligente non è potuta partire: { $error }
search-not-analysed = Questa foto non è ancora analizzata.
music-dialog-title = Musica per la presentazione
filter-music = Musica
ss-previous = Precedente (←)
ss-play-pause = Riproduci / pausa (Spazio)
ss-next = Successiva (→)
ss-seconds = Secondi per foto
ss-interval-off = Disattivato: le foto cambiano solo con → o con i pulsanti (Impostazioni, Presentazione)
ss-fade = Dissolvenza
ss-motion = Movimento
ss-other-music-hint = Scegli un altro file musicale
ss-exit-full = Esci dallo schermo intero (F11 o Esc)
ss-exit = Esci (Esc)

## Messages of the window (src/app.rs)

save-failed-prefix = Non è stato possibile salvare la tua libreria:
save-failed =
    { $prefix } { $error }
    Le tue ultime modifiche per ora sono solo in questa finestra; AlbumCraft continua a riprovare.
save-ok-again = La tua libreria viene di nuovo salvata.
lib-unreadable-kept =
    Non è stato possibile leggere il file della tua libreria ({ $error }).

    È stato conservato in:
    { $backup }

    AlbumCraft è stato avviato con una libreria vuota. Le tue foto non sono state toccate.
lib-unreadable-nosave =
    Non è stato possibile leggere il file della tua libreria ({ $error }).

    AlbumCraft è stato avviato con una libreria vuota e questa volta non salverà nessuna modifica, per non sovrascrivere il file.
dialog-library = Libreria
crash-title = AlbumCraft si è chiuso in modo imprevisto
crash-text =
    AlbumCraft si è arrestato a causa di un errore durante l'ultima esecuzione. Le modifiche fatte negli ultimissimi secondi potrebbero mancare.

    I dettagli sono stati scritti in:
    { $log }

    Allega questo file se segnali il problema.
picasa-imported =
    { $n ->
        [one] Note, didascalie e modifiche importate dal file .picasa.ini di { $n } cartella.
       *[other] Note, didascalie e modifiche importate dai file .picasa.ini di { $n } cartelle.
    }
copy-no-edits = Questa foto non ha modifiche da copiare.
copied-edits =
    { $n ->
        [one] { $n } modifica copiata. Seleziona delle foto e incollala (Ctrl+Maiusc+V).
       *[other] { $n } modifiche copiate. Seleziona delle foto e incollale (Ctrl+Maiusc+V).
    }
paste-first = Copia prima le modifiche di una foto (Ctrl+Maiusc+C).
pasted-edits =
    { $n ->
        [one] Modifiche incollate su { $n } foto.
       *[other] Modifiche incollate su { $n } foto.
    }
no-pictures-to-show = Non ci sono foto da mostrare.
music-error = Nessuna musica: { $error }
choose-folder-dialog = Scegli una cartella di foto
added-to-album =
    { $n ->
        [one] { $n } foto aggiunta a «{ $name }».
       *[other] { $n } foto aggiunte a «{ $name }».
    }
trash-samples = Le foto di esempio non possono essere spostate nel cestino.
trash-no-rejects = Nessuna foto è scartata. Premi X per scartarne una.
task-trash =
    { $n ->
        [one] Spostamento di { $n } foto nel cestino
       *[other] Spostamento di { $n } foto nel cestino
    }
task-email =
    { $n ->
        [one] Preparazione di { $n } foto per un'e-mail
       *[other] Preparazione di { $n } foto per un'e-mail
    }
default-title = Le mie foto
gallery-folder-dialog = Crea la galleria web in…
task-gallery =
    { $n ->
        [one] Creazione della galleria web di { $n } foto
       *[other] Creazione della galleria web di { $n } foto
    }
movie-save-dialog = Salva il filmato come…
filter-mp4 = Video MP4
task-movie = Creazione del filmato
scan-truncated = Vengono mostrate solo le prime { $n } foto: rimuovi delle cartelle dalla libreria per vedere le altre.
selection-none = Nessuna selezione
selection-one = { $name }     { $date }     { $w }x{ $h } pixel     { $size }
selection-several = { $pictures }     { $a }      { $size } su disco
selection-range = { $pictures }     dal { $a } al { $b }     { $size } su disco

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Mucchio di foto
cstyle-pile-tip = Foto sparse, come stampe gettate su un tavolo
cstyle-mosaic = Mosaico
cstyle-mosaic-tip = Foto di dimensioni diverse riempiono la pagina
cstyle-frame = Foto al centro
cstyle-frame-tip = Un mosaico attorno a una grande foto al centro
cstyle-grid = Griglia
cstyle-grid-tip = Righe e colonne regolari
cstyle-film = Pellicola
cstyle-film-tip = Foto su strisce di pellicola
cstyle-clothesline = Filo da bucato
cstyle-clothesline-tip = Foto istantanee fissate con mollette a delle corde
cstyle-fan = Ventaglio
cstyle-fan-tip = Aperte come un mazzo di carte
cstyle-shape = Forma
cstyle-shape-tip = Un cuore, un cerchio o una stella fatti di foto
cstyle-blend = Esposizione multipla
cstyle-blend-tip = Le foto si vedono una attraverso l'altra (ideale con 2-4)
cstyle-sheet = Provino a contatto
cstyle-sheet-tip = Pagine di piccole foto con un titolo e i nomi dei file
shape-heart = Cuore
shape-circle = Cerchio
shape-star = Stella
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = Letter (USA)
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Quadrato
fmt-wide = Schermo (16:9)
grad-vertical = Dall'alto al basso
grad-diagonal = Diagonale
grad-radial = Dal centro
collage-need-sheet = Seleziona prima le foto del provino a contatto.
collage-need-two = Seleziona almeno due foto per creare un collage.
collage-unreadable = Non è stato possibile leggere questa foto.
collage-new-layout = Nuova disposizione
collage-new-layout-hint = Altre posizioni, rotazioni e dimensioni
collage-shuffle = Mescola
collage-shuffle-hint = Metti le foto in ordine casuale
collage-spacing = Spazio tra le foto
border-none = Nessuno
border-white = Bianco
border-instant = Istantanea
collage-frame-text = Testo sulle cornici
cap-none = Nessuna
cap-caption = Didascalia
collage-shadows = Ombre
bg-gradient = Sfumatura
bg-picture = Immagine
collage-to = a
collage-use-selected = Usa la foto selezionata
collage-use-selected-hint = Clicca prima su una foto del collage
collage-choose-file = Scegli un file…
collage-no-bg = Ancora nessuna immagine scelta
collage-bg-dialog = Immagine di sfondo
collage-page = Pagina
collage-size = Dimensione dell'immagine
collage-px = { $px } pixel
collage-centre = Metti al centro
collage-put-back = Rimetti a posto
collage-put-back-hint = Annulla gli spostamenti di questa foto
collage-leave-out = Escludi
collage-leave-out-hint = Rimuovi questa foto dal collage (Canc)
collage-title = Titolo
collage-no-title = Senza titolo
collage-no-subtitle = Nessuna seconda riga
collage-header = Titolo nella prima pagina
collage-under = Sotto ogni foto
collage-date = Data
collage-pictures = Foto
collage-page-numbers = Numeri di pagina
collage-sending = Invio della pagina { $page } di { $total } alla stampante…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } pagina
       *[other] { $pages } pagine
    }
collage-no-printer = Nessuna stampante disponibile
collage-save-pdf = Salva come PDF…
collage-save-picture = Salva l'immagine…
collage-save-picture-hint = Salva il collage come immagine JPEG o PNG
collage-file-sheet = Provino a contatto
collage-file-collage = Collage
collage-save-pdf-title = Salva come PDF
collage-save-picture-title = Salva il collage come immagine
filter-pdf = Documento PDF
filter-jpeg = Immagine JPEG
filter-png = Immagine PNG
collage-saving-sheet = Salvataggio del provino a contatto…
collage-saving = Salvataggio del collage…
task-save-collage = Salvataggio del collage
collage-printed =
    { $n ->
        [one] { $n } pagina inviata a { $printer }.
       *[other] { $n } pagine inviate a { $printer }.
    }
collage-print-failed = La stampa non è stata completata: { $error }
collage-hint-swap = Trascina una foto su un'altra per scambiarle. Clicca su una foto per selezionarla (Canc la esclude).
collage-hint-free = Trascina una foto per spostarla, scorri sopra di essa per ruotarla. Clicca su una foto per selezionarla (Canc la esclude).
collage-page-of = Pagina { $page } di { $total }

## Places map (src/map_view.rs)

map-search-still = Sono ancora in corso le ricerche delle posizioni GPS delle tue foto: riprova tra un attimo.
map-none-located =
    { $n ->
        [one] Questa foto non ha ancora un luogo. Selezionala e clicca su { $icon } (Imposta il luogo) nella barra di selezione per collocarla.
       *[other] Nessuna di queste { $n } foto ha ancora un luogo. Selezionale e clicca su { $icon } (Imposta il luogo) nella barra di selezione per collocarle.
    }
map-location-set =
    { $n ->
        [one] Luogo impostato per { $n } foto.
       *[other] Luogo impostato per { $n } foto.
    }
map-location-removed =
    { $n ->
        [one] Luogo rimosso da { $n } foto.
       *[other] Luogo rimosso da { $n } foto.
    }
map-nothing-online = Non è stato trovato nulla online.
map-set-location = Imposta il luogo
map-sub-scope =
    { $n } su { $total ->
        [one] { $total } foto
       *[other] { $total } foto
    } con un luogo
map-sub-indexing = { $n } con un luogo · ricerca delle posizioni GPS…
map-sub-all =
    { $n ->
        [one] { $n } foto con un luogo
       *[other] { $n } foto con un luogo
    }
map-title-scope = Luoghi · { $title }
map-show-whole = Mostra tutta la libreria
map-online = Mappa online
map-builtin = Mappa integrata
map-online-tip = Viene mostrata OpenStreetMap. Clicca per usare solo la mappa integrata (nessuna connessione a Internet).
map-builtin-tip = Viene mostrata la mappa del mondo integrata. Clicca per mostrare le mappe dettagliate di OpenStreetMap (scarica immagini di mappa da Internet).
map-show-all = Mostra tutto
map-show-all-hint = Zoom per mostrare tutte le foto sulla mappa
map-search-online-hint = Cerca un luogo o un indirizzo
map-search-builtin-hint = Cerca una città o delle coordinate
map-no-city-builtin = Nessuna città con questo nome nell'elenco integrato.
map-no-city-hint = Nessuna città con questo nome. Prova con delle coordinate, come 44,84, -0,58.
map-cities = CITTÀ
map-cities-builtin = CITTÀ (ELENCO INTEGRATO)
map-searching = Ricerca online…
map-places-osm = LUOGHI E INDIRIZZI (OPENSTREETMAP)
map-search-addresses = Cerca indirizzi online   (Invio)
map-where = Dove sono state scattate?
map-where-help = Cerca un luogo qui sopra, oppure clicca sulla mappa per posizionare un segnaposto. Poi potrai trascinarlo.
map-name-hint = Nome del luogo (facoltativo)
map-drag-pin = Trascina il segnaposto per regolarlo.
map-drag-pin-offline = Trascina il segnaposto per regolarlo. Le mappe online mostrano le strade.
map-set-n =
    { $n ->
        [one] Imposta il luogo di { $n } foto
       *[other] Imposta il luogo di { $n } foto
    }
map-remove-location = Rimuovi il luogo
map-remove-location-hint = Queste foto non avranno nessun luogo, anche se la fotocamera ne aveva registrato uno (i file non vengono modificati)
map-close-hint = Chiudi (Esc)
map-change-location = Cambia il luogo…
map-select-in-library = Seleziona nella libreria
map-select-in-library-hint = Seleziona queste foto nella libreria
map-show-hint = Mostra sulla mappa
map-preparing = Preparazione della mappa…
map-hover = { $pictures } · { $place }
map-credit-builtin = Mappa integrata: Natural Earth · GeoNames
map-problem = { $problem }. Viene mostrata la mappa integrata.
map-empty-indexing = Ricerca delle posizioni GPS delle tue foto…
map-empty =
    Ancora nessuna foto con un luogo.

    Le foto di telefoni e fotocamere con GPS compaiono qui da sole. Per collocare le altre, selezionale e clicca su { $icon } (Imposta il luogo) nella barra di selezione.

## Import (src/import_view.rs)

import-subtitle = Copia foto nella tua libreria
import-from = Da
import-choose = Scegli…
import-refresh-hint = Cerca di nuovo schede e dischi, e rileggi la cartella
import-hide-dups = Nascondi le { $n } già presenti nella libreria
import-dups-hint = Foto con lo stesso contenuto di una foto della tua libreria, anche con un nome diverso
import-uncheck-all = Deseleziona tutte
import-check-all = Seleziona tutte
import-copy-into = Copia in
import-another-folder = Un'altra cartella…
import-name-hint = Nome della nuova cartella, es. Estate in Spagna
import-checked = { $chosen } su { $shown } selezionate. Nulla viene sovrascritto; gli originali non vengono modificati.
import-go =
    { $n ->
        [one] Importa { $n } foto
       *[other] Importa { $n } foto
    }
import-dest-dialog = Metti la nuova cartella in…
import-from-dialog = Importa foto da…
import-pick-dest = Scegli dove mettere la nuova cartella.
import-running = Un'importazione è ancora in corso: attendi che finisca (vedi la barra di stato).
import-started =
    { $n ->
        [one] Importazione di { $n } foto in { $path }…
       *[other] Importazione di { $n } foto in { $path }…
    }
import-listing-in = Ricerca delle foto in { $from }…
import-none-in = Nessuna foto in { $path }.
import-choose-another = Scegli un'altra cartella…
import-heading = Importa da una scheda di memoria, una fotocamera o una cartella
import-copy-note = Le foto vengono copiate in una nuova cartella della tua libreria. Gli originali non vengono modificati.
import-no-cards = Nessuna scheda di memoria né fotocamera trovata. Collegane una e clicca su { $icon } qui sopra.
import-instead = Per mostrare le foto dove si trovano già, senza copiarle:
import-add-instead = aggiungi invece una cartella alla libreria
import-all-dups =
    { $n ->
        [one] La foto è già nella tua libreria.
       *[other] Tutte le { $n } foto sono già nella tua libreria.
    }
import-show-anyway = Mostrale comunque
import-truncated = Vengono elencate solo le prime { $n } foto.
import-group-hint = Seleziona o deseleziona tutto questo gruppo
import-in-library = Nella libreria
import-click-preview = Clicca su una foto per vederla qui
import-this = Importa questa foto
import-this-hint = Spazio
import-already = già nella tua libreria

## Print (src/print_view.rs)

print-select-first = Seleziona prima le foto che vuoi stampare.
print-page-layout = Impaginazione
print-layout-full = Pagina intera
print-shrink = Riduci per adattare
print-shrink-hint = Viene stampata la foto intera, con del bianco dove le proporzioni differiscono
print-crop = Ritaglia per adattare
print-crop-hint = La foto riempie il formato di stampa; i bordi in eccesso vengono tagliati
print-options = Opzioni di bordo e testo
print-border = Bordo
print-no-border = Nessun bordo
print-thin-line = Sottile linea nera
print-white-border = Bordo bianco
print-text-under = Testo sotto ogni foto
print-no-text = Nessun testo
print-printer = Stampante
print-no-printer-found = Nessuna stampante trovata
print-setup = Impostazioni della stampante…
print-setup-hint = Formato della carta, orientamento e impostazioni specifiche della stampante
print-paper = Carta
print-default = Predefinito della stampante
print-no-answer = Nessuna stampante ha risposto: viene mostrata una pagina A4
print-paper-describe = { $dpi } dpi, { $w } x { $h } pollici ({ $orientation })
print-copies = Copie per foto
print-copies-n =
    { $n ->
        [one] { $n } copia
       *[other] { $n } copie
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } stampa
       *[other] { $prints } stampe
    }
print-cancel = Annulla  { $icon }
print-go = Stampa  { $icon }
print-review = Verifica
print-review-hint = Verifica che ogni foto abbia abbastanza pixel per la sua dimensione di stampa
print-none-left = Non è rimasta nessuna foto da stampare.
print-job-printed =
    { $n ->
        [one] { $n } pagina inviata a { $printer }.
       *[other] { $n } pagine inviate a { $printer }.
    }
print-preview = Anteprima
print-ready = Tutte le tue foto sono pronte per essere stampate.
print-too-small = Alcune delle tue foto sono troppo piccole per essere stampate bene.  Puoi rimuovere queste foto, stamparle comunque, oppure annullare e cambiare il formato di stampa.
print-quality-best = Qualità ottima ({ $ppi } pixel/pollice)
print-quality-good = Buona qualità ({ $ppi } pixel/pollice)
print-quality-bad = Cattiva qualità ({ $ppi } pixel/pollice)

## Results of background work (src/backend.rs)

backend-no-data-dir = Non è stata trovata nessuna cartella per i dati dell'applicazione.
backend-cache-fail = Non è stato possibile svuotare la cache ({ $error })
fail-export = Impossibile esportare: errore imprevisto.
fail-trash = Impossibile spostare le foto nel cestino: errore imprevisto.
fail-email = Impossibile preparare l'e-mail: errore imprevisto.
fail-gallery = Impossibile creare la galleria web: errore imprevisto.
fail-movie = Impossibile creare il filmato: errore imprevisto
fail-collage = Impossibile salvare il collage: errore imprevisto.
fail-download = Il download si è interrotto: errore imprevisto.
analysis-no-data-dir = Nessuna cartella per i dati dell'applicazione è stata trovata.
analysis-stopped = L'analisi delle foto è stata fermata: è fallita { $n } volte di seguito ({ $error }). Verrà ritentata al prossimo avvio di AlbumCraft.
pack-nothing-downloaded = Non è stato scaricato niente: { $error }.
pack-downloaded = { $pack } è scaricato ({ $size }).
pack-download-stopped = Il download di { $pack } è stato fermato. Avviandolo di nuovo riprende da dove era.
pack-download-failed = Impossibile scaricare { $pack }. { $error }
export-nothing = Non è stato esportato nulla: { $error }.
export-result =
    { $ok } su { $total ->
        [one] { $total } foto esportata
       *[other] { $total } foto esportate
    } in { $path }
export-stopped = L'esportazione è stata fermata prima della fine.
export-failed-list = Non esportate:
export-more = … e altre { $n }
gallery-ready =
    La galleria web è pronta:
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } foto non ha potuto essere letta ed è stata esclusa.
       *[other] { $n } foto non hanno potuto essere lette e sono state escluse.
    }
gallery-stopped = La galleria web non è stata creata: arresto richiesto.
gallery-failed = Impossibile creare la galleria web: { $error }
movie-ready =
    Il filmato è pronto:
    { $path }
movie-stopped = Il filmato non è stato creato: arresto richiesto.
movie-failed = Impossibile creare il filmato: { $error }
collage-saved-many =
    { $n } immagini salvate, la prima è:
    { $path }
collage-saved =
    Salvato:
    { $path }
collage-save-failed = Impossibile salvare il collage: { $error }
import-unexpected = L'importazione si è interrotta in modo imprevisto.
import-summary =
    { $n ->
        [one] { $n } foto importata in { $path }.
       *[other] { $n } foto importate in { $path }.
    }
import-already-there =
    { $n ->
        [one] { $n } era già presente.
       *[other] { $n } erano già presenti.
    }
import-was-stopped = L'importazione è stata fermata prima della fine.
import-failed-count = { $n } non hanno potuto essere copiate:

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = gen, feb, mar, apr, mag, giu, lug, ago, set, ott, nov, dic
locale-months-long = gennaio, febbraio, marzo, aprile, maggio, giugno, luglio, agosto, settembre, ottobre, novembre, dicembre
locale-date = { $day } { $month } { $year }
locale-decimal = ,
locale-group = .
locale-unit-kb = KB
locale-unit-mb = MB
locale-unit-gb = GB

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Inserisci un nome per la nuova cartella.
import-name-chars = Il nome di una cartella non può contenere / \ : * ? " < > |
import-name-long = Questo nome di cartella è troppo lungo.
import-drive = Disco { $letter }:
import-mkdir-fail = Non è stato possibile creare la cartella { $path }: { $error }
import-too-many = troppi file hanno questo nome
import-appeared = durante la copia è comparso un file con questo nome
export-logo-bad = il logo { $path } non è leggibile ({ $error })
export-unreadable = non è stato possibile leggere la foto
gallery-too-many = troppe gallerie hanno questo nome qui
gallery-none-read = non è stato possibile leggere nessuna delle foto
movie-no-pictures = nessuna foto
movie-ffmpeg-start = Non è stato possibile avviare FFmpeg: { $error }
movie-ffmpeg-input = FFmpeg non ha un ingresso
movie-ffmpeg-stopped = FFmpeg si è arrestato ({ $error })
movie-ffmpeg-failed =
    FFmpeg non è riuscito:
    { $tail }
root-already = Questa cartella è già nella libreria.
root-inside = Questa cartella è già nella libreria, dentro { $outer }.
root-excluded = That folder is inside { $folder }, which is left out of the library.
exclude-is-root = That is a library folder: remove it instead.
exclude-outside = That folder is not inside a library folder.
exclude-already = That folder is already left out.
online-tile-bad = non è stato possibile leggere una tessera di mappa: { $error }
online-map-down = Mappa online non raggiungibile ({ $error })
online-search-down = La ricerca online non è raggiungibile ({ $error }).
online-search-odd = Risposta imprevista dalla ricerca online: { $error }
collage-nothing = non c'è nulla da salvare
collage-extension = salva come .pdf, .png o .jpg
collage-page-fail = non è stato possibile disegnare una pagina
geo-near = Vicino a { $name }
mail-no-pictures = nessuna foto da allegare
mail-copies-fail = Impossibile creare le copie per l'e-mail: { $error }
mail-stopped = L'e-mail non è stata preparata: arresto richiesto.
mail-none-read = Impossibile creare le copie per l'e-mail: nessuna delle foto ha potuto essere letta.
mail-left-out =
    { $n ->
        [one] { $n } foto non ha potuto essere letta ed è stata esclusa.
       *[other] { $n } foto non hanno potuto essere lette e sono state escluse.
    }
mail-opened =
    Un nuovo messaggio con { $n ->
        [one] { $n } foto
       *[other] { $n } foto
    } è aperto nel tuo programma di posta.{ $left_out }
mail-asked =
    Al tuo programma di posta è stato chiesto un nuovo messaggio con { $n ->
        [one] { $n } foto
       *[other] { $n } foto
    }. Se non sono allegate, allegale da:
    { $path }{ $left_out }
mail-open-fail =
    Impossibile aprire il tuo programma di posta ({ $error }).
    Le piccole copie di { $n ->
        [one] { $n } foto
       *[other] { $n } foto
    } si trovano in:
    { $path }
    Allegale al tuo messaggio.{ $left_out }
mail-too-many = troppe foto
mail-no-mapi = nessun programma di posta (mapi32.dll: { $error })
mail-no-attach = il programma di posta non accetta allegati (nessun MAPISendMailW)
mail-refused = il programma di posta ha rifiutato il messaggio (errore MAPI { $code })
mail-open-mail = impossibile aprire Mail: { $error }
mail-open-mail-short = impossibile aprire Mail
mail-no-xdg = xdg-email (di xdg-utils) non è installato
mail-xdg-fail = impossibile avviare xdg-email: { $error }
gallery-html-lang = it
gallery-count = { $n ->
    [one] { $n } foto
   *[other] { $n } foto
}
gallery-close = Chiudi
gallery-prev = Precedente
gallery-next = Successiva
sheet-count = { $n ->
    [one] { $n } foto
   *[other] { $n } foto
}
