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
tags-proposed = Proposés :
tags-proposed-hint = Ajouter ce mot-clé (clic droit : il ne convient pas)
tags-not-this = Pas « { $tag } » : ne plus le proposer ici

gallery-intro = { $n ->
    [one] Une page web avec une photo et vos retouches, lisible dans tout navigateur ou à mettre sur un hébergeur web.
   *[other] Une page web avec { $n } photos et vos retouches, lisible dans tout navigateur ou à mettre sur un hébergeur web.
}
gallery-title = Titre :
gallery-theme = Thème :
gallery-layout = Disposition :
gallery-size = Taille des photos :
gallery-size-medium = Moyenne (1280 pixels, fichiers légers)
gallery-size-large = Grande (2048 pixels, pour les grands écrans)
gallery-captions = Afficher les légendes
gallery-make = Choisir le dossier et créer…

folder-note-intro = Une courte description affichée sous le nom du dossier :


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
settings-excluded-title = Dossiers exclus
settings-excluded-intro = Dossiers d'un dossier de la bibliothèque qui ne sont pas affichés, avec tout ce qu'ils contiennent (par exemple Screenshots).
settings-excluded-none = Rien n’est exclu.
settings-excluded-add = Exclure un dossier
settings-excluded-remove-hint = Afficher de nouveau ce dossier

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
settings-ai-intro = Repérer les visages, chercher en décrivant une photo, proposer des mots-clés. Tout est calculé sur cet ordinateur : aucune photo, aucun visage ni aucun texte de recherche n'est envoyé nulle part. Les réseaux dont ces fonctions ont besoin ne sont téléchargés que si vous le demandez, depuis les versions publiées d'AlbumCraft.
settings-ai-unavailable = Cette version d'AlbumCraft ne peut pas faire tourner de réseaux : ces fonctions n'y sont pas disponibles.
settings-ai-networks = Réseaux
settings-ai-pack-size = { $size }  ·  licence : { $licence }
settings-ai-remove = Supprimer
settings-ai-remove-hint = Supprimer les réseaux de cet ordinateur. Ce qui a été trouvé dans vos photos est conservé.
settings-ai-downloaded = Téléchargé
settings-ai-downloading = Téléchargement… (voir la barre en bas)
settings-ai-download = Télécharger
settings-ai-remove-failed = Les réseaux n'ont pas pu être supprimés : { $error }
settings-ai-search-uses = La recherche intelligente utilise
settings-ai-search-auto = Le pack adapté à la langue de l'interface
settings-ai-search-auto-hint = Le pack multilingue quand l'interface n'est pas en anglais, sinon le pack anglais
settings-ai-download-task = Téléchargement de { $pack }
settings-ai-analysis = Analyse de la photothèque
settings-ai-analysis-text = Chaque photo est analysée une fois, en arrière-plan, quand AlbumCraft n'a rien d'autre à faire. L'analyse s'arrête à la fermeture d'AlbumCraft et reprend au démarrage suivant.
settings-ai-progress = { $n ->
    [one] { $done } photo analysée sur { $total }
   *[other] { $done } photos analysées sur { $total }
}
settings-ai-progress-paused = { $n ->
    [one] { $done } photo analysée sur { $total } (en pause)
   *[other] { $done } photos analysées sur { $total } (en pause)
}
settings-ai-all-analysed = Toutes les photos sont analysées.
settings-ai-nothing = Rien à analyser : aucun réseau n'est téléchargé.
settings-ai-pause = Mettre l'analyse en pause

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
status-analysis = Analyse des photos { $done } / { $total }
status-analysis-paused = Analyse des photos en pause ({ $done } / { $total })
status-analysis-pause = Pause
status-analysis-pause-hint = Arrêter l'analyse pour l'instant. Ce qui est fait est conservé ; elle s'arrête aussi à la fermeture d'AlbumCraft et reprend au démarrage suivant.
status-analysis-resume = Reprendre
status-analysis-resume-hint = Continuer d'analyser les photos en arrière-plan

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
ctx-more-like-this = Plus de photos semblables
ctx-more-like-this-hint = Afficher les photos qui ressemblent à celle-ci
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
gallery-theme-paper = Papier
gallery-theme-night = Nuit
gallery-theme-museum = Musée
gallery-theme-darkroom = Chambre noire
gallery-theme-instant = Instantanés
gallery-theme-ocean = Océan
gallery-theme-vintage = Rétro
gallery-theme-blossom = Pétales
gallery-layout-grid = Grille carrée
gallery-layout-masonry = Colonnes
gallery-layout-rows = Rangées
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

## Info panel, search, slideshow bar

