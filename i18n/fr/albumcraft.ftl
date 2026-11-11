# Texte de l'interface d'AlbumCraft, en français.

language-name = Français

## Shared buttons and words

button-ok = OK
button-cancel = Annuler
button-close = Fermer
button-remove = Retirer
button-save = Enregistrer
add-folder = Ajouter un dossier…
scan-again = Analyser à nouveau
removed-root = { $path } a été retiré de la bibliothèque. Les photos et vos notes sur elles sont intactes.

## Settings dialog

settings-title = Paramètres
settings-saved-note = Les modifications sont enregistrées au fur et à mesure.

settings-section-interface = Interface
settings-section-folders = Dossiers
settings-section-editing = Retouche
settings-section-slideshow = Diaporama
settings-section-export = Exportation
settings-section-maps = Cartes
settings-section-storage = Stockage
settings-section-ai = Fonctions d'IA

## Settings > Interface

settings-language = Langue
settings-language-system = Comme le système ({ $language })
settings-language-note = D'autres langues peuvent être ajoutées sous forme de fichiers de traduction ; voir i18n/ dans le code source.
settings-theme = Thème
theme-system = Comme le système
theme-light = Clair
theme-dark = Sombre

## Dialogs (src/dialogs.rs)

dialog-new-album = Nouvel album
dialog-rename-album = Renommer l'album
dialog-tags = Mots-clés et personnes
dialog-gallery = Galerie web
dialog-folder-note = Description du dossier
dialog-folder-manager = Dossiers de la bibliothèque
dialog-movie = Film
dialog-about = À propos d'AlbumCraft
dialog-online-maps = Cartes
dialog-print-review = Vérification
dialog-trash = Mettre à la corbeille

new-album-name = Nom de l'album :
new-album-adds = { $n ->
    [one] La photo sélectionnée y sera ajoutée.
   *[other] Les { $n } photos sélectionnées y seront ajoutées.
}
new-album-create = Créer
rename-album-rename = Renommer

tags-intro = { $n ->
    [one] À ajouter à la photo sélectionnée. Séparez les mots par des virgules.
   *[other] À ajouter aux { $n } photos sélectionnées. Séparez les mots par des virgules.
}
tags-tags = Mots-clés :
tags-people = Personnes :
tags-add = Ajouter

gallery-intro = { $n ->
    [one] Une page web avec une photo et vos retouches, lisible dans tout navigateur ou à mettre sur un hébergeur web.
   *[other] Une page web avec { $n } photos et vos retouches, lisible dans tout navigateur ou à mettre sur un hébergeur web.
}
gallery-title = Titre :
gallery-style = Style :
gallery-size = Taille des photos :
gallery-size-medium = Moyenne (1280 pixels, fichiers légers)
gallery-size-large = Grande (2048 pixels, pour les grands écrans)
gallery-captions = Afficher les légendes
gallery-make = Choisir le dossier et créer…

folder-note-intro = Une courte description affichée sous le nom du dossier :

folders-intro = La bibliothèque montre les photos de ces dossiers et de leurs sous-dossiers, là où elles sont : rien n'est copié ni déplacé.
folders-import-hint = Pour copier des photos d'une carte mémoire ou d'un appareil photo, utilisez Importer.
folders-none = (aucun pour l'instant — des photos d'exemple sont affichées à la place)

# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] Une vidéo d'une photo comme dans le diaporama : { $seconds } s
   *[other] Une vidéo de { $n } photos comme dans le diaporama : { $seconds } s chacune
}{ $fade ->
    [yes] , fondus enchaînés
   *[no] {""}
}{ $motion ->
    [yes] , panoramique et zoom
   *[no] {""}
}.
movie-length = Durée : environ { $seconds } s. Modifiez le minutage et les effets dans la barre du diaporama ou dans les Paramètres.
movie-music = Musique
movie-other-music = Autre musique…
movie-add-music = Ajouter une musique…
movie-no-ffmpeg = La création de films nécessite le logiciel libre FFmpeg, qui n'a pas été trouvé.
movie-install-ffmpeg = Installez-le depuis ffmpeg.org (ou le gestionnaire de paquets de votre système), puis rouvrez cette fenêtre.
movie-make = Choisir le fichier et créer…
movie-preview = Aperçu en diaporama

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] Mettre la photo rejetée ({ $size }) à la corbeille ?
   *[other] Mettre les { $count } photos rejetées ({ $size }) à la corbeille ?
}
trash-explain = Les fichiers sont retirés de leurs dossiers. Vous pouvez les récupérer dans la corbeille du système tant qu'elle n'a pas été vidée.
trash-move = Mettre à la corbeille

