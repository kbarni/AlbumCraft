# Texto de la interfaz de AlbumCraft, en español.

language-name = Español

## Shared buttons and words

button-ok = Aceptar
button-cancel = Cancelar
button-close = Cerrar
button-remove = Quitar
button-save = Guardar
add-folder = Añadir carpeta…
scan-again = Analizar de nuevo
removed-root = Se ha quitado { $path } de la biblioteca. Las fotos y sus notas no se han modificado.

## Settings dialog

settings-title = Ajustes
settings-saved-note = Los cambios se guardan sobre la marcha.

settings-section-interface = Interfaz
settings-section-folders = Carpetas
settings-section-editing = Edición
settings-section-slideshow = Presentación
settings-section-export = Exportación
settings-section-maps = Mapas
settings-section-storage = Almacenamiento
settings-section-ai = Funciones de IA

## Settings > Interface

settings-language = Idioma
settings-language-system = Igual que el sistema ({ $language })
settings-language-note = Se pueden añadir más idiomas como archivos de traducción; consulte i18n/ en el código fuente.
settings-theme = Tema
theme-system = Igual que el sistema
theme-light = Claro
theme-dark = Oscuro

## Dialogs (src/dialogs.rs)

dialog-new-album = Álbum nuevo
dialog-rename-album = Cambiar el nombre del álbum
dialog-tags = Etiquetas y personas
dialog-gallery = Galería web
dialog-folder-note = Descripción de la carpeta
dialog-movie = Película
dialog-about = Acerca de AlbumCraft
dialog-online-maps = Mapas
dialog-print-review = Revisión
dialog-trash = Mover a la papelera

new-album-name = Nombre del álbum:
new-album-adds = { $n ->
    [one] Se añadirá la foto seleccionada.
   *[other] Se añadirán las { $n } fotos seleccionadas.
}
new-album-create = Crear
rename-album-rename = Cambiar nombre

tags-intro = { $n ->
    [one] Se añadirán a la foto seleccionada. Separe las palabras con comas.
   *[other] Se añadirán a las { $n } fotos seleccionadas. Separe las palabras con comas.
}
tags-tags = Etiquetas:
tags-people = Personas:
tags-add = Añadir

gallery-intro = { $n ->
    [one] Una página web con una foto y sus retoques, que se puede ver en cualquier navegador o subir a un alojamiento web.
   *[other] Una página web con { $n } fotos y sus retoques, que se puede ver en cualquier navegador o subir a un alojamiento web.
}
gallery-title = Título:
gallery-style = Estilo:
gallery-size = Tamaño de las fotos:
gallery-size-medium = Mediano (1280 píxeles, archivos ligeros)
gallery-size-large = Grande (2048 píxeles, para pantallas grandes)
gallery-captions = Mostrar los pies de foto
gallery-make = Elegir la carpeta y crear…

folder-note-intro = Una breve descripción que se muestra bajo el nombre de la carpeta:


# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] Un vídeo de una foto como en la presentación: { $seconds } s
   *[other] Un vídeo de { $n } fotos como en la presentación: { $seconds } s cada una
}{ $fade ->
    [yes] , con fundidos
   *[no] {""}
}{ $motion ->
    [yes] , con paneo y zoom
   *[no] {""}
}.
movie-length = Duración: unos { $seconds } s. Cambie los tiempos y los efectos en la barra de la presentación o en Ajustes.
movie-music = Música
movie-other-music = Otra música…
movie-add-music = Añadir música…
movie-no-ffmpeg = Para crear películas hace falta el programa libre FFmpeg, que no se ha encontrado.
movie-install-ffmpeg = Instálelo desde ffmpeg.org (o con el gestor de paquetes de su sistema) y vuelva a abrir esta ventana.
movie-make = Elegir el archivo y crear…
movie-preview = Vista previa como presentación

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] ¿Mover a la papelera la foto rechazada ({ $size })?
   *[other] ¿Mover a la papelera las { $count } fotos rechazadas ({ $size })?
}
trash-explain = Los archivos se quitan de sus carpetas. Puede recuperarlos de la papelera del sistema mientras no se vacíe.
trash-move = Mover a la papelera

print-review-remove = Quitar estas fotos
print-review-print = Imprimir de todos modos

about-text = Un organizador de fotos para el ordenador: explore sus carpetas de fotos, cree álbumes, elija, rechace y puntúe sus fotos, corríjalas y retóquelas, y lance presentaciones.
about-edits = Los retoques nunca modifican sus archivos originales; se aplican cuando ve o exporta una foto.
about-credits = Mapa integrado: Natural Earth (dominio público). Lista de ciudades: GeoNames (CC BY 4.0). Mapas en línea: © colaboradores de OpenStreetMap (ODbL).

online-maps-question = ¿Mostrar mapas detallados desde Internet?
online-maps-what = AlbumCraft puede mostrar dónde se tomaron sus fotos. Con los mapas en línea, descarga planos de OpenStreetMap y puede buscar direcciones cuando indica un lugar.
online-maps-privacy = Sus fotos y sus ubicaciones nunca se envían. Los servidores de mapas ven su dirección de Internet, las zonas que mira y los lugares que busca.
online-maps-offline = Sin los mapas en línea, AlbumCraft usa su mapamundi y su lista de ciudades integrados, y no se conecta a Internet.
online-maps-later = Puede cambiarlo más tarde en el mapa o en Ajustes (menú principal): Mapas.
online-maps-use = Usar los mapas en línea
online-maps-builtin = Solo el mapa integrado

## Notices at the bottom of the window

notice-undo = Deshacer
notice-show = Mostrar en la carpeta
notice-close = Cerrar este mensaje
notice-no-file-manager = No se pudo abrir el administrador de archivos: { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}

settings-bar = Barra en la parte inferior de la biblioteca
settings-bar-compact = Compacta: una sola línea, visible cuando hay fotos seleccionadas
settings-bar-tray = Bandeja: miniaturas de las fotos seleccionadas y botones grandes
settings-bar-note = En la barra compacta, los botones pierden su nombre cuando la ventana es estrecha. Si los botones no caben, la barra se desplaza hacia un lado.
settings-thumb-size = Tamaño de las fotos en la biblioteca

settings-folders-intro = La biblioteca muestra las fotos de estas carpetas y sus subcarpetas allí donde están: no se copia ni se mueve nada. Para copiar fotos de una tarjeta de memoria o de una cámara, use Importar.
settings-folders-none = Todavía no hay carpetas: se muestran fotos de ejemplo.
settings-folders-remove-hint = Dejar de mostrar esta carpeta (se conservan sus fotos y sus notas)
settings-folders-rescan-hint = Buscar ahora las fotos nuevas, modificadas y eliminadas
settings-excluded-title = Left out folders
settings-excluded-intro = Folders inside a library folder that are not shown, with everything under them (for example Screenshots).
settings-excluded-none = Nothing is left out.
settings-excluded-add = Leave out a folder
settings-excluded-remove-hint = Show this folder again

settings-edit-opens = La pestaña Editar se abre en
settings-edit-basic = Básico: correcciones con un clic y herramientas
settings-edit-advanced = Avanzado: los deslizadores
settings-info-panel = Panel de información
settings-info-show = Mostrar el panel de información en el editor (I)