info-hide-hint = Masquer ce panneau (I)
info-properties = Propriétés
info-camera = Appareil
info-lens = Objectif
info-taken = Prise le
info-file-date = { $date } (date du fichier)
info-size = Taille
info-megapixels = { $w } × { $h } ({ $mp } Mpx)
info-file = Fichier
info-focal = Focale
info-focal-35 = { $f } mm (35 mm : { $e } mm)
info-aperture = Ouverture
info-exposure = Exposition
info-folder = Dossier
info-sample = Photo d'exemple intégrée
info-edits = Retouches
info-no-exif = Aucune information d'appareil (EXIF) dans cette photo.
info-exif = EXIF détaillées
info-flash = Flash
info-bias = Correction d'exposition
info-program = Programme
info-metering = Mesure
info-distance = Distance
info-colour-space = Espace colorimétrique
info-gps = Position GPS
info-digitized = Numérisée
info-modified = Modifiée
info-software = Logiciel
info-preview = Aperçu de l'appareil
info-preview-hint = { $w } × { $h } pixels, stocké dans le fichier
info-all-exif = Toutes les données EXIF ({ $n } entrées)
info-location = Lieu
info-set-in-app = Défini dans AlbumCraft
info-from-gps = Depuis le GPS de l'appareil
info-modify = Modifier…
info-modify-hint = Déplacer l'épingle, renommer le lieu ou le supprimer
info-show-map-hint = Ouvrir Lieux sur cette photo
info-no-location = Aucun lieu pour l'instant.
info-set-location-hint = Trouver le lieu ou cliquer dessus sur une carte
info-tags = Mots-clés
info-no-tags = Aucun mot-clé pour l'instant.
info-remove-tag = Retirer ce mot-clé
info-add-tag = Ajouter un mot-clé…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = E
coord-w = O
info-orient-2 = En miroir
info-orient-3 = À l'envers
info-orient-4 = En miroir, à l'envers
info-orient-5 = En miroir, tournée à gauche
info-orient-6 = Tournée à droite (appareil tenu droit)
info-orient-7 = En miroir, tournée à droite
info-orient-8 = Tournée à gauche (appareil tenu droit)
search-remove-hint = Retirer de la recherche
search-hint = Rechercher mots-clés, lieux, légendes…
search-and = et…
search-clear-hint = Effacer la recherche
search-for = Rechercher « { $text } »
search-kind-tags = Mots-clés
search-like-chip = Comme { $name }
search-looks-like-title = Ressemble à
search-similar-title = Photos semblables
search-still-analysing = (analyse en cours : certaines photos ne sont pas encore classées)
search-show-more = Afficher plus
search-show-more-hint = { $n ->
    [one] Une autre photo correspond moins bien
   *[other] { $count } autres photos correspondent moins bien
}
search-like-no-pack = Pour trouver des photos semblables, téléchargez un pack de recherche dans Paramètres › Fonctions d'IA.
search-failed-because = La recherche intelligente a échoué : { $error }
search-failed = la recherche s'est arrêtée de façon inattendue
search-no-folder = AlbumCraft n'a pas de dossier pour ses données
search-pack-failed = La recherche intelligente n'a pas pu démarrer : { $error }
search-not-analysed = Cette photo n'est pas encore analysée.
music-dialog-title = Musique pour le diaporama
filter-music = Musique
ss-previous = Précédente (←)
ss-play-pause = Lecture / pause (Espace)
ss-next = Suivante (→)
ss-seconds = Secondes par photo
ss-interval-off = Désactivé : les photos ne changent qu'avec → ou les boutons (Paramètres, Diaporama)
ss-fade = Fondu
ss-motion = Mouvement
ss-other-music-hint = Choisir un autre fichier de musique
ss-exit-full = Quitter le plein écran (F11 ou Échap)
ss-exit = Quitter (Échap)

## Messages of the window (src/app.rs)

save-failed-prefix = Votre bibliothèque n'a pas pu être enregistrée :
save-failed =
    { $prefix } { $error }
    Vos dernières modifications ne sont pour l'instant que dans cette fenêtre ; AlbumCraft continue d'essayer.
save-ok-again = Votre bibliothèque est de nouveau enregistrée.
lib-unreadable-kept =
    Le fichier de votre bibliothèque n'a pas pu être lu ({ $error }).

    Il a été conservé sous :
    { $backup }

    AlbumCraft a démarré avec une bibliothèque vide. Vos photos elles-mêmes ne sont pas touchées.
lib-unreadable-nosave =
    Le fichier de votre bibliothèque n'a pas pu être lu ({ $error }).

    AlbumCraft a démarré avec une bibliothèque vide et n'enregistrera aucune modification cette fois, pour que le fichier ne soit pas écrasé.
dialog-library = Bibliothèque
crash-title = AlbumCraft s'est fermé de manière inattendue
crash-text =
    AlbumCraft s'est arrêté à cause d'une erreur lors de sa dernière exécution. Les modifications faites dans les toutes dernières secondes peuvent manquer.

    Les détails ont été écrits dans :
    { $log }

    Merci de joindre ce fichier si vous signalez le problème.