print-review-remove = Retirer ces photos
print-review-print = Imprimer quand même

about-text = Un organiseur de photos pour l'ordinateur : parcourez vos dossiers de photos, créez des albums, choisissez, rejetez et notez vos photos, corrigez-les et retouchez-les, et lancez des diaporamas.
about-edits = Les retouches ne modifient jamais vos fichiers originaux ; elles sont appliquées quand vous regardez ou exportez une photo.
about-credits = Carte intégrée : Natural Earth (domaine public). Liste des villes : GeoNames (CC BY 4.0). Cartes en ligne : © les contributeurs d'OpenStreetMap (ODbL).

online-maps-question = Afficher des cartes détaillées depuis Internet ?
online-maps-what = AlbumCraft peut montrer où vos photos ont été prises. Avec les cartes en ligne, il télécharge des plans d'OpenStreetMap et peut rechercher des adresses quand vous indiquez un lieu.
online-maps-privacy = Vos photos et leurs lieux ne sont jamais envoyés. Les serveurs de cartes voient votre adresse internet, les zones que vous regardez et les lieux que vous recherchez.
online-maps-offline = Sans les cartes en ligne, AlbumCraft utilise sa carte du monde et sa liste de villes intégrées, et ne se connecte pas à Internet.
online-maps-later = Vous pourrez changer cela plus tard sur la carte, ou dans les Paramètres (menu principal) : Cartes.
online-maps-use = Utiliser les cartes en ligne
online-maps-builtin = Carte intégrée seulement

## Notices at the bottom of the window

notice-undo = Annuler
notice-show = Afficher dans le dossier
notice-close = Fermer ce message
notice-no-file-manager = Impossible d'ouvrir le gestionnaire de fichiers : { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } photo
   *[other] { $n } photos
}

settings-bar = Barre en bas de la bibliothèque
settings-bar-compact = Compacte : une seule ligne, affichée quand des photos sont sélectionnées
settings-bar-tray = Plateau : miniatures des photos sélectionnées et grands boutons
settings-bar-note = Dans la barre compacte, les boutons perdent leur nom quand la fenêtre est étroite. Quand les boutons ne tiennent pas, la barre défile sur le côté.
settings-thumb-size = Taille des photos dans la bibliothèque

settings-folders-intro = La bibliothèque montre les photos de ces dossiers et de leurs sous-dossiers, là où elles sont : rien n'est copié ni déplacé. Pour copier des photos d'une carte mémoire ou d'un appareil photo, utilisez Importer.
settings-folders-none = Aucun dossier pour l'instant : des photos d'exemple sont affichées à la place.
settings-folders-remove-hint = Ne plus afficher ce dossier (ses photos et vos notes sur elles sont conservées)
settings-folders-rescan-hint = Chercher maintenant les photos nouvelles, modifiées et supprimées

settings-edit-opens = L'onglet Retouche s'ouvre sur
settings-edit-basic = Basique : corrections en un clic et outils
settings-edit-advanced = Avancé : les curseurs
settings-info-panel = Panneau d'infos
settings-info-show = Afficher le panneau d'infos dans l'éditeur (I)