settings-timing = Tiempos
settings-next-after = Foto siguiente después de
settings-note-paused = La barra espaciadora pausa la presentación; ← y → permiten navegar.
settings-note-manual = Cada foto se queda en pantalla hasta que pase a la siguiente con → o con los botones. El visor a pantalla completa (F11) sigue avanzando solo.
settings-effects = Efectos
settings-fade = Fundido entre las fotos
settings-motion = Paneo y zoom lentos
settings-motion-subtle = Sutil
settings-motion-medium = Medio
settings-captions = Mostrar los pies de foto
settings-viewer-note = El visor a pantalla completa (F11 en el editor) siempre empieza sin fundido ni paneo y zoom.
settings-music = Música
settings-music-play = Reproducir música durante las presentaciones
settings-music-another = Elegir otra…
settings-music-choose = Elegir música…
settings-movies-note = Las películas hechas con fotos también usan estos ajustes.

settings-export-note = Con lo que empieza la vista Exportar.
settings-export-size = Tamaño de las fotos
settings-export-quality = Calidad JPEG
settings-export-info = Información en las copias
settings-export-gps = Incluir el lugar de la toma (GPS)
settings-export-gps-hint = Desactívelo antes de compartir fotos si no quiere que otros vean el lugar.
settings-export-exif = Incluir los datos de la cámara (EXIF)
settings-export-exif-hint = Cámara, objetivo, exposición y fecha de la toma, copiados del archivo original. El lugar sigue el ajuste anterior.
settings-creator = Autor
settings-creator-hint = Su nombre
settings-copyright = Derechos de autor
settings-copyright-hint = © Su nombre
settings-credit-note = Se escriben en cada copia (EXIF), en lugar de los del original. Las copias enviadas por correo también siguen estos ajustes. La firma o el logotipo se elige en la vista Exportar.

settings-maps-online = Mapas en línea
settings-maps-use = Usar los mapas en línea (OpenStreetMap)
settings-maps-note = Planos detallados y búsqueda de direcciones. Sus fotos y sus ubicaciones nunca se envían; los servidores de mapas ven su dirección de Internet, las zonas que mira y los lugares que busca. Desactivado: solo el mapamundi y la lista de ciudades integrados, y ninguna conexión a Internet.
settings-maps-tiles = Teselas de mapa descargadas
settings-maps-tiles-after = Las teselas de mapa se descargan de nuevo cuando hace falta.

settings-notes = Sus notas
settings-notes-note = Las notas, pies de foto, etiquetas, retoques y álbumes se guardan en un solo archivo; sus archivos de fotos nunca se modifican.
settings-thumbs = Miniaturas
settings-thumbs-after = Las miniaturas se rehacen a partir de las fotos cuando hace falta (más lento durante un rato).
cache-on-disk = { $size } en el disco
cache-measuring = Midiendo…
cache-empty = Vaciar

settings-ai-soon = Todavía no disponible. Previsto:
settings-ai-faces = Caras
settings-ai-faces-text = detectar las caras y agruparlas por persona; usted pone nombre a los grupos. Una descarga de unos 10 a 40 MB.
settings-ai-search = Búsqueda inteligente
settings-ai-search-text = encontrar fotos describiéndolas («nieve», «tarta de cumpleaños»). Ligera (inglés, unos 100 MB) o multilingüe (unos 400 MB).
settings-ai-tags = Sugerencias de etiquetas
settings-ai-tags-text = etiquetas propuestas según el contenido de la foto.
settings-ai-local = Todo se calculará en este ordenador; no se enviará nada a ninguna parte.

export-size-original = Tamaño original
export-size-very-large = Muy grande (2560 píxeles)
export-size-large = Grande (1600 píxeles)
export-size-small = Pequeño, para correo (800 píxeles)

## Export view (src/export_view.rs)

filter-pictures = Fotos
view-back-editor = Editor
view-back-library = Biblioteca
view-back-hint = Volver (Esc)

export-title = Exportar
export-count-all = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
export-count-some = { $n } de { $total } fotos
export-select-first = Seleccione primero algunas fotos.
export-logo-unreadable = No se puede leer el logotipo { $path }.
export-task = { $n ->
    [one] Exportando { $n } foto
   *[other] Exportando { $n } fotos
}
export-save-into = Guardar en
export-choose-folder = Elegir carpeta…
export-choose-folder-hint = Elegir la carpeta de las copias
export-safe = No se sobrescribe nada; los originales no se modifican.
export-go = { $n ->
    [one] Exportar { $n } foto
   *[other] Exportar { $n } fotos
}
export-dialog-title = Exportar las fotos a…

export-size = Tamaño
export-size-other = Otro:
export-size-pixels = { " " }píxeles
export-size-note = El lado más largo. Las fotos nunca se amplían.
export-quality = Calidad JPEG

export-signature = Firma
export-signature-on = Añadir una firma o un logotipo
export-sig-text = Texto
export-sig-logo = Imagen (logotipo)
export-logo-other = Otra imagen…
export-logo-choose = Elegir una imagen…
export-logo-dialog = Imagen del logotipo o de la firma
export-logo-note = Un PNG con fondo transparente da el mejor resultado.
export-sig-text-hint = Su nombre, o © Su nombre
export-sig-position = Posición
export-sig-size = Tamaño
export-sig-size-hint = Relativo al lado más corto de la foto
export-sig-opacity = Opacidad
export-sig-colour = Color
export-sig-shadow = Sombra
export-sig-shadow-hint = Un ligero halo para que el texto se lea sobre cualquier foto

export-info = Información en los archivos
export-gps = Lugar de la toma (GPS)
export-gps-hint = Desactívelo antes de compartir fotos si no quiere que otros vean el lugar.
export-exif = Datos de la cámara (EXIF)
export-exif-hint = Cámara, objetivo, exposición y fecha de la toma, copiados del archivo original.
export-creator = Autor
export-creator-hint = Su nombre
export-copyright = Derechos de autor
export-copyright-hint = © Su nombre
export-credit-note = Se escriben en cada copia. La firma, el autor y los derechos de autor se recuerdan para la próxima vez (también en Ajustes, Exportación).

export-left-out = Excluida: haga clic para exportarla también
export-leave-out = Haga clic para excluir esta foto

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = Todas las fotos
nav-picks = Elegidas
nav-places = Lugares
nav-places-hint = Sus fotos en un mapa
nav-albums = Álbumes
nav-people = Personas
nav-folders = Carpetas
people-hint = Indique las personas desde la barra de selección.
scanning = Buscando fotos…

bar-import = Importar
bar-import-hint = Copiar fotos de una tarjeta de memoria, una cámara o una carpeta a su biblioteca
bar-folders = Carpetas
bar-folders-hint = Las carpetas que se muestran en la biblioteca, donde están (no se copia nada)
bar-size-hint = Tamaño de las fotos
bar-picks-hint = Mostrar solo las fotos elegidas
bar-hide-rejects = Ocultar las fotos rechazadas

sort-name = Nombre de archivo
sort-taken = Fecha de la toma
sort-rating = Puntuación
sort-hint = Orden de las fotos: { $order }

filter-any = Todas
filter-any-no-rejects = Todas, sin las rechazadas
filter-min = { $n }+
filter-min-no-rejects = { $n }+, sin las rechazadas
filter-hint = Filtrar por puntuación; ocultar las fotos rechazadas
filter-any-rating = Todas las puntuaciones
filter-or-more = { $rating } o más

