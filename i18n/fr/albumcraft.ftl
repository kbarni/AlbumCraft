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