picasa-imported =
    { $n ->
        [one] Notes, légendes et retouches importées depuis le fichier .picasa.ini de { $n } dossier.
       *[other] Notes, légendes et retouches importées depuis les fichiers .picasa.ini de { $n } dossiers.
    }
copy-no-edits = Cette photo n'a aucune retouche à copier.
copied-edits =
    { $n ->
        [one] { $n } retouche copiée. Sélectionnez des photos et collez-la (Ctrl+Maj+V).
       *[other] { $n } retouches copiées. Sélectionnez des photos et collez-les (Ctrl+Maj+V).
    }
paste-first = Copiez d'abord les retouches d'une photo (Ctrl+Maj+C).
pasted-edits =
    { $n ->
        [one] Retouches collées sur { $n } photo.
       *[other] Retouches collées sur { $n } photos.
    }
no-pictures-to-show = Il n'y a aucune photo à afficher.
music-error = Pas de musique : { $error }
choose-folder-dialog = Choisissez un dossier de photos
added-to-album =
    { $n ->
        [one] { $n } photo ajoutée à « { $name } ».
       *[other] { $n } photos ajoutées à « { $name } ».
    }
trash-samples = Les photos d'exemple ne peuvent pas être mises à la corbeille.
trash-no-rejects = Aucune photo n'est rejetée. Appuyez sur X pour en rejeter une.
task-trash =
    { $n ->
        [one] Mise de { $n } photo à la corbeille
       *[other] Mise de { $n } photos à la corbeille
    }
task-email =
    { $n ->
        [one] Préparation de { $n } photo pour un e-mail
       *[other] Préparation de { $n } photos pour un e-mail
    }
default-title = Mes photos
gallery-folder-dialog = Créer la galerie web dans…
task-gallery =
    { $n ->
        [one] Création de la galerie web de { $n } photo
       *[other] Création de la galerie web de { $n } photos
    }
movie-save-dialog = Enregistrer le film sous…
filter-mp4 = Vidéo MP4
task-movie = Création du film
scan-truncated = Seules les { $n } premières photos sont affichées : retirez des dossiers de la bibliothèque pour voir les autres.
selection-none = Aucune sélection
selection-one = { $name }     { $date }     { $w }x{ $h } pixels     { $size }
selection-several = { $pictures }     { $a }      { $size } sur le disque
selection-range = { $pictures }     du { $a } au { $b }     { $size } sur le disque

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Tas de photos
cstyle-pile-tip = Des photos éparpillées, comme des tirages jetés sur une table
cstyle-mosaic = Mosaïque
cstyle-mosaic-tip = Des photos de tailles différentes remplissent la page
cstyle-frame = Photo centrale
cstyle-frame-tip = Une mosaïque autour d'une grande photo au milieu
cstyle-grid = Grille
cstyle-grid-tip = Des lignes et des colonnes régulières
cstyle-film = Pellicule
cstyle-film-tip = Des photos sur des bandes de pellicule
cstyle-clothesline = Corde à linge
cstyle-clothesline-tip = Des photos instantanées pincées sur des cordes
cstyle-fan = Éventail
cstyle-fan-tip = Étalées comme un jeu de cartes
cstyle-shape = Forme
cstyle-shape-tip = Un cœur, un cercle ou une étoile fait de photos
cstyle-blend = Exposition multiple
cstyle-blend-tip = Les photos vues les unes à travers les autres (idéal avec 2 à 4)
cstyle-sheet = Planche contact
cstyle-sheet-tip = Des pages de petites photos avec un titre et leurs noms de fichier
shape-heart = Cœur
shape-circle = Cercle
shape-star = Étoile
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = Lettre US
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Carré
fmt-wide = Écran (16:9)
grad-vertical = De haut en bas
grad-diagonal = Diagonale
grad-radial = Depuis le centre
collage-need-sheet = Sélectionnez d'abord les photos de la planche contact.
collage-need-two = Sélectionnez au moins deux photos pour créer un collage.
collage-unreadable = Cette photo n'a pas pu être lue.
collage-new-layout = Nouvelle disposition
collage-new-layout-hint = Autres positions, rotations et tailles
collage-shuffle = Mélanger
collage-shuffle-hint = Mettre les photos dans un ordre aléatoire
collage-spacing = Espace entre les photos
border-none = Aucune
border-white = Blanche
border-instant = Instantané
collage-frame-text = Texte sur les cadres
cap-none = Aucun
cap-caption = Légende
collage-shadows = Ombres
bg-gradient = Dégradé
bg-picture = Image
collage-to = à
collage-use-selected = Utiliser la photo sélectionnée
collage-use-selected-hint = Cliquez d'abord sur une photo du collage
collage-choose-file = Choisir un fichier…
collage-no-bg = Aucune image choisie pour l'instant
collage-bg-dialog = Image d'arrière-plan
collage-page = Page
collage-size = Taille de l'image
collage-px = { $px } pixels
collage-centre = Mettre au milieu
collage-put-back = Remettre
collage-put-back-hint = Annuler les déplacements de cette photo
collage-leave-out = Exclure
collage-leave-out-hint = Retirer cette photo du collage (Suppr)
collage-title = Titre
collage-no-title = Sans titre
collage-no-subtitle = Pas de deuxième ligne
collage-header = Titre sur la première page
collage-under = Sous chaque photo
collage-date = Date
collage-pictures = Photos
collage-page-numbers = Numéros de page
collage-sending = Envoi de la page { $page } sur { $total } à l'imprimante…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }
collage-no-printer = Aucune imprimante disponible
collage-save-pdf = Enregistrer en PDF…
collage-save-picture = Enregistrer l'image…
collage-save-picture-hint = Enregistrer le collage en image JPEG ou PNG
collage-file-sheet = Planche contact
collage-file-collage = Collage
collage-save-pdf-title = Enregistrer en PDF
collage-save-picture-title = Enregistrer le collage en image
filter-pdf = Document PDF
filter-jpeg = Image JPEG
filter-png = Image PNG
collage-saving-sheet = Enregistrement de la planche contact…
collage-saving = Enregistrement du collage…
task-save-collage = Enregistrement du collage
collage-printed =
    { $n ->
        [one] { $n } page envoyée à { $printer }.
       *[other] { $n } pages envoyées à { $printer }.
    }