album-rename = Cambiar nombre…
album-delete = Eliminar el álbum
album-show-map = Mostrar en el mapa
album-slideshow = Reproducir como presentación
album-deleted = Se ha eliminado el álbum «{ $name }». Sus fotos no se han modificado.

empty-scanning = Buscando fotos…
empty-no-match = Ninguna foto coincide con su búsqueda o su filtro.
empty-album = Este álbum está vacío. Seleccione fotos y añádalas desde la barra de selección.
empty-person = Todavía no hay nadie indicado.
empty-picks = Todavía no hay fotos elegidas. Pase el ratón sobre una foto y haga clic en su bandera, o pulse P.
empty-unreachable = No se encontró ninguna foto en las carpetas de su biblioteca. Si están en un disco que no está conectado, conéctelo y analice de nuevo.
empty-library = Su biblioteca está vacía. Importe fotos o añada una carpeta que ya tenga.
empty-folders-hint = Las carpetas que se muestran en la biblioteca
empty-import = Importar fotos
empty-import-hint = Copiar fotos de una tarjeta de memoria, una cámara o una carpeta
empty-add = Añadir una carpeta
empty-add-hint = Mostrar las fotos de una carpeta donde están, sin copiarlas

folder-play-hint = Reproducir esta carpeta como presentación
folder-edit-hint = Cambiar la descripción
folder-map-hint = Mostrar esta carpeta en el mapa
tile-pick-hint = Elegir (P)
tile-reject-hint = Rechazar (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = Sin puntuación
rating-stars = { $n ->
    [one] { $n } estrella
   *[other] { $n } estrellas
}
flag-pick = Elegir
flag-unpick = Dejar de elegir
flag-reject = Rechazar
flag-unreject = Dejar de rechazar
flag-rating = Puntuación
album-new = Álbum nuevo…
menu-copy-edits = Copiar los retoques
menu-paste-edits = Pegar los retoques
menu-paste-n = { $n ->
    [one] Pegar { $n } retoque
   *[other] Pegar { $n } retoques
}
copy-edits-select-one = Seleccione una sola foto para copiar sus retoques.
menu-import = Importar fotos…
menu-add-folder = Añadir una carpeta a la biblioteca…
menu-folders = Carpetas de la biblioteca…
menu-scan = Analizar las carpetas de nuevo
menu-select-all = Seleccionar todo
menu-select-none = No seleccionar nada
menu-rotate-left = Girar a la izquierda
menu-rotate-right = Girar a la derecha
menu-trash-rejects = Mover las fotos rechazadas a la papelera…
menu-trash-rejects-hint = Quitar de sus carpetas los archivos de las fotos rechazadas, tras confirmar
menu-contact-sheet = Hoja de contactos…
menu-contact-sheet-hint = Páginas de fotos pequeñas con un título y sus nombres de archivo, para imprimir o guardar como PDF
menu-undo-edits = Deshacer todos los retoques
menu-settings = Ajustes…
menu-about = Acerca de AlbumCraft…
menu-exit = Salir
edits-removed = { $n ->
    [one] Se han quitado los retoques de { $n } foto.
   *[other] Se han quitado los retoques de { $n } fotos.
}

status-library = Biblioteca: { $folders ->
    [one] { $folders } carpeta
   *[other] { $folders } carpetas
}, { $pictures ->
    [one] { $pictures } foto
   *[other] { $pictures } fotos
}, { $size } en el disco
status-importing = Importando { $done } de { $total }…
status-stop = Detener
status-stop-hint = Detener tras la foto en curso (las ya copiadas se quedan)
task-running = { $label }…
task-stopping = { $label }: deteniendo…
task-cancel-hint = Detener tras la foto en curso
status-samples = Se muestran fotos de ejemplo. Use Importar o Carpetas para añadir las suyas.

selected-count = { $n ->
    [one] { $n } foto
   *[other] { $count } fotos
}
selected-all = { $n ->
    [one] { $pictures } seleccionada
   *[other] { $pictures } seleccionadas
}
selected-set = { $name } · { $pictures }
fallback-folder = Carpeta
fallback-album = Álbum
tray-nothing = Nada seleccionado
tray-click = Haga clic en las fotos para seleccionarlas

output-web = Página web
output-web-tip = Crear una galería web con estas fotos
output-mail = Correo
output-mail-tip = Pequeñas copias, adjuntas a un mensaje nuevo en su programa de correo
output-print = Imprimir
output-print-tip = Imprimir estas fotos (Ctrl+P)
output-export = Exportar
output-export-tip = Guardar copias retocadas en una carpeta
output-collage = Collage
output-collage-tip = Crear un collage o una hoja de contactos
output-movie = Película
output-movie-tip = Crear una película

bar-clear-hint = Quitar la selección (Esc)
bar-pick-hint = Elegir / dejar de elegir (P)
bar-reject-hint = Rechazar / dejar de rechazar (X)
bar-rotate-left = Girar a la izquierda (Ctrl+[)
bar-rotate-right = Girar a la derecha (Ctrl+])
bar-tags = Etiquetas y personas
bar-location = Definir el lugar: dónde se tomaron estas fotos, en un mapa
bar-to-album = A un álbum
bar-clear = Quitar

ctx-open = Abrir en el editor
ctx-slideshow = Reproducir la presentación desde aquí
ctx-add-to-album = Añadir a un álbum
ctx-remove-from-album = Quitar del álbum
ctx-tags = Etiquetas y personas…
ctx-location = Definir el lugar…
ctx-export = Exportar…
ctx-show-file = Mostrar en el administrador de archivos

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Vívido
look-vivid-tip = Colores ricos y saturados de película de diapositivas, con negros profundos
look-soft-film = Película suave
look-soft-film-tip = Película de retrato delicada: contraste suave, cálido y favorecedor para la piel
look-polaroid = Instantánea
look-polaroid-tip = Copia de cámara instantánea desvaída: negros elevados, luces cálidas, sombras frías
look-cross-process = Proceso cruzado
look-cross-process-tip = Luces amarillas, sombras verdeazuladas y contraste marcado
look-lomo = Lomo
look-lomo-tip = Aspecto saturado de cámara de juguete, con esquinas oscuras
look-matte = Mate
look-matte-tip = Negros planos y lavados, colores apagados
look-teal-orange = Verdeazulado y naranja
look-teal-orange-tip = Etalonaje de cine: sombras verdeazuladas, piel y luces cálidas
look-high-key = High key
look-high-key-tip = Luminoso y aireado, con sombras abiertas
look-cool = Frío
look-cool-tip = Una dominante más fría, más azul
look-warm = Cálido
look-warm-tip = Una dominante más cálida, dorada
look-bw = B/N
look-bw-tip = Blanco y negro clásico
look-dramatic-bw = B/N dramático
look-dramatic-bw-tip = Blanco y negro con filtro rojo: cielos oscuros, fuerte contraste local
look-sepia = Sepia
look-sepia-tip = Tono marrón de foto antigua
look-soft-focus = Enfoque suave
look-soft-focus-tip = Centro nítido, bordes suaves y soñadores
look-glow = Resplandor
look-glow-tip = Un suave halo luminoso
look-film-grain = Grano de película
look-film-grain-tip = Grano al estilo de la película
look-focal-bw = B/N focal
look-focal-bw-tip = Color en el centro, blanco y negro alrededor
effect-sharpen = Nitidez
effect-sepia = Sepia
effect-bw = B/N
effect-warmify = Calentar
effect-film-grain = Grano de película
effect-tint = Tinte
effect-saturation = Saturación
effect-soft-focus = Enfoque suave
effect-glow = Resplandor
effect-filtered-bw = B/N filtrado
effect-focal-bw = B/N focal
effect-graduated-tint = Tinte degradado
op-rotate-right = Girar a la derecha
op-rotate-left = Girar a la izquierda
op-crop = Recortar
op-straighten = Enderezar { $angle }°
op-perspective-v = Perspectiva, vertical { $v }
op-perspective-h = Perspectiva, horizontal { $h }
op-perspective-both = Perspectiva { $v } / { $h }
op-redeye = Ojos rojos
op-lucky = Corrección automática (suerte)
op-auto-contrast = Contraste automático
op-auto-color = Color automático
op-tuning = Ajuste fino
op-look = { $look } { $amount } %
op-adjust = Ajuste
op-adjust-parts = Ajuste: { $parts }
adj-wb = balance de blancos
adj-exposure = exposición
adj-highlights = luces
adj-shadows = sombras
adj-blacks = negros
adj-contrast = contraste
adj-saturation = saturación
adj-vibrance = intensidad
adj-clarity = claridad
adj-sharpen = nitidez
adj-noise = reducción de ruido
decor-vignette-light = Viñeteado claro
decor-vignette = Viñeteado
decor-instant = Marco de instantánea
decor-border = Borde
decor-shadow = Sombra paralela
decor-rounded = Esquinas redondeadas
decor-text = Texto «{ $text }»
wb-as-shot = Como se tomó
wb-daylight = Luz del día
wb-cloudy = Nublado
wb-shade = Sombra
wb-tungsten = Tungsteno
wb-fluorescent = Fluorescente
wb-flash = Flash
wb-custom = Personalizado
wb-auto = Automático
gallery-style-light = Cuadrícula clara
gallery-style-dark = Cuadrícula oscura
gallery-style-masonry = Mosaico (fotos enteras)
crop-free = Libre
crop-original = Forma original
crop-square = Cuadrado
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (copia de 10 × 15)
crop-16-9 = 16 × 9 (pantalla panorámica)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Doble clic para restablecer
edit-loading = La foto todavía se está cargando.
edit-straightened = Enderezada { $angle }°.
edit-already-level = Esta foto ya parece estar derecha.
edit-no-horizon = No se encontró ningún horizonte ni línea vertical nítida para enderezar.
edit-histogram = Histograma y datos de la cámara
edit-unknown-camera = Cámara desconocida
edit-35mm = (equivalente a 35 mm: { $mm } mm)
edit-focal = Distancia focal: { $mm } mm
edit-iso = ISO: { $iso }
edit-no-camera = Esta foto no tiene datos de la cámara
edit-clip-shadows = sombras { $pct } %
edit-clip-highlights = luces { $pct } %
edit-clipped = Recorte: { $parts }
edit-clipped-hint = Parte de la foto que es negro puro o blanco puro en al menos un color
edit-back = Volver a la biblioteca
edit-info = Info
edit-info-hint = Propiedades, lugar y etiquetas (I)
edit-side-by-side-hint = Mostrar las fotos una al lado de la otra
edit-stacked-hint = Mostrar una foto encima de la otra
edit-focus-hint = Cambiar la foto que se retoca (la que recibe sus cambios)
edit-compare = Comparar
edit-compare-hint = Esta foto junto a otra, una al lado de la otra
edit-original = Original
edit-original-hint = Mantener pulsado para ver la foto sin sus retoques (o mantener O)
edit-slideshow = Presentación
edit-fullscreen-hint = Pantalla completa (F11): desde esta foto, una tras otra, con música
edit-next = Foto siguiente
edit-previous = Foto anterior
edit-position = { $n } de { $total }
edit-tab-edit = Editar
edit-tab-presets = Ajustes preestablecidos
edit-tab-effects = Efectos
edit-undo = Deshacer
edit-undo-hint = Deshacer el último cambio (Ctrl+Z)
edit-redo = Rehacer
edit-list = Retoques de esta foto:
edit-list-none = (ninguno: es el original)
edit-step-hint =
    { $label }
    Haga clic para volver a este paso (se quitan los siguientes; Deshacer los recupera)
edit-copy-hint = Copiar estos retoques para pegarlos en otras fotos (Ctrl+Mayús+C)
edit-paste-n = Pegar { $n }
edit-paste-hint = Añadir los retoques copiados a esta foto (Ctrl+Mayús+V)
edit-basic = Básico
edit-advanced = Avanzado
edit-wb-help = Haga clic en un elemento de la foto que debería ser gris neutro o blanco.
edit-crop-help = Arrastre los bordes o las esquinas del marco, arrastre dentro para moverlo, o dibuje un marco nuevo. Intro lo aplica.
edit-crop-shape = Forma
edit-crop-orientation = Orientación
edit-landscape = Horizontal
edit-landscape-hint = Más ancha que alta
edit-portrait = Vertical
edit-portrait-hint = Más alta que ancha
edit-apply = Aplicar
edit-remove-crop = Quitar el recorte
edit-remove-crop-hint = Volver a mostrar toda la foto
edit-redeye-help = Dibuje un marco alrededor de los ojos rojos. Solo se oscurecen las pupilas rojas dentro del marco.
edit-straighten-title = Enderezar
edit-straighten-help = Mueva el deslizador hasta que el horizonte quede alineado con la cuadrícula.
edit-oneclick = Correcciones con un clic
edit-lucky = Me siento afortunado
edit-lucky-hint = Exposición, balance de blancos, punto negro y un poco de intensidad, todo a la vez (luego ajústelos en Avanzado)
edit-auto-exposure = Exposición automática
edit-auto-exposure-hint = Aclarar u oscurecer para lograr una exposición equilibrada
edit-auto-colour = Color automático
edit-auto-colour-hint = Balance de blancos automático: quitar una dominante de color
edit-auto-contrast = Contraste automático
edit-auto-contrast-hint = Estirar los tonos más oscuros y más claros
edit-auto-straighten = Enderezado automático
edit-auto-straighten-hint = Enderezar un horizonte o unas verticales inclinados
edit-tools = Herramientas
edit-pick-neutral = Punto neutro
edit-pick-neutral-hint = Haga clic en un elemento gris para ajustar el balance de blancos
edit-sec-wb = BALANCE DE BLANCOS
edit-sec-light = LUZ
edit-sec-colour = COLOR
edit-sec-detail = DETALLE
edit-sec-geometry = GEOMETRÍA
edit-wb-pick = Elegir
edit-wb-pick-hint = Haga clic en un elemento gris de la foto
edit-s-temperature = Temperatura
edit-s-temperature-tip = Más azul (a la izquierda) o más cálido (a la derecha)
edit-s-tint = Tinte
edit-s-tint-tip = Más verde (a la izquierda) o más magenta (a la derecha)
edit-s-exposure = Exposición
edit-s-exposure-tip = Luminosidad global, en pasos de diafragma
edit-s-highlights = Luces
edit-s-highlights-tip = Zonas claras: a la izquierda recupera detalle, a la derecha aclara
edit-s-shadows = Sombras
edit-s-shadows-tip = Zonas oscuras: a la derecha las abre
edit-s-blacks = Negros
edit-s-blacks-tip = Punto negro: a la izquierda profundiza, a la derecha eleva
edit-s-contrast = Contraste
edit-s-contrast-tip = Diferencia entre lo oscuro y lo claro
edit-s-saturation = Saturación
edit-s-saturation-tip = Todos los colores
edit-s-vibrance = Intensidad
edit-s-vibrance-tip = Sobre todo los colores apagados; respeta la piel y los colores ya vivos
edit-s-clarity = Claridad
edit-s-clarity-tip = Contraste local en los tonos medios
edit-s-sharpen = Nitidez
edit-s-sharpen-tip = Detalles finos más nítidos
edit-s-noise = Ruido
edit-s-noise-tip = Reducción de ruido: suaviza el grano y conserva los contornos
edit-reset-all = Restablecer todo
edit-crop-btn = Recortar…
edit-crop-btn-hint = Elegir la parte de la foto que se conserva, o cambiar el recorte
edit-s-straighten-tip = Girar un poco la foto para enderezar el horizonte (amplía para ocultar las esquinas)
edit-s-vertical = Vertical
edit-s-vertical-tip = Perspectiva: a la derecha ensancha la parte superior, para corregir los edificios que se inclinan hacia atrás al fotografiar desde abajo
edit-s-horizontal = Horizontal
edit-s-horizontal-tip = Perspectiva: a la derecha agranda el lado derecho, para una pared vista en diagonal
edit-intensity = Intensidad
edit-remove-preset = Quitar el ajuste preestablecido
edit-presets-help = Aspectos para toda la foto. Haga clic en uno y luego ajuste su intensidad.
edit-effects-add = Añadir sobre la foto o a su alrededor
edit-add-instant = Marco de instantánea
edit-add-text = Texto
edit-your-text = Su texto
edit-effects-note = Los efectos se aplican encima de todos los demás retoques. Añada uno para ajustarlo aquí.
edit-p-amount = Intensidad
edit-p-size = Tamaño
edit-p-width = Anchura
edit-p-bottom = Inferior
edit-p-colour = Color
edit-p-darkness = Oscuridad
edit-p-background = Fondo
edit-p-radius = Radio
edit-vignette-note = A la izquierda del centro, en cambio, se aclaran las esquinas.
edit-halo = Halo ligero (legible sobre cualquier foto)
edit-remove-effect = Quitar este efecto
edit-caption-hint = ¡Añada un pie de foto!
edit-print-hint = Imprimir esta foto (Ctrl+P)
edit-album = Álbum
edit-zoom-hint = Zoom (100 % = los píxeles reales de la foto)
edit-zoom-slider-hint = Zoom: a la izquierda = ajustada, en el medio = 1:1, a la derecha = 400 % (la rueda también funciona; arrastre la foto para ampliar al 100 % y moverla)
edit-one-to-one-hint = Tamaño original (100 %): un píxel de la foto por píxel de pantalla
edit-fit = Ajustar
edit-fit-hint = Ajustar toda la foto a la ventana (un doble clic en una foto ampliada hace lo mismo)
edit-people = Personas:
edit-person-remove = Quitar la persona
edit-person-add = añadir una persona
edit-compare-none = No hay otra foto en esta carpeta
edit-compare-selected = Seleccionada
edit-compare-swap-hint = Haga clic para retocar esta foto en su lugar
edit-view-loading = Cargando…
edit-sharpening = Aplicando nitidez…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = No se puede leer esta foto.
edit-full-unreadable = No se puede leer la foto completa.
edit-full-loading = Cargando la foto completa…
edit-draw-box = Arrastre sobre la foto para dibujar un marco
edit-click-neutral = Haga clic en un elemento que debería ser gris o blanco
edit-wb-title = Balance de blancos

## Info panel, search, slideshow bar

info-hide-hint = Ocultar este panel (I)
info-properties = Propiedades
info-camera = Cámara
info-lens = Objetivo
info-taken = Tomada el
info-file-date = { $date } (fecha del archivo)
info-size = Tamaño
info-megapixels = { $w } × { $h } ({ $mp } Mpx)
info-file = Archivo
info-focal = Distancia focal
info-focal-35 = { $f } mm (35 mm: { $e } mm)
info-aperture = Apertura
info-exposure = Exposición
info-folder = Carpeta
info-sample = Foto de ejemplo integrada
info-edits = Retoques
info-no-exif = Esta foto no tiene datos de la cámara (EXIF).
info-exif = EXIF detallado
info-flash = Flash
info-bias = Compensación de exposición
info-program = Programa
info-metering = Medición
info-distance = Distancia
info-colour-space = Espacio de color
info-gps = Posición GPS
info-digitized = Digitalizada
info-modified = Modificada
info-software = Software
info-preview = Vista previa de la cámara
info-preview-hint = { $w } × { $h } píxeles, almacenada en el archivo
info-all-exif = Todos los datos EXIF ({ $n } entradas)
info-location = Lugar
info-set-in-app = Definido en AlbumCraft
info-from-gps = Del GPS de la cámara
info-modify = Modificar…
info-modify-hint = Mover el marcador, cambiar el nombre del lugar o quitarlo
info-show-map-hint = Abrir Lugares en esta foto
info-no-location = Todavía no hay lugar.
info-set-location-hint = Buscar el lugar o hacer clic en él en un mapa
info-tags = Etiquetas
info-no-tags = Todavía no hay etiquetas.
info-remove-tag = Quitar esta etiqueta
info-add-tag = Añadir una etiqueta…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = E
coord-w = O
info-orient-2 = En espejo
info-orient-3 = Boca abajo
info-orient-4 = En espejo, boca abajo
info-orient-5 = En espejo, girada a la izquierda
info-orient-6 = Girada a la derecha (cámara en posición vertical)
info-orient-7 = En espejo, girada a la derecha
info-orient-8 = Girada a la izquierda (cámara en posición vertical)
search-remove-hint = Quitar de la búsqueda
search-hint = Buscar etiquetas, lugares, pies de foto…
search-and = y…
search-clear-hint = Borrar la búsqueda
search-for = Buscar «{ $text }»
search-kind-tags = Etiquetas
music-dialog-title = Música para la presentación
filter-music = Música
ss-previous = Anterior (←)
ss-play-pause = Reproducir / pausa (Espacio)
ss-next = Siguiente (→)
ss-seconds = Segundos por foto
ss-interval-off = Desactivado: las fotos solo cambian con → o con los botones (Ajustes, Presentación)
ss-fade = Fundido
ss-motion = Movimiento
ss-other-music-hint = Elegir otro archivo de música
ss-exit-full = Salir de la pantalla completa (F11 o Esc)
ss-exit = Salir (Esc)

## Messages of the window (src/app.rs)

save-failed-prefix = No se pudo guardar su biblioteca:
save-failed =
    { $prefix } { $error }
    Sus últimos cambios solo están por ahora en esta ventana; AlbumCraft sigue intentándolo.
save-ok-again = Su biblioteca se vuelve a guardar.
lib-unreadable-kept =
    No se pudo leer el archivo de su biblioteca ({ $error }).

    Se ha conservado en:
    { $backup }

    AlbumCraft ha arrancado con una biblioteca vacía. Sus fotos no se han tocado.
lib-unreadable-nosave =
    No se pudo leer el archivo de su biblioteca ({ $error }).

    AlbumCraft ha arrancado con una biblioteca vacía y esta vez no guardará ningún cambio, para no sobrescribir el archivo.
dialog-library = Biblioteca
crash-title = AlbumCraft se cerró inesperadamente
crash-text =
    AlbumCraft se detuvo por un error la última vez que se ejecutó. Pueden faltar los cambios hechos en los últimos segundos.

    Los detalles se han escrito en:
    { $log }

    Adjunte este archivo si informa del problema.
picasa-imported =
    { $n ->
        [one] Notas, pies de foto y retoques importados del archivo .picasa.ini de { $n } carpeta.
       *[other] Notas, pies de foto y retoques importados de los archivos .picasa.ini de { $n } carpetas.
    }
copy-no-edits = Esta foto no tiene retoques que copiar.
copied-edits =
    { $n ->
        [one] { $n } retoque copiado. Seleccione fotos y péguelo (Ctrl+Mayús+V).
       *[other] { $n } retoques copiados. Seleccione fotos y péguelos (Ctrl+Mayús+V).
    }
paste-first = Primero copie los retoques de una foto (Ctrl+Mayús+C).
pasted-edits =
    { $n ->
        [one] Retoques pegados en { $n } foto.
       *[other] Retoques pegados en { $n } fotos.
    }
no-pictures-to-show = No hay ninguna foto que mostrar.
music-error = Sin música: { $error }
choose-folder-dialog = Elija una carpeta de fotos
added-to-album =
    { $n ->
        [one] { $n } foto añadida a «{ $name }».
       *[other] { $n } fotos añadidas a «{ $name }».
    }
trash-samples = Las fotos de ejemplo no se pueden mover a la papelera.
trash-no-rejects = No hay ninguna foto rechazada. Pulse X para rechazar una.
task-trash =
    { $n ->
        [one] Moviendo { $n } foto a la papelera
       *[other] Moviendo { $n } fotos a la papelera
    }
task-email =
    { $n ->
        [one] Preparando { $n } foto para un correo
       *[other] Preparando { $n } fotos para un correo
    }
default-title = Mis fotos
gallery-folder-dialog = Crear la galería web en…
task-gallery =
    { $n ->
        [one] Creando la galería web de { $n } foto
       *[other] Creando la galería web de { $n } fotos
    }
movie-save-dialog = Guardar la película como…
filter-mp4 = Vídeo MP4
task-movie = Creando la película
scan-truncated = Solo se muestran las primeras { $n } fotos: quite carpetas de la biblioteca para ver las demás.
selection-none = Nada seleccionado
selection-one = { $name }     { $date }     { $w }x{ $h } píxeles     { $size }
selection-several = { $pictures }     { $a }      { $size } en el disco
selection-range = { $pictures }     del { $a } al { $b }     { $size } en el disco

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Montón de fotos
cstyle-pile-tip = Fotos esparcidas, como copias tiradas sobre una mesa
cstyle-mosaic = Mosaico
cstyle-mosaic-tip = Fotos de distintos tamaños llenan la página
cstyle-frame = Foto central
cstyle-frame-tip = Un mosaico alrededor de una foto grande en el centro
cstyle-grid = Cuadrícula
cstyle-grid-tip = Filas y columnas regulares
cstyle-film = Tira de película
cstyle-film-tip = Fotos sobre tiras de película
cstyle-clothesline = Tendedero
cstyle-clothesline-tip = Fotos instantáneas sujetas con pinzas a cuerdas
cstyle-fan = Abanico
cstyle-fan-tip = Extendidas como una baraja
cstyle-shape = Forma
cstyle-shape-tip = Un corazón, un círculo o una estrella hechos de fotos
cstyle-blend = Exposición múltiple
cstyle-blend-tip = Las fotos se ven unas a través de otras (ideal con 2 a 4)
cstyle-sheet = Hoja de contactos
cstyle-sheet-tip = Páginas de fotos pequeñas con un título y sus nombres de archivo
shape-heart = Corazón
shape-circle = Círculo
shape-star = Estrella
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = Carta (EE. UU.)
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Cuadrado
fmt-wide = Pantalla (16:9)
grad-vertical = De arriba abajo
grad-diagonal = Diagonal
grad-radial = Desde el centro
collage-need-sheet = Seleccione primero las fotos de la hoja de contactos.
collage-need-two = Seleccione al menos dos fotos para crear un collage.
collage-unreadable = No se pudo leer esta foto.
collage-new-layout = Nueva disposición
collage-new-layout-hint = Otras posiciones, giros y tamaños
collage-shuffle = Mezclar
collage-shuffle-hint = Poner las fotos en orden aleatorio
collage-spacing = Espacio entre las fotos
border-none = Ninguno
border-white = Blanco
border-instant = Instantánea
collage-frame-text = Texto en los marcos
cap-none = Ninguno
cap-caption = Pie de foto
collage-shadows = Sombras
bg-gradient = Degradado
bg-picture = Imagen
collage-to = a
collage-use-selected = Usar la foto seleccionada
collage-use-selected-hint = Haga clic primero en una foto del collage
collage-choose-file = Elegir un archivo…
collage-no-bg = Todavía no se ha elegido ninguna imagen
collage-bg-dialog = Imagen de fondo
collage-page = Página
collage-size = Tamaño de la imagen
collage-px = { $px } píxeles
collage-centre = Poner en el centro
collage-put-back = Restaurar
collage-put-back-hint = Deshacer los movimientos de esta foto
collage-leave-out = Excluir
collage-leave-out-hint = Quitar esta foto del collage (Supr)
collage-title = Título
collage-no-title = Sin título
collage-no-subtitle = Sin segunda línea
collage-header = Título en la primera página
collage-under = Bajo cada foto
collage-date = Fecha
collage-pictures = Fotos
collage-page-numbers = Números de página
collage-sending = Enviando la página { $page } de { $total } a la impresora…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } página
       *[other] { $pages } páginas
    }
