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