collage-print-failed = L'impression ne s'est pas terminée : { $error }
collage-hint-swap = Faites glisser une photo sur une autre pour les échanger. Cliquez sur une photo pour la sélectionner (Suppr l'exclut).
collage-hint-free = Faites glisser une photo pour la déplacer, faites défiler dessus pour la faire tourner. Cliquez sur une photo pour la sélectionner (Suppr l'exclut).
collage-page-of = Page { $page } sur { $total }

## Places map (src/map_view.rs)

map-search-still = Recherche des positions GPS de vos photos en cours : réessayez dans un instant.
map-none-located =
    { $n ->
        [one] Cette photo n'a pas encore de lieu. Sélectionnez-la et cliquez sur { $icon } (Définir le lieu) dans la barre de sélection pour la placer.
       *[other] Aucune de ces { $n } photos n'a encore de lieu. Sélectionnez-les et cliquez sur { $icon } (Définir le lieu) dans la barre de sélection pour les placer.
    }
map-location-set =
    { $n ->
        [one] Lieu défini pour { $n } photo.
       *[other] Lieu défini pour { $n } photos.
    }
map-location-removed =
    { $n ->
        [one] Lieu retiré de { $n } photo.
       *[other] Lieu retiré de { $n } photos.
    }
map-nothing-online = Rien trouvé en ligne.
map-set-location = Définir le lieu
map-sub-scope =
    { $n } sur { $total ->
        [one] { $total } photo
       *[other] { $total } photos
    } avec un lieu
map-sub-indexing = { $n } avec un lieu · recherche des positions GPS…
map-sub-all =
    { $n ->
        [one] { $n } photo avec un lieu
       *[other] { $n } photos avec un lieu
    }
map-title-scope = Lieux · { $title }
map-show-whole = Afficher toute la bibliothèque
map-online = Carte en ligne
map-builtin = Carte intégrée
map-online-tip = OpenStreetMap est affiché. Cliquez pour utiliser seulement la carte intégrée (aucune connexion à Internet).
map-builtin-tip = La carte du monde intégrée est affichée. Cliquez pour afficher les cartes détaillées d'OpenStreetMap (télécharge des images de carte depuis Internet).
map-show-all = Tout afficher
map-show-all-hint = Zoomer pour montrer toutes les photos sur la carte
map-search-online-hint = Rechercher un lieu ou une adresse
map-search-builtin-hint = Rechercher une ville ou des coordonnées
map-no-city-builtin = Aucune ville de ce nom dans la liste intégrée.
map-no-city-hint = Aucune ville de ce nom. Essayez des coordonnées, comme 44,84, -0,58.
map-cities = VILLES
map-cities-builtin = VILLES (LISTE INTÉGRÉE)
map-searching = Recherche en ligne…
map-places-osm = LIEUX ET ADRESSES (OPENSTREETMAP)
map-search-addresses = Rechercher des adresses en ligne   (Entrée)
map-where = Où ont-elles été prises ?
map-where-help = Recherchez un lieu ci-dessus, ou cliquez sur la carte pour poser une épingle. Vous pourrez ensuite la faire glisser.
map-name-hint = Nom du lieu (facultatif)
map-drag-pin = Faites glisser l'épingle pour l'ajuster.
map-drag-pin-offline = Faites glisser l'épingle pour l'ajuster. Les cartes en ligne montrent les rues.
map-set-n =
    { $n ->
        [one] Définir le lieu de { $n } photo
       *[other] Définir le lieu de { $n } photos
    }
map-remove-location = Retirer le lieu
map-remove-location-hint = Ces photos n'auront aucun lieu, même si l'appareil en avait enregistré un (les fichiers ne sont pas modifiés)
map-close-hint = Fermer (Échap)
map-change-location = Modifier le lieu…
map-select-in-library = Sélectionner dans la bibliothèque
map-select-in-library-hint = Sélectionner ces photos dans la bibliothèque
map-show-hint = Afficher sur la carte
map-preparing = Préparation de la carte…
map-hover = { $pictures } · { $place }
map-credit-builtin = Carte intégrée : Natural Earth · GeoNames
map-problem = { $problem }. La carte intégrée est affichée.
map-empty-indexing = Recherche des positions GPS de vos photos…
map-empty =
    Aucune photo avec un lieu pour l'instant.

    Les photos des téléphones et des appareils GPS apparaissent ici toutes seules. Pour placer les autres, sélectionnez-les et cliquez sur { $icon } (Définir le lieu) dans la barre de sélection.

## Import (src/import_view.rs)

import-subtitle = Copier des photos dans votre bibliothèque
import-from = Depuis
import-choose = Choisir…
import-refresh-hint = Chercher à nouveau les cartes et les disques, et relire le dossier
import-hide-dups = Masquer les { $n } déjà dans la bibliothèque
import-dups-hint = Photos au même contenu qu'une photo de votre bibliothèque, même sous un autre nom
import-uncheck-all = Tout décocher
import-check-all = Tout cocher
import-copy-into = Copier dans
import-another-folder = Un autre dossier…
import-name-hint = Nom du nouveau dossier, p. ex. Été en Espagne
import-checked = { $chosen } sur { $shown } cochées. Rien n'est écrasé ; les originaux ne sont pas modifiés.
import-go =
    { $n ->
        [one] Importer { $n } photo
       *[other] Importer { $n } photos
    }
import-dest-dialog = Placer le nouveau dossier dans…
import-from-dialog = Importer des photos depuis…
import-pick-dest = Choisissez où placer le nouveau dossier.
import-running = Une importation est encore en cours : attendez qu'elle se termine (voir la barre d'état).
import-started =
    { $n ->
        [one] Importation de { $n } photo dans { $path }…
       *[other] Importation de { $n } photos dans { $path }…
    }
import-listing-in = Recherche des photos dans { $from }…
import-none-in = Aucune photo dans { $path }.
import-choose-another = Choisir un autre dossier…
import-heading = Importer depuis une carte mémoire, un appareil photo ou un dossier
import-copy-note = Les photos sont copiées dans un nouveau dossier de votre bibliothèque. Les originaux ne sont pas modifiés.
import-no-cards = Aucune carte mémoire ni aucun appareil photo trouvé. Branchez-en un et cliquez sur { $icon } ci-dessus.
import-instead = Pour afficher des photos là où elles sont déjà, sans les copier :
import-add-instead = ajoutez plutôt un dossier à la bibliothèque
import-all-dups =
    { $n ->
        [one] La photo est déjà dans votre bibliothèque.
       *[other] Les { $n } photos sont toutes déjà dans votre bibliothèque.
    }
import-show-anyway = Les afficher quand même
import-truncated = Seules les { $n } premières photos sont listées.
import-group-hint = Cocher ou décocher tout ce groupe
import-in-library = Dans la bibliothèque
import-click-preview = Cliquez sur une photo pour la voir ici
import-this = Importer cette photo
import-this-hint = Espace
import-already = déjà dans votre bibliothèque

## Print (src/print_view.rs)

print-select-first = Sélectionnez d'abord les photos que vous voulez imprimer.
print-page-layout = Mise en page
print-layout-full = Pleine page
print-shrink = Réduire pour ajuster
print-shrink-hint = La photo entière est imprimée, avec du blanc là où les formes diffèrent
print-crop = Rogner pour ajuster
print-crop-hint = La photo remplit le format d'impression ; les bords qui dépassent sont coupés
print-options = Options de bordure et de texte
print-border = Bordure
print-no-border = Pas de bordure
print-thin-line = Fin trait noir
print-white-border = Bordure blanche
print-text-under = Texte sous chaque photo
print-no-text = Pas de texte
print-printer = Imprimante
print-no-printer-found = Aucune imprimante trouvée
print-setup = Réglages de l'imprimante…
print-setup-hint = Format du papier, orientation et réglages propres à l'imprimante
print-paper = Papier
print-default = Par défaut de l'imprimante
print-no-answer = Aucune imprimante n'a répondu — une page A4 est affichée
print-paper-describe = { $dpi } ppp, { $w } x { $h } po ({ $orientation })
print-copies = Copies par photo
print-copies-n =
    { $n ->
        [one] { $n } copie
       *[other] { $n } copies
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } tirage
       *[other] { $prints } tirages
    }