collage-no-printer = No hay ninguna impresora disponible
collage-save-pdf = Guardar como PDF…
collage-save-picture = Guardar la imagen…
collage-save-picture-hint = Guardar el collage como imagen JPEG o PNG
collage-file-sheet = Hoja de contactos
collage-file-collage = Collage
collage-save-pdf-title = Guardar como PDF
collage-save-picture-title = Guardar el collage como imagen
filter-pdf = Documento PDF
filter-jpeg = Imagen JPEG
filter-png = Imagen PNG
collage-saving-sheet = Guardando la hoja de contactos…
collage-saving = Guardando el collage…
task-save-collage = Guardando el collage
collage-printed =
    { $n ->
        [one] { $n } página enviada a { $printer }.
       *[other] { $n } páginas enviadas a { $printer }.
    }
collage-print-failed = La impresión no terminó: { $error }
collage-hint-swap = Arrastre una foto sobre otra para intercambiarlas. Haga clic en una foto para seleccionarla (Supr la excluye).
collage-hint-free = Arrastre una foto para moverla, desplácese sobre ella para girarla. Haga clic en una foto para seleccionarla (Supr la excluye).
collage-page-of = Página { $page } de { $total }

## Places map (src/map_view.rs)

map-search-still = Aún se están buscando las posiciones GPS de sus fotos: inténtelo de nuevo en un momento.
map-none-located =
    { $n ->
        [one] Esta foto todavía no tiene lugar. Selecciónela y haga clic en { $icon } (Definir el lugar) en la barra de selección para situarla.
       *[other] Ninguna de estas { $n } fotos tiene todavía lugar. Selecciónelas y haga clic en { $icon } (Definir el lugar) en la barra de selección para situarlas.
    }