settings-timing = Minutage
settings-next-after = Photo suivante après
settings-note-paused = Espace met le diaporama en pause, ← et → permettent de naviguer.
settings-note-manual = Chaque photo reste affichée jusqu'à ce que vous passiez à la suivante avec → ou les boutons. Le visionneur plein écran (F11) avance quand même tout seul.
settings-effects = Effets
settings-fade = Fondu enchaîné entre les photos
settings-motion = Panoramique et zoom lents
settings-motion-subtle = Discret
settings-motion-medium = Moyen
settings-captions = Afficher les légendes
settings-viewer-note = Le visionneur plein écran (F11 dans l'éditeur) démarre toujours sans fondu enchaîné ni panoramique et zoom.
settings-music = Musique
settings-music-play = Jouer de la musique pendant les diaporamas
settings-music-another = Choisir une autre…
settings-music-choose = Choisir une musique…
settings-movies-note = Les films faits à partir de photos utilisent aussi ces réglages.

settings-export-note = Ce avec quoi la vue Exportation démarre.
settings-export-size = Taille des photos
settings-export-quality = Qualité JPEG
settings-export-info = Informations dans les copies
settings-export-gps = Inclure le lieu de prise de vue (GPS)
settings-export-gps-hint = Désactivez avant de partager des photos si vous ne voulez pas que d'autres voient le lieu.
settings-export-exif = Inclure les informations de l'appareil (EXIF)
settings-export-exif-hint = Appareil, objectif, exposition et date de prise de vue, copiés depuis le fichier original. Le lieu suit le réglage ci-dessus.
settings-creator = Auteur
settings-creator-hint = Votre nom
settings-copyright = Copyright
settings-copyright-hint = © Votre nom
settings-credit-note = Écrits dans chaque copie (EXIF), à la place de ceux de l'original. Les copies envoyées par e-mail suivent aussi ces réglages. La signature ou le logo se choisit dans la vue Exportation.

settings-maps-online = Cartes en ligne
settings-maps-use = Utiliser les cartes en ligne (OpenStreetMap)
settings-maps-note = Plans détaillés et recherche d'adresses. Vos photos et leurs lieux ne sont jamais envoyés ; les serveurs de cartes voient votre adresse internet, les zones que vous regardez et les lieux que vous recherchez. Désactivé : seulement la carte du monde et la liste de villes intégrées, et aucune connexion à Internet.
settings-maps-tiles = Tuiles de carte téléchargées
settings-maps-tiles-after = Les tuiles de carte sont téléchargées à nouveau quand c'est nécessaire.

settings-notes = Vos notes
settings-notes-note = Les notes, légendes, mots-clés, retouches et albums sont gardés dans un seul fichier ; vos fichiers de photos ne sont jamais modifiés.
settings-thumbs = Miniatures
settings-thumbs-after = Les miniatures sont refaites à partir des photos quand c'est nécessaire (plus lent un moment).
cache-on-disk = { $size } sur le disque
cache-measuring = Mesure en cours…
cache-empty = Vider

settings-ai-soon = Pas encore disponible. Prévu :
settings-ai-faces = Visages
settings-ai-faces-text = repérer les visages et les regrouper par personne ; c'est vous qui nommez les groupes. Un téléchargement d'environ 10 à 40 Mo.
settings-ai-search = Recherche intelligente
settings-ai-search-text = trouver des photos en les décrivant (« neige », « gâteau d'anniversaire »). Légère (anglais, environ 100 Mo) ou multilingue (environ 400 Mo).
settings-ai-tags = Suggestions de mots-clés
settings-ai-tags-text = des mots-clés proposés d'après le contenu de la photo.
settings-ai-local = Tout sera calculé sur cet ordinateur ; rien n'est envoyé nulle part.

export-size-original = Taille d'origine
export-size-very-large = Très grande (2560 pixels)
export-size-large = Grande (1600 pixels)
export-size-small = Petite, pour l'e-mail (800 pixels)

## Export view (src/export_view.rs)

filter-pictures = Photos
view-back-editor = Éditeur
view-back-library = Bibliothèque
view-back-hint = Retour (Échap)

export-title = Exportation
export-count-all = { $n ->
    [one] { $n } photo
   *[other] { $n } photos
}
export-count-some = { $n } sur { $total } photos
export-select-first = Sélectionnez d'abord des photos.
export-logo-unreadable = Le logo { $path } est illisible.
export-task = { $n ->
    [one] Exportation de { $n } photo
   *[other] Exportation de { $n } photos
}
export-save-into = Enregistrer dans
export-choose-folder = Choisir un dossier…
export-choose-folder-hint = Choisir le dossier des copies
export-safe = Rien n'est écrasé ; les originaux ne sont pas modifiés.
export-go = { $n ->
    [one] Exporter { $n } photo
   *[other] Exporter { $n } photos
}
export-dialog-title = Exporter les photos vers…

export-size = Taille
export-size-other = Autre :
export-size-pixels = { " " }pixels
export-size-note = Le plus grand côté. Les photos ne sont jamais agrandies.
export-quality = Qualité JPEG

export-signature = Signature
export-signature-on = Ajouter une signature ou un logo
export-sig-text = Texte
export-sig-logo = Image (logo)
export-logo-other = Autre image…
export-logo-choose = Choisir une image…
export-logo-dialog = Image du logo ou de la signature
export-logo-note = Un PNG à fond transparent donne le meilleur résultat.
export-sig-text-hint = Votre nom, ou © Votre nom
export-sig-position = Position
export-sig-size = Taille
export-sig-size-hint = Par rapport au plus petit côté de la photo
export-sig-opacity = Opacité
export-sig-colour = Couleur
export-sig-shadow = Ombre
export-sig-shadow-hint = Un léger halo pour que le texte reste lisible sur toute photo

export-info = Informations dans les fichiers
export-gps = Lieu de prise de vue (GPS)
export-gps-hint = Désactivez avant de partager des photos si vous ne voulez pas que d'autres voient le lieu.
export-exif = Informations de l'appareil (EXIF)
export-exif-hint = Appareil, objectif, exposition et date de prise de vue, copiés depuis le fichier original.
export-creator = Auteur
export-creator-hint = Votre nom
export-copyright = Copyright
export-copyright-hint = © Votre nom
export-credit-note = Écrits dans chaque copie. La signature, l'auteur et le copyright sont conservés pour la prochaine fois (aussi dans les Paramètres, Exportation).

export-left-out = Exclue : cliquez pour l'exporter aussi
export-leave-out = Cliquez pour exclure cette photo

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = Toutes les photos
nav-picks = Choisies
nav-places = Lieux
nav-places-hint = Vos photos sur une carte
nav-albums = Albums
nav-people = Personnes
nav-folders = Dossiers
people-hint = Indiquez les personnes depuis la barre de sélection.
scanning = Recherche des photos…

bar-import = Importer
bar-import-hint = Copier des photos d'une carte mémoire, d'un appareil photo ou d'un dossier dans votre bibliothèque
bar-folders = Dossiers
bar-folders-hint = Les dossiers affichés dans la bibliothèque, là où ils sont (rien n'est copié)
bar-appearance = Apparence : { $mode } (cliquez pour changer)
bar-size-hint = Taille des photos
bar-picks-hint = Afficher seulement les photos choisies
bar-hide-rejects = Masquer les photos rejetées

sort-name = Nom du fichier
sort-taken = Date de prise de vue
sort-rating = Note
sort-hint = Ordre des photos : { $order }

filter-any = Toutes
filter-any-no-rejects = Toutes, sans les rejetées
filter-min = { $n }+
filter-min-no-rejects = { $n }+, sans les rejetées
filter-hint = Filtrer par note ; masquer les photos rejetées
filter-any-rating = Toutes les notes
filter-or-more = { $rating } ou plus

album-rename = Renommer…
album-delete = Supprimer l'album
album-show-map = Afficher sur la carte
album-slideshow = Lire en diaporama
album-deleted = L'album « { $name } » a été supprimé. Ses photos sont intactes.

empty-scanning = Recherche des photos…
empty-no-match = Aucune photo ne correspond à votre recherche ou à votre filtre.
empty-album = Cet album est vide. Sélectionnez des photos et ajoutez-les depuis la barre de sélection.
empty-person = Personne n'est encore indiqué.
empty-picks = Aucune photo choisie pour l'instant. Survolez une photo et cliquez sur son drapeau, ou appuyez sur P.
empty-unreachable = Aucune photo n'a été trouvée dans les dossiers de votre bibliothèque. Si elles sont sur un disque non connecté, connectez-le et analysez à nouveau.
empty-library = Votre bibliothèque est vide. Importez des photos, ou ajoutez un dossier que vous avez déjà.
empty-folders-hint = Les dossiers affichés dans la bibliothèque
empty-import = Importer des photos
empty-import-hint = Copier des photos d'une carte mémoire, d'un appareil photo ou d'un dossier
empty-add = Ajouter un dossier
empty-add-hint = Afficher les photos d'un dossier là où elles sont, sans les copier

folder-play-hint = Lire ce dossier en diaporama
folder-edit-hint = Modifier la description
folder-map-hint = Afficher ce dossier sur la carte
tile-pick-hint = Choisir (P)
tile-reject-hint = Rejeter (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = Aucune note
rating-stars = { $n ->
    [one] { $n } étoile
   *[other] { $n } étoiles
}
flag-pick = Choisir
flag-unpick = Ne plus choisir
flag-reject = Rejeter
flag-unreject = Ne plus rejeter
flag-rating = Note
album-new = Nouvel album…
menu-copy-edits = Copier les retouches
menu-paste-edits = Coller les retouches
menu-paste-n = { $n ->
    [one] Coller { $n } retouche
   *[other] Coller { $n } retouches
}
copy-edits-select-one = Sélectionnez une seule photo pour copier ses retouches.
menu-import = Importer des photos…
menu-add-folder = Ajouter un dossier à la bibliothèque…
menu-folders = Dossiers de la bibliothèque…
menu-scan = Analyser à nouveau les dossiers
menu-select-all = Tout sélectionner
menu-select-none = Ne rien sélectionner
menu-rotate-left = Pivoter à gauche
menu-rotate-right = Pivoter à droite
menu-trash-rejects = Mettre les photos rejetées à la corbeille…
menu-trash-rejects-hint = Retirer de leurs dossiers les fichiers des photos rejetées, après confirmation
menu-contact-sheet = Planche contact…
menu-contact-sheet-hint = Des pages de petites photos avec un titre et leurs noms de fichier, à imprimer ou à enregistrer en PDF
menu-undo-edits = Annuler toutes les retouches
menu-settings = Paramètres…
menu-about = À propos d'AlbumCraft…
menu-exit = Quitter
edits-removed = { $n ->
    [one] Les retouches de { $n } photo ont été retirées.
   *[other] Les retouches de { $n } photos ont été retirées.
}

status-library = Bibliothèque : { $folders ->
    [one] { $folders } dossier
   *[other] { $folders } dossiers
}, { $pictures ->
    [one] { $pictures } photo
   *[other] { $pictures } photos
}, { $size } sur le disque
status-importing = Importation de { $done } sur { $total }…
status-stop = Arrêter
status-stop-hint = Arrêter après la photo en cours (celles déjà copiées restent)
task-running = { $label }…
task-stopping = { $label } : arrêt en cours…
task-cancel-hint = Arrêter après la photo en cours
status-samples = Photos d'exemple affichées. Utilisez Importer ou Dossiers pour ajouter les vôtres.

selected-count = { $n ->
    [one] { $n } photo
   *[other] { $count } photos
}
selected-all = { $n ->
    [one] { $pictures } sélectionnée
   *[other] { $pictures } sélectionnées
}
selected-set = { $name } · { $pictures }
fallback-folder = Dossier
fallback-album = Album
tray-nothing = Aucune sélection
tray-click = Cliquez sur des photos pour les sélectionner

output-web = Page web
output-web-tip = Créer une galerie web avec ces photos
output-mail = E-mail
output-mail-tip = De petites copies, jointes à un nouveau message dans votre logiciel de messagerie
output-print = Imprimer
output-print-tip = Imprimer ces photos (Ctrl+P)
output-export = Exporter
output-export-tip = Enregistrer des copies retouchées dans un dossier
output-collage = Collage
output-collage-tip = Créer un collage ou une planche contact
output-movie = Film
output-movie-tip = Créer un film

bar-clear-hint = Effacer la sélection (Échap)
bar-pick-hint = Choisir / ne plus choisir (P)
bar-reject-hint = Rejeter / ne plus rejeter (X)
bar-rotate-left = Pivoter à gauche (Ctrl+[)
bar-rotate-right = Pivoter à droite (Ctrl+])
bar-tags = Mots-clés et personnes
bar-location = Définir le lieu : où ces photos ont été prises, sur une carte
bar-to-album = Vers un album
bar-clear = Effacer

ctx-open = Ouvrir dans l'éditeur
ctx-slideshow = Lire le diaporama à partir d'ici
ctx-add-to-album = Ajouter à un album
ctx-remove-from-album = Retirer de l'album
ctx-tags = Mots-clés et personnes…
ctx-location = Définir le lieu…
ctx-export = Exporter…
ctx-show-file = Afficher dans le gestionnaire de fichiers

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Éclatant
look-vivid-tip = Couleurs riches et saturées de film diapositive, avec des noirs profonds
look-soft-film = Film doux
look-soft-film-tip = Film portrait délicat : contraste doux, chaud et flatteur pour la peau
look-polaroid = Instantané
look-polaroid-tip = Tirage d'appareil instantané passé : noirs relevés, hautes lumières chaudes, ombres froides
look-cross-process = Cross-processing
look-cross-process-tip = Hautes lumières jaunes, ombres bleu-vert et contraste marqué
look-lomo = Lomo
look-lomo-tip = Rendu saturé d'appareil jouet, aux coins sombres
look-matte = Mat
look-matte-tip = Noirs plats et délavés, couleurs atténuées
look-teal-orange = Bleu-vert et orange
look-teal-orange-tip = Étalonnage cinéma : ombres bleu-vert, peau et hautes lumières chaudes
look-high-key = High key
look-high-key-tip = Lumineux et aéré, aux ombres ouvertes
look-cool = Froid
look-cool-tip = Une dominante plus froide, plus bleue
look-warm = Chaud
look-warm-tip = Une dominante plus chaude, dorée
look-bw = N & B
look-bw-tip = Noir et blanc classique
look-dramatic-bw = N & B dramatique
look-dramatic-bw-tip = Noir et blanc avec un filtre rouge : ciels sombres, fort contraste local
look-sepia = Sépia
look-sepia-tip = Ton brun de photo ancienne
look-soft-focus = Flou artistique
look-soft-focus-tip = Centre net, bords doux et rêveurs
look-glow = Halo lumineux
look-glow-tip = Un doux halo lumineux
look-film-grain = Grain de film
look-film-grain-tip = Grain façon pellicule
look-focal-bw = N & B focal
look-focal-bw-tip = Couleur au centre, noir et blanc autour
effect-sharpen = Netteté
effect-sepia = Sépia
effect-bw = N & B
effect-warmify = Réchauffer
effect-film-grain = Grain de film
effect-tint = Teinte
effect-saturation = Saturation
effect-soft-focus = Flou artistique
effect-glow = Halo lumineux
effect-filtered-bw = N & B filtré
effect-focal-bw = N & B focal
effect-graduated-tint = Teinte dégradée
op-rotate-right = Pivoter à droite
op-rotate-left = Pivoter à gauche
op-crop = Recadrer
op-straighten = Redresser { $angle }°
op-perspective-v = Perspective, verticale { $v }
op-perspective-h = Perspective, horizontale { $h }
op-perspective-both = Perspective { $v } / { $h }
op-redeye = Yeux rouges
op-lucky = Correction auto (chance)
op-auto-contrast = Contraste auto
op-auto-color = Couleur auto
op-tuning = Réglage
op-look = { $look } { $amount } %
op-adjust = Ajustement
op-adjust-parts = Ajustement : { $parts }
adj-wb = balance des blancs
adj-exposure = exposition
adj-highlights = hautes lumières
adj-shadows = ombres
adj-blacks = noirs
adj-contrast = contraste
adj-saturation = saturation
adj-vibrance = éclat
adj-clarity = clarté
adj-sharpen = netteté
adj-noise = réduction du bruit
decor-vignette-light = Vignettage clair
decor-vignette = Vignettage
decor-instant = Cadre d'instantané
decor-border = Bordure
decor-shadow = Ombre portée
decor-rounded = Coins arrondis
decor-text = Texte « { $text } »
wb-as-shot = Tel quel
wb-daylight = Lumière du jour
wb-cloudy = Nuageux
wb-shade = Ombre
wb-tungsten = Tungstène
wb-fluorescent = Fluorescent
wb-flash = Flash
wb-custom = Personnalisé
wb-auto = Auto
gallery-style-light = Grille claire
gallery-style-dark = Grille sombre
gallery-style-masonry = Mosaïque (photos entières)
crop-free = Libre
crop-original = Forme d'origine
crop-square = Carré
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (tirage 10 × 15)
crop-16-9 = 16 × 9 (écran large)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Double-clic pour réinitialiser
edit-loading = La photo est encore en cours de chargement.
edit-straightened = Redressée de { $angle }°.
edit-already-level = Cette photo semble déjà droite.
edit-no-horizon = Aucun horizon ni aucune ligne verticale nette trouvés pour redresser.
edit-histogram = Histogramme et infos de l'appareil
edit-unknown-camera = Appareil inconnu
edit-35mm = (équivalent 35 mm : { $mm } mm)
edit-focal = Focale : { $mm } mm
edit-iso = ISO : { $iso }
edit-no-camera = Aucune information d'appareil dans cette photo
edit-clip-shadows = ombres { $pct } %
edit-clip-highlights = hautes lumières { $pct } %
edit-clipped = Écrêtage : { $parts }
edit-clipped-hint = Part de la photo qui est noir pur ou blanc pur dans au moins une couleur
edit-back = Retour à la bibliothèque
edit-info = Infos
edit-info-hint = Propriétés, lieu et mots-clés (I)
edit-side-by-side-hint = Afficher les photos côte à côte
edit-stacked-hint = Afficher une photo au-dessus de l'autre
edit-focus-hint = Changer la photo sur laquelle porte la retouche (là où vont vos modifications)
edit-compare = Comparer
edit-compare-hint = Cette photo à côté d'une autre, côte à côte
edit-original = Original
edit-original-hint = Maintenir pour voir la photo sans vos retouches (ou maintenir O)
edit-slideshow = Diaporama
edit-fullscreen-hint = Plein écran (F11) : à partir de cette photo, l'une après l'autre, avec de la musique
edit-next = Photo suivante
edit-previous = Photo précédente
edit-position = { $n } sur { $total }
edit-tab-edit = Retouche
edit-tab-presets = Préréglages
edit-tab-effects = Effets
edit-undo = Annuler
edit-undo-hint = Annuler la dernière modification (Ctrl+Z)
edit-redo = Rétablir
edit-list = Retouches de cette photo :
edit-list-none = (aucune — c'est l'original)
edit-step-hint =
    { $label }
    Cliquez pour revenir à cette étape (les suivantes sont retirées ; Annuler les ramène)
edit-copy-hint = Copier ces retouches pour les coller sur d'autres photos (Ctrl+Maj+C)
edit-paste-n = Coller { $n }
edit-paste-hint = Ajouter les retouches copiées à cette photo (Ctrl+Maj+V)
edit-basic = Basique
edit-advanced = Avancé
edit-wb-help = Cliquez sur un élément de la photo qui devrait être gris neutre ou blanc.
edit-crop-help = Faites glisser les bords ou les coins du cadre, glissez à l'intérieur pour le déplacer, ou tracez un nouveau cadre. Entrée applique.
edit-crop-shape = Forme
edit-crop-orientation = Orientation
edit-landscape = Paysage
edit-landscape-hint = Plus large que haut
edit-portrait = Portrait
edit-portrait-hint = Plus haut que large
edit-apply = Appliquer
edit-remove-crop = Retirer le recadrage
edit-remove-crop-hint = Afficher de nouveau toute la photo
edit-redeye-help = Tracez un cadre autour des yeux rouges. Seules les pupilles rouges à l'intérieur du cadre sont assombries.
edit-straighten-title = Redresser
edit-straighten-help = Déplacez le curseur jusqu'à ce que l'horizon soit aligné sur la grille.
edit-oneclick = Corrections en un clic
edit-lucky = Je me sens chanceux
edit-lucky-hint = Exposition, balance des blancs, point noir et un peu d'éclat, tout à la fois (puis affinez-les dans Avancé)
edit-auto-exposure = Exposition auto
edit-auto-exposure-hint = Éclaircir ou assombrir pour une exposition équilibrée
edit-auto-colour = Couleur auto
edit-auto-colour-hint = Balance des blancs auto : supprimer une dominante de couleur
edit-auto-contrast = Contraste auto
edit-auto-contrast-hint = Étirer les tons les plus sombres et les plus clairs
edit-auto-straighten = Redressement auto
edit-auto-straighten-hint = Redresser un horizon ou des verticales penchés
edit-tools = Outils
edit-pick-neutral = Point neutre
edit-pick-neutral-hint = Cliquez sur un élément gris pour régler la balance des blancs
edit-sec-wb = BALANCE DES BLANCS
edit-sec-light = LUMIÈRE
edit-sec-colour = COULEUR
edit-sec-detail = DÉTAIL
edit-sec-geometry = GÉOMÉTRIE
edit-wb-pick = Cibler
edit-wb-pick-hint = Cliquez sur un élément gris de la photo
edit-s-temperature = Température
edit-s-temperature-tip = Plus bleu (à gauche) ou plus chaud (à droite)
edit-s-tint = Teinte
edit-s-tint-tip = Plus vert (à gauche) ou plus magenta (à droite)
edit-s-exposure = Exposition
edit-s-exposure-tip = Luminosité globale, en diaphragmes
edit-s-highlights = Hautes lumières
edit-s-highlights-tip = Zones claires : à gauche récupère du détail, à droite éclaircit
edit-s-shadows = Ombres
edit-s-shadows-tip = Zones sombres : à droite les ouvre
edit-s-blacks = Noirs
edit-s-blacks-tip = Point noir : à gauche approfondit, à droite relève
edit-s-contrast = Contraste
edit-s-contrast-tip = Écart entre le sombre et le clair
edit-s-saturation = Saturation
edit-s-saturation-tip = Toutes les couleurs
edit-s-vibrance = Éclat
edit-s-vibrance-tip = Surtout les couleurs ternes ; ménage la peau et les couleurs déjà vives
edit-s-clarity = Clarté
edit-s-clarity-tip = Contraste local dans les tons moyens
edit-s-sharpen = Netteté
edit-s-sharpen-tip = Détails fins plus nets
edit-s-noise = Bruit
edit-s-noise-tip = Réduction du bruit : lisse le grain en gardant les contours
edit-reset-all = Tout réinitialiser
edit-crop-btn = Recadrer…
edit-crop-btn-hint = Choisir la partie de la photo à garder, ou modifier le recadrage
edit-s-straighten-tip = Tourner un peu la photo pour redresser l'horizon (zoome pour masquer les coins)
edit-s-vertical = Vertical
edit-s-vertical-tip = Perspective : à droite élargit le haut, pour redresser les bâtiments qui penchent en arrière quand on photographie d'en bas
edit-s-horizontal = Horizontal
edit-s-horizontal-tip = Perspective : à droite agrandit le côté droit, pour un mur vu en biais
edit-intensity = Intensité
edit-remove-preset = Retirer le préréglage
edit-presets-help = Des rendus pour toute la photo. Cliquez sur l'un d'eux, puis réglez son intensité.
edit-effects-add = Ajouter sur ou autour de la photo
edit-add-instant = Cadre d'instantané
edit-add-text = Texte
edit-your-text = Votre texte
edit-effects-note = Les effets s'appliquent par-dessus toutes les autres retouches. Ajoutez-en un pour le régler ici.
edit-p-amount = Intensité
edit-p-size = Taille
edit-p-width = Largeur
edit-p-bottom = Bas
edit-p-colour = Couleur
edit-p-darkness = Obscurité
edit-p-background = Fond
edit-p-radius = Rayon
edit-vignette-note = À gauche du milieu, les coins sont éclaircis à la place.
edit-halo = Halo léger (lisible sur toute photo)
edit-remove-effect = Retirer cet effet
edit-caption-hint = Ajoutez une légende !
edit-print-hint = Imprimer cette photo (Ctrl+P)
edit-album = Album
edit-zoom-hint = Zoom (100 % = les pixels réels de la photo)
edit-zoom-slider-hint = Zoom : à gauche = ajusté, au milieu = 1:1, à droite = 400 % (la molette fonctionne aussi ; faites glisser la photo pour zoomer à 100 % et la déplacer)
edit-one-to-one-hint = Taille d'origine (100 %) : un pixel de la photo par pixel d'écran
edit-fit = Ajuster
edit-fit-hint = Ajuster toute la photo à la fenêtre (un double-clic sur une photo zoomée fait pareil)
edit-people = Personnes :
edit-person-remove = Retirer la personne
edit-person-add = ajouter une personne
edit-compare-none = Aucune autre photo dans ce dossier
edit-compare-selected = Sélectionnée
edit-compare-swap-hint = Cliquez pour retoucher plutôt cette photo
edit-view-loading = Chargement…
edit-sharpening = Netteté en cours…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = Cette photo est illisible.
edit-full-unreadable = La photo complète est illisible.
edit-full-loading = Chargement de la photo complète…
edit-draw-box = Faites glisser sur la photo pour tracer un cadre
edit-click-neutral = Cliquez sur un élément qui devrait être gris ou blanc
edit-wb-title = Balance des blancs