print-cancel = Annuler  { $icon }
print-go = Imprimer  { $icon }
print-review = Vérifier
print-review-hint = Vérifier que chaque photo a assez de pixels pour sa taille d'impression
print-none-left = Il ne reste aucune photo à imprimer.
print-job-printed =
    { $n ->
        [one] { $n } page envoyée à { $printer }.
       *[other] { $n } pages envoyées à { $printer }.
    }
print-preview = Aperçu
print-ready = Toutes vos photos sont prêtes à être imprimées.
print-too-small = Certaines de vos photos sont trop petites pour être bien imprimées.  Vous pouvez retirer ces photos, les imprimer quand même, ou annuler et changer le format d'impression.
print-quality-best = Qualité optimale ({ $ppi } pixels/pouce)
print-quality-good = Bonne qualité ({ $ppi } pixels/pouce)
print-quality-bad = Mauvaise qualité ({ $ppi } pixels/pouce)

## Results of background work (src/backend.rs)

backend-no-data-dir = Aucun dossier pour les données de l'application n'a été trouvé.
backend-cache-fail = Le cache n'a pas pu être vidé ({ $error })
fail-export = Exportation impossible : erreur inattendue.
fail-trash = Impossible de mettre les photos à la corbeille : erreur inattendue.
fail-email = Impossible de préparer l'e-mail : erreur inattendue.
fail-gallery = Impossible de créer la galerie web : erreur inattendue.
fail-movie = Impossible de créer le film : erreur inattendue
fail-collage = Impossible d'enregistrer le collage : erreur inattendue.
fail-download = Le téléchargement s'est arrêté : erreur inattendue.
analysis-no-data-dir = Aucun dossier pour les données de l'application n'a été trouvé.
analysis-stopped = L'analyse des photos a été arrêtée : elle a échoué { $n } fois de suite ({ $error }). Elle sera tentée à nouveau au prochain démarrage d'AlbumCraft.
pack-nothing-downloaded = Rien n'a été téléchargé : { $error }.
pack-downloaded = { $pack } est téléchargé ({ $size }).
pack-download-stopped = Le téléchargement de { $pack } a été arrêté. Le relancer reprend là où il en était.
pack-download-failed = { $pack } n'a pas pu être téléchargé. { $error }
export-nothing = Rien n'a été exporté : { $error }.
export-result =
    { $ok } sur { $total ->
        [one] { $total } photo exportée
       *[other] { $total } photos exportées
    } vers { $path }