map-location-set =
    { $n ->
        [one] Lugar definido para { $n } foto.
       *[other] Lugar definido para { $n } fotos.
    }
map-location-removed =
    { $n ->
        [one] Lugar quitado de { $n } foto.
       *[other] Lugar quitado de { $n } fotos.
    }
map-nothing-online = No se encontró nada en línea.
map-set-location = Definir el lugar
map-sub-scope =
    { $n } de { $total ->
        [one] { $total } foto
       *[other] { $total } fotos
    } con lugar
map-sub-indexing = { $n } con lugar · buscando posiciones GPS…
map-sub-all =
    { $n ->
        [one] { $n } foto con lugar
       *[other] { $n } fotos con lugar
    }
map-title-scope = Lugares · { $title }
map-show-whole = Mostrar toda la biblioteca
map-online = Mapa en línea
map-builtin = Mapa integrado
map-online-tip = Se muestra OpenStreetMap. Haga clic para usar solo el mapa integrado (sin conexión a Internet).
map-builtin-tip = Se muestra el mapamundi integrado. Haga clic para mostrar los mapas detallados de OpenStreetMap (descarga imágenes de mapa desde Internet).
map-show-all = Mostrar todo
map-show-all-hint = Ajustar el zoom para mostrar todas las fotos en el mapa
map-search-online-hint = Buscar un lugar o una dirección
map-search-builtin-hint = Buscar una ciudad o unas coordenadas
map-no-city-builtin = No hay ninguna ciudad con ese nombre en la lista integrada.
map-no-city-hint = No hay ninguna ciudad con ese nombre. Pruebe con coordenadas, como 44,84, -0,58.
map-cities = CIUDADES
map-cities-builtin = CIUDADES (LISTA INTEGRADA)
map-searching = Buscando en línea…
map-places-osm = LUGARES Y DIRECCIONES (OPENSTREETMAP)
map-search-addresses = Buscar direcciones en línea   (Intro)
map-where = ¿Dónde se tomaron?
map-where-help = Busque un lugar arriba, o haga clic en el mapa para colocar un marcador. Después podrá arrastrarlo.
map-name-hint = Nombre del lugar (opcional)
map-drag-pin = Arrastre el marcador para ajustarlo.
map-drag-pin-offline = Arrastre el marcador para ajustarlo. Los mapas en línea muestran las calles.
map-set-n =
    { $n ->
        [one] Definir el lugar de { $n } foto
       *[other] Definir el lugar de { $n } fotos
    }