export-stopped = L'exportation a été arrêtée avant la fin.
export-failed-list = Non exportées :
export-more = … et { $n } autres
gallery-ready =
    La galerie web est prête :
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } photo n'a pas pu être lue et a été laissée de côté.
       *[other] { $n } photos n'ont pas pu être lues et ont été laissées de côté.
    }
gallery-stopped = La galerie web n'a pas été créée : arrêt demandé.
gallery-failed = Impossible de créer la galerie web : { $error }
movie-ready =
    Le film est prêt :
    { $path }
movie-stopped = Le film n'a pas été créé : arrêt demandé.
movie-failed = Impossible de créer le film : { $error }
collage-saved-many =
    { $n } images enregistrées, la première est :
    { $path }
collage-saved =
    Enregistré :
    { $path }
collage-save-failed = Impossible d'enregistrer le collage : { $error }
import-unexpected = L'importation s'est arrêtée de manière inattendue.
import-summary =
    { $n ->
        [one] { $n } photo importée dans { $path }.
       *[other] { $n } photos importées dans { $path }.
    }
import-already-there =
    { $n ->
        [one] { $n } était déjà là.
       *[other] { $n } étaient déjà là.
    }
import-was-stopped = L'importation a été arrêtée avant la fin.
import-failed-count = { $n } n'ont pas pu être copiées :

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = janv., févr., mars, avr., mai, juin, juil., août, sept., oct., nov., déc.
locale-months-long = janvier, février, mars, avril, mai, juin, juillet, août, septembre, octobre, novembre, décembre
locale-date = { $day } { $month } { $year }
locale-decimal = ,
locale-group = { " " }
locale-unit-kb = Ko
locale-unit-mb = Mo
locale-unit-gb = Go

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Saisissez un nom pour le nouveau dossier.
import-name-chars = Un nom de dossier ne peut pas contenir / \ : * ? " < > |
import-name-long = Ce nom de dossier est trop long.
import-drive = Disque { $letter } :
import-mkdir-fail = Le dossier { $path } n'a pas pu être créé : { $error }
import-too-many = trop de fichiers portent ce nom
import-appeared = un fichier de ce nom est apparu pendant la copie
export-logo-bad = le logo { $path } est illisible ({ $error })
export-unreadable = la photo n'a pas pu être lue
gallery-too-many = trop de galeries portent ce nom ici
gallery-none-read = aucune des photos n'a pu être lue
movie-no-pictures = aucune photo
movie-ffmpeg-start = FFmpeg n'a pas pu être lancé : { $error }
movie-ffmpeg-input = FFmpeg n'a pas d'entrée
movie-ffmpeg-stopped = FFmpeg s'est arrêté ({ $error })
movie-ffmpeg-failed =
    FFmpeg a échoué :
    { $tail }
root-already = Ce dossier est déjà dans la bibliothèque.
root-inside = Ce dossier est déjà dans la bibliothèque, dans { $outer }.
root-excluded = Ce dossier est dans { $folder }, qui est exclu de la bibliothèque.
exclude-is-root = C’est un dossier de la bibliothèque : retirez-le plutôt.
exclude-outside = Ce dossier n’est dans aucun dossier de la bibliothèque.
exclude-already = Ce dossier est déjà exclu.
online-tile-bad = une tuile de carte n'a pas pu être lue : { $error }
online-map-down = Carte en ligne inaccessible ({ $error })
online-search-down = La recherche en ligne est inaccessible ({ $error }).
online-search-odd = Réponse inattendue de la recherche en ligne : { $error }
collage-nothing = il n'y a rien à enregistrer
collage-extension = enregistrez en .pdf, .png ou .jpg
collage-page-fail = une page n'a pas pu être dessinée
geo-near = Près de { $name }
mail-no-pictures = aucune photo à joindre
mail-copies-fail = Impossible de faire les copies pour l'e-mail : { $error }
mail-stopped = L'e-mail n'a pas été préparé : arrêt demandé.
mail-none-read = Impossible de faire les copies pour l'e-mail : aucune des photos n'a pu être lue.
mail-left-out =
    { $n ->
        [one] { $n } photo n'a pas pu être lue et a été laissée de côté.
       *[other] { $n } photos n'ont pas pu être lues et ont été laissées de côté.
    }
mail-opened =
    Un nouveau message avec { $n ->
        [one] { $n } photo
       *[other] { $n } photos
    } est ouvert dans votre logiciel de messagerie.{ $left_out }
mail-asked =
    Votre logiciel de messagerie a reçu la demande d'un nouveau message avec { $n ->
        [one] { $n } photo
       *[other] { $n } photos
    }. Si elles ne sont pas jointes, joignez-les depuis :
    { $path }{ $left_out }
mail-open-fail =
    Impossible d'ouvrir votre logiciel de messagerie ({ $error }).
    Les petites copies de { $n ->
        [one] { $n } photo
       *[other] { $n } photos
    } se trouvent dans :
    { $path }
    Joignez-les à votre message.{ $left_out }
mail-too-many = trop de photos
mail-no-mapi = aucun logiciel de messagerie (mapi32.dll : { $error })
mail-no-attach = le logiciel de messagerie n'accepte pas les pièces jointes (pas de MAPISendMailW)
mail-refused = le logiciel de messagerie a refusé le message (erreur MAPI { $code })
mail-open-mail = impossible d'ouvrir Mail : { $error }
mail-open-mail-short = impossible d'ouvrir Mail
mail-no-xdg = xdg-email (de xdg-utils) n'est pas installé
mail-xdg-fail = impossible de lancer xdg-email : { $error }
gallery-html-lang = fr
gallery-count = { $n ->
    [one] { $n } photo
   *[other] { $n } photos
}
gallery-close = Fermer
gallery-prev = Précédente
gallery-next = Suivante
gallery-zoom = Zoomer
sheet-count = { $n ->
    [one] { $n } photo
   *[other] { $n } photos
}