map-remove-location = Quitar el lugar
map-remove-location-hint = Estas fotos no tendrán ningún lugar, aunque la cámara hubiera guardado uno (los archivos no se modifican)
map-close-hint = Cerrar (Esc)
map-change-location = Cambiar el lugar…
map-select-in-library = Seleccionar en la biblioteca
map-select-in-library-hint = Seleccionar estas fotos en la biblioteca
map-show-hint = Mostrar en el mapa
map-preparing = Preparando el mapa…
map-hover = { $pictures } · { $place }
map-credit-builtin = Mapa integrado: Natural Earth · GeoNames
map-problem = { $problem }. Se muestra el mapa integrado.
map-empty-indexing = Buscando las posiciones GPS de sus fotos…
map-empty =
    Todavía no hay fotos con lugar.

    Las fotos de teléfonos y de cámaras con GPS aparecen aquí solas. Para situar las demás, selecciónelas y haga clic en { $icon } (Definir el lugar) en la barra de selección.

## Import (src/import_view.rs)

import-subtitle = Copiar fotos a su biblioteca
import-from = Desde
import-choose = Elegir…
import-refresh-hint = Buscar de nuevo tarjetas y discos, y volver a leer la carpeta
import-hide-dups = Ocultar las { $n } que ya están en la biblioteca
import-dups-hint = Fotos con el mismo contenido que una foto de su biblioteca, aunque tengan otro nombre
import-uncheck-all = Desmarcar todas
import-check-all = Marcar todas
import-copy-into = Copiar en
import-another-folder = Otra carpeta…
import-name-hint = Nombre de la carpeta nueva, p. ej. Verano en España
import-checked = { $chosen } de { $shown } marcadas. No se sobrescribe nada; los originales no se modifican.
import-go =
    { $n ->
        [one] Importar { $n } foto
       *[other] Importar { $n } fotos
    }
import-dest-dialog = Colocar la carpeta nueva en…
import-from-dialog = Importar fotos desde…
import-pick-dest = Elija dónde colocar la carpeta nueva.
import-running = Todavía hay una importación en curso: espere a que termine (vea la barra de estado).
import-started =
    { $n ->
        [one] Importando { $n } foto en { $path }…
       *[other] Importando { $n } fotos en { $path }…
    }
import-listing-in = Buscando fotos en { $from }…
import-none-in = No hay fotos en { $path }.
import-choose-another = Elegir otra carpeta…
import-heading = Importar desde una tarjeta de memoria, una cámara o una carpeta
import-copy-note = Las fotos se copian en una carpeta nueva de su biblioteca. Los originales no se modifican.
import-no-cards = No se encontró ninguna tarjeta de memoria ni cámara. Conecte una y haga clic en { $icon } arriba.
import-instead = Para mostrar fotos donde ya están, sin copiarlas:
import-add-instead = añada una carpeta a la biblioteca
import-all-dups =
    { $n ->
        [one] La foto ya está en su biblioteca.
       *[other] Las { $n } fotos ya están todas en su biblioteca.
    }
import-show-anyway = Mostrarlas de todos modos
import-truncated = Solo se enumeran las primeras { $n } fotos.
import-group-hint = Marcar o desmarcar todo este grupo
import-in-library = En la biblioteca
import-click-preview = Haga clic en una foto para verla aquí
import-this = Importar esta foto
import-this-hint = Espacio
import-already = ya está en su biblioteca

## Print (src/print_view.rs)

print-select-first = Seleccione primero las fotos que quiere imprimir.
print-page-layout = Diseño de página
print-layout-full = Página completa
print-shrink = Reducir para ajustar
print-shrink-hint = Se imprime la foto entera, con blanco donde las formas difieren
print-crop = Recortar para ajustar
print-crop-hint = La foto llena el formato de impresión; se cortan los bordes que sobresalen
print-options = Opciones de borde y de texto
print-border = Borde
print-no-border = Sin borde
print-thin-line = Línea negra fina
print-white-border = Borde blanco
print-text-under = Texto bajo cada foto
print-no-text = Sin texto
print-printer = Impresora
print-no-printer-found = No se encontró ninguna impresora
print-setup = Ajustes de la impresora…
print-setup-hint = Tamaño del papel, orientación y ajustes propios de la impresora
print-paper = Papel
print-default = Predeterminado de la impresora
print-no-answer = Ninguna impresora ha respondido: se muestra una página A4
print-paper-describe = { $dpi } ppp, { $w } x { $h } pulg. ({ $orientation })
print-copies = Copias por foto
print-copies-n =
    { $n ->
        [one] { $n } copia
       *[other] { $n } copias
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } impresión
       *[other] { $prints } impresiones
    }