# Tag proposals: the built-in vocabulary (core vocabulary.rs). Short tag names, capitalised.
tag-beach = Plage
tag-sea = Mer
tag-coast = Côte
tag-island = Île
tag-harbour = Port
tag-lake = Lac
tag-river = Rivière
tag-waterfall = Cascade
tag-mountain = Montagne
tag-hills = Collines
tag-valley = Vallée
tag-forest = Forêt
tag-countryside = Campagne
tag-field = Champs
tag-vineyard = Vignoble
tag-desert = Désert
tag-glacier = Glacier
tag-volcano = Volcan
tag-cave = Grotte
tag-park = Parc
tag-garden = Jardin
tag-city = Ville
tag-skyline = Panorama urbain
tag-street = Rue
tag-old-town = Vieille ville
tag-village = Village
tag-market = Marché
tag-square = Place
tag-home = À la maison
tag-kitchen = Cuisine
tag-restaurant = Restaurant
tag-cafe = Café
tag-bar = Bar
tag-hotel = Hôtel
tag-office = Bureau
tag-school = École
tag-museum = Musée
tag-stadium = Stade
tag-airport = Aéroport
tag-station = Gare
tag-road = Route
tag-campsite = Camping
tag-farm = Ferme
tag-zoo = Zoo
tag-aquarium = Aquarium
tag-underwater = Sous l'eau
tag-sky = Ciel
tag-architecture = Architecture
tag-church = Église
tag-cathedral = Cathédrale
tag-temple = Temple
tag-mosque = Mosquée
tag-castle = Château fort
tag-palace = Palais
tag-ruins = Ruines
tag-monument = Monument
tag-statue = Statue
tag-tower = Tour
tag-lighthouse = Phare
tag-bridge = Pont
tag-house = Maison
tag-interior = Intérieur
tag-stairs = Escaliers
tag-window = Fenêtre
tag-door = Porte
tag-snow = Neige
tag-ice = Glace
tag-rain = Pluie
tag-fog = Brouillard
tag-storm = Orage
tag-rainbow = Arc-en-ciel
tag-clouds = Nuages
tag-sunset = Coucher de soleil
tag-sunrise = Lever de soleil
tag-night = Nuit
tag-northern-lights = Aurore boréale
tag-stars = Ciel étoilé
tag-moon = Lune
tag-autumn = Automne
tag-spring = Printemps
tag-summer = Été
tag-winter = Hiver
tag-flowers = Fleurs
tag-tree = Arbres
tag-leaves = Feuilles
tag-mushroom = Champignons
tag-rocks = Rochers
tag-sand = Sable
tag-portrait = Portrait
tag-selfie = Selfie
tag-group = Photo de groupe
tag-family = Famille
tag-couple = Couple
tag-baby = Bébé
tag-children = Enfants
tag-friends = Amis
tag-crowd = Foule
tag-wedding = Mariage
tag-birthday = Anniversaire
tag-christmas = Noël
tag-easter = Pâques
tag-halloween = Halloween
tag-new-year = Nouvel An
tag-fireworks = Feu d'artifice
tag-party = Fête
tag-concert = Concert
tag-festival = Festival
tag-parade = Défilé
tag-graduation = Remise de diplômes
tag-picnic = Pique-nique
tag-barbecue = Barbecue
tag-dinner = Dîner
tag-ceremony = Cérémonie
tag-performance = Spectacle
tag-dance = Danse
tag-travel = Voyage
tag-hiking = Randonnée
tag-climbing = Escalade
tag-skiing = Ski
tag-snowboard = Snowboard
tag-sailing = Voile
tag-surfing = Surf
tag-swimming = Natation
tag-diving = Plongée
tag-kayak = Kayak
tag-fishing = Pêche
tag-cycling = Vélo
tag-running = Course à pied
tag-football = Football
tag-tennis = Tennis
tag-golf = Golf
tag-basketball = Basket
tag-horse-riding = Équitation
tag-sport = Sport
tag-camping = Camping sauvage
tag-gardening = Jardinage
tag-cooking = Cuisiner
tag-reading = Lecture
tag-music = Musique
tag-games = Jeux
tag-sightseeing = Tourisme
tag-road-trip = Road trip
tag-animals = Animaux
tag-dog = Chien
tag-cat = Chat
tag-bird = Oiseau
tag-horse = Cheval
tag-cow = Vaches
tag-sheep = Moutons
tag-goat = Chèvre
tag-pig = Cochon
tag-chicken = Poules
tag-duck = Canards
tag-fish = Poissons
tag-insect = Insectes
tag-butterfly = Papillon
tag-rabbit = Lapin
tag-deer = Cerf
tag-wildlife = Animaux sauvages
tag-whale = Baleine
tag-seal = Phoque
tag-elephant = Éléphant
tag-food = Nourriture
tag-meal = Repas
tag-breakfast = Petit-déjeuner
tag-cake = Gâteau
tag-dessert = Dessert
tag-bread = Pain
tag-fruit = Fruits
tag-vegetables = Légumes
tag-seafood = Fruits de mer
tag-pizza = Pizza
tag-coffee = Tasse de café
tag-wine = Vin
tag-beer = Bière
tag-drinks = Boissons
tag-car = Voiture
tag-vintage-car = Voiture ancienne
tag-motorbike = Moto
tag-bicycle = Bicyclette
tag-bus = Bus
tag-train = Train
tag-tram = Tramway
tag-plane = Avion
tag-boat = Bateau
tag-ship = Navire
tag-truck = Camion
tag-tractor = Tracteur
tag-toys = Jouets
tag-books = Livres
tag-art = Art
tag-street-art = Street art
tag-sculpture = Sculpture
tag-instrument = Instrument de musique
tag-computer = Ordinateur
tag-phone = Téléphone
tag-clock = Horloge
tag-black-and-white = Noir et blanc
tag-macro = Gros plan
tag-aerial = Vue aérienne
tag-panorama = Panorama
tag-landscape = Paysage
tag-silhouette = Silhouette
tag-reflection = Reflet
tag-long-exposure = Pose longue
tag-golden-hour = Heure dorée
tag-backlight = Contre-jour
tag-minimalist = Minimaliste
tag-abstract = Abstrait
tag-vintage = Rétro
tag-lights = Lumières de la ville
tag-screenshot = Capture d'écran
tag-document = Document
tag-receipt = Ticket de caisse
tag-whiteboard = Tableau blanc
tag-map = Carte
tag-sign = Panneau
tag-ticket = Billet