print-cancel = Cancelar  { $icon }
print-go = Imprimir  { $icon }
print-review = Revisar
print-review-hint = Comprobar que cada foto tiene suficientes píxeles para su tamaño de impresión
print-none-left = No queda ninguna foto por imprimir.
print-job-printed =
    { $n ->
        [one] { $n } página enviada a { $printer }.
       *[other] { $n } páginas enviadas a { $printer }.
    }
print-preview = Vista previa
print-ready = Todas sus fotos están listas para imprimirse.
print-too-small = Algunas de sus fotos son demasiado pequeñas para imprimirse bien.  Puede quitar esas fotos, imprimirlas de todos modos, o cancelar y cambiar el tamaño de impresión.
print-quality-best = Calidad óptima ({ $ppi } píxeles/pulgada)
print-quality-good = Buena calidad ({ $ppi } píxeles/pulgada)
print-quality-bad = Mala calidad ({ $ppi } píxeles/pulgada)

## Results of background work (src/backend.rs)

backend-no-data-dir = No se encontró ninguna carpeta para los datos de la aplicación.
backend-cache-fail = No se pudo vaciar la caché ({ $error })
fail-export = No se puede exportar: error inesperado.
fail-trash = No se pueden mover las fotos a la papelera: error inesperado.
fail-email = No se puede preparar el correo: error inesperado.
fail-gallery = No se puede crear la galería web: error inesperado.
fail-movie = No se puede crear la película: error inesperado
fail-collage = No se puede guardar el collage: error inesperado.
export-nothing = No se exportó nada: { $error }.
export-result =
    { $ok } de { $total ->
        [one] { $total } foto exportada
       *[other] { $total } fotos exportadas
    } a { $path }
export-stopped = La exportación se detuvo antes de terminar.
export-failed-list = No exportadas:
export-more = … y { $n } más
gallery-ready =
    La galería web está lista:
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } foto no se pudo leer y se ha omitido.
       *[other] { $n } fotos no se pudieron leer y se han omitido.
    }
gallery-stopped = No se creó la galería web: se pidió detenerla.
gallery-failed = No se puede crear la galería web: { $error }
movie-ready =
    La película está lista:
    { $path }
movie-stopped = No se creó la película: se pidió detenerla.
movie-failed = No se puede crear la película: { $error }
collage-saved-many =
    { $n } imágenes guardadas, la primera es:
    { $path }
collage-saved =
    Guardado:
    { $path }
collage-save-failed = No se puede guardar el collage: { $error }
import-unexpected = La importación se detuvo inesperadamente.
import-summary =
    { $n ->
        [one] { $n } foto importada en { $path }.
       *[other] { $n } fotos importadas en { $path }.
    }
import-already-there =
    { $n ->
        [one] { $n } ya estaba.
       *[other] { $n } ya estaban.
    }
import-was-stopped = La importación se detuvo antes de terminar.
import-failed-count = No se pudieron copiar { $n }:

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = ene., feb., mar., abr., may., jun., jul., ago., sept., oct., nov., dic.
locale-months-long = enero, febrero, marzo, abril, mayo, junio, julio, agosto, septiembre, octubre, noviembre, diciembre
locale-date = { $day } { $month } { $year }
locale-decimal = ,
locale-group = .
locale-unit-kb = KB
locale-unit-mb = MB
locale-unit-gb = GB

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Escriba un nombre para la carpeta nueva.
import-name-chars = Un nombre de carpeta no puede contener / \ : * ? " < > |
import-name-long = Este nombre de carpeta es demasiado largo.
import-drive = Disco { $letter }:
import-mkdir-fail = No se pudo crear la carpeta { $path }: { $error }
import-too-many = demasiados archivos tienen este nombre
import-appeared = ha aparecido un archivo con este nombre durante la copia
export-logo-bad = no se puede leer el logotipo { $path } ({ $error })
export-unreadable = no se pudo leer la foto
gallery-too-many = demasiadas galerías tienen este nombre aquí
gallery-none-read = no se pudo leer ninguna de las fotos
movie-no-pictures = no hay fotos
movie-ffmpeg-start = No se pudo iniciar FFmpeg: { $error }
movie-ffmpeg-input = FFmpeg no tiene entrada
movie-ffmpeg-stopped = FFmpeg se detuvo ({ $error })
movie-ffmpeg-failed =
    FFmpeg ha fallado:
    { $tail }
root-already = Esta carpeta ya está en la biblioteca.
root-inside = Esta carpeta ya está en la biblioteca, dentro de { $outer }.
root-excluded = That folder is inside { $folder }, which is left out of the library.
exclude-is-root = That is a library folder: remove it instead.
exclude-outside = That folder is not inside a library folder.
exclude-already = That folder is already left out.
online-tile-bad = no se pudo leer una tesela de mapa: { $error }
online-map-down = Mapa en línea no accesible ({ $error })
online-search-down = La búsqueda en línea no es accesible ({ $error }).
online-search-odd = Respuesta inesperada de la búsqueda en línea: { $error }
collage-nothing = no hay nada que guardar
collage-extension = guarde como .pdf, .png o .jpg
collage-page-fail = no se pudo dibujar una página
geo-near = Cerca de { $name }
mail-no-pictures = no hay fotos que adjuntar
mail-copies-fail = No se pudieron hacer las copias para el correo: { $error }
mail-stopped = No se preparó el correo: se pidió detenerlo.
mail-none-read = No se pudieron hacer las copias para el correo: no se pudo leer ninguna de las fotos.
mail-left-out =
    { $n ->
        [one] { $n } foto no se pudo leer y se ha omitido.
       *[other] { $n } fotos no se pudieron leer y se han omitido.
    }
mail-opened =
    Se ha abierto en su programa de correo un mensaje nuevo con { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    }.{ $left_out }
mail-asked =
    Se ha pedido a su programa de correo un mensaje nuevo con { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    }. Si no están adjuntas, adjúntelas desde:
    { $path }{ $left_out }
mail-open-fail =
    No se pudo abrir su programa de correo ({ $error }).
    Las copias pequeñas de { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    } están en:
    { $path }
    Adjúntelas a su mensaje.{ $left_out }
mail-too-many = demasiadas fotos
mail-no-mapi = no hay programa de correo (mapi32.dll: { $error })
mail-no-attach = el programa de correo no acepta archivos adjuntos (no hay MAPISendMailW)
mail-refused = el programa de correo rechazó el mensaje (error MAPI { $code })
mail-open-mail = no se pudo abrir Mail: { $error }
mail-open-mail-short = no se pudo abrir Mail
mail-no-xdg = xdg-email (de xdg-utils) no está instalado
mail-xdg-fail = no se pudo ejecutar xdg-email: { $error }
gallery-html-lang = es
gallery-count = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
gallery-close = Cerrar
gallery-prev = Anterior
gallery-next = Siguiente
sheet-count = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
