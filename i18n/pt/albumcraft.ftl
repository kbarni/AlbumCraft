# Texto da interface do AlbumCraft, em português (do Brasil).

language-name = Português

## Shared buttons and words

button-ok = OK
button-cancel = Cancelar
button-close = Fechar
button-remove = Remover
button-save = Salvar
add-folder = Adicionar pasta…
scan-again = Analisar novamente
removed-root = { $path } foi removida da biblioteca. As fotos e as suas anotações sobre elas estão intactas.

## Settings dialog

settings-title = Configurações
settings-saved-note = As alterações são salvas na hora.

settings-section-interface = Interface
settings-section-folders = Pastas
settings-section-editing = Edição
settings-section-slideshow = Apresentação
settings-section-export = Exportação
settings-section-maps = Mapas
settings-section-storage = Armazenamento
settings-section-ai = Recursos de IA

## Settings > Interface

settings-language = Idioma
settings-language-system = Igual ao sistema ({ $language })
settings-language-note = Outros idiomas podem ser adicionados como arquivos de tradução; veja i18n/ no código-fonte.
settings-theme = Tema
theme-system = Igual ao sistema
theme-light = Claro
theme-dark = Escuro

## Dialogs (src/dialogs.rs)

dialog-new-album = Novo álbum
dialog-rename-album = Renomear álbum
dialog-tags = Tags e pessoas
dialog-gallery = Galeria web
dialog-folder-note = Descrição da pasta
dialog-movie = Filme
dialog-about = Sobre o AlbumCraft
dialog-online-maps = Mapas
dialog-print-review = Revisão
dialog-trash = Mover para a lixeira

new-album-name = Nome do álbum:
new-album-adds = { $n ->
    [one] A foto selecionada será adicionada a ele.
   *[other] As { $n } fotos selecionadas serão adicionadas a ele.
}
new-album-create = Criar
rename-album-rename = Renomear

tags-intro = { $n ->
    [one] Para adicionar à foto selecionada. Separe as palavras com vírgulas.
   *[other] Para adicionar às { $n } fotos selecionadas. Separe as palavras com vírgulas.
}
tags-tags = Tags:
tags-people = Pessoas:
tags-add = Adicionar

gallery-intro = { $n ->
    [one] Uma página web com uma foto e as suas edições, que pode ser aberta em qualquer navegador ou colocada em um serviço de hospedagem.
   *[other] Uma página web com { $n } fotos e as suas edições, que pode ser aberta em qualquer navegador ou colocada em um serviço de hospedagem.
}
gallery-title = Título:
gallery-style = Estilo:
gallery-size = Tamanho das fotos:
gallery-size-medium = Médio (1280 pixels, arquivos leves)
gallery-size-large = Grande (2048 pixels, para telas grandes)
gallery-captions = Mostrar as legendas
gallery-make = Escolher a pasta e criar…

folder-note-intro = Uma breve descrição mostrada abaixo do nome da pasta:


# $fade and $motion are "yes" or "no"
movie-intro = { $n ->
    [one] Um vídeo de uma foto como na apresentação: { $seconds } s
   *[other] Um vídeo de { $n } fotos como na apresentação: { $seconds } s cada
}{ $fade ->
    [yes] , com transições suaves
   *[no] {""}
}{ $motion ->
    [yes] , com panorâmica e zoom
   *[no] {""}
}.
movie-length = Duração: cerca de { $seconds } s. Altere os tempos e os efeitos na barra da apresentação ou em Configurações.
movie-music = Música
movie-other-music = Outra música…
movie-add-music = Adicionar música…
movie-no-ffmpeg = Para criar filmes é necessário o software livre FFmpeg, que não foi encontrado.
movie-install-ffmpeg = Instale-o em ffmpeg.org (ou pelo gerenciador de pacotes do seu sistema) e reabra esta janela.
movie-make = Escolher o arquivo e criar…
movie-preview = Visualizar como apresentação

# $count is $n written with thousands separators; $size is like "12.5 MB"
trash-question = { $n ->
    [one] Mover a foto rejeitada ({ $size }) para a lixeira?
   *[other] Mover as { $count } fotos rejeitadas ({ $size }) para a lixeira?
}
trash-explain = Os arquivos são removidos de suas pastas. Você pode recuperá-los na lixeira do sistema enquanto ela não for esvaziada.
trash-move = Mover para a lixeira

print-review-remove = Remover essas fotos
print-review-print = Imprimir mesmo assim

about-text = Um organizador de fotos para o computador: navegue pelas suas pastas de fotos, crie álbuns, escolha, rejeite e avalie as fotos, corrija-as e retoque-as, e inicie apresentações.
about-edits = As edições nunca alteram os seus arquivos originais; elas são aplicadas quando você vê ou exporta uma foto.
about-credits = Mapa integrado: Natural Earth (domínio público). Lista de cidades: GeoNames (CC BY 4.0). Mapas online: © colaboradores do OpenStreetMap (ODbL).

online-maps-question = Mostrar mapas detalhados da internet?
online-maps-what = O AlbumCraft pode mostrar onde as suas fotos foram tiradas. Com os mapas online, ele baixa mapas do OpenStreetMap e pode procurar endereços quando você informa um lugar.
online-maps-privacy = As suas fotos e os locais delas nunca são enviados. Os servidores de mapas veem o seu endereço de internet, as áreas que você olha e os lugares que você procura.
online-maps-offline = Sem os mapas online, o AlbumCraft usa o seu mapa-múndi e a lista de cidades integrados e não se conecta à internet.
online-maps-later = Você pode mudar isso depois no mapa ou em Configurações (menu principal): Mapas.
online-maps-use = Usar os mapas online
online-maps-builtin = Somente o mapa integrado

## Notices at the bottom of the window

notice-undo = Desfazer
notice-show = Mostrar na pasta
notice-close = Fechar esta mensagem
notice-no-file-manager = Não foi possível abrir o gerenciador de arquivos: { $error }

## Settings dialog sections (src/settings_view.rs)

count-pictures = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}

settings-bar = Barra na parte inferior da biblioteca
settings-bar-compact = Compacta: uma única linha, mostrada quando há fotos selecionadas
settings-bar-tray = Bandeja: miniaturas das fotos selecionadas e botões grandes
settings-bar-note = Na barra compacta, os botões perdem o nome quando a janela é estreita. Quando os botões não cabem, a barra rola para o lado.
settings-thumb-size = Tamanho das fotos na biblioteca

settings-folders-intro = A biblioteca mostra as fotos dessas pastas e de suas subpastas onde elas estão: nada é copiado nem movido. Para copiar fotos de um cartão de memória ou de uma câmera, use Importar.
settings-folders-none = Nenhuma pasta ainda: fotos de exemplo são mostradas no lugar.
settings-folders-remove-hint = Deixar de mostrar esta pasta (as fotos dela e as suas anotações sobre elas são mantidas)
settings-folders-rescan-hint = Procurar agora fotos novas, alteradas e excluídas
settings-excluded-title = Left out folders
settings-excluded-intro = Folders inside a library folder that are not shown, with everything under them (for example Screenshots).
settings-excluded-none = Nothing is left out.
settings-excluded-add = Leave out a folder
settings-excluded-remove-hint = Show this folder again

settings-edit-opens = A aba Editar abre em
settings-edit-basic = Básico: correções com um clique e ferramentas
settings-edit-advanced = Avançado: os controles deslizantes
settings-info-panel = Painel de informações
settings-info-show = Mostrar o painel de informações no editor (I)

settings-timing = Tempos
settings-next-after = Próxima foto após
settings-note-paused = A barra de espaço pausa a apresentação; ← e → permitem navegar.
settings-note-manual = Cada foto permanece na tela até você passar para a próxima com → ou com os botões. O visualizador em tela cheia (F11) continua avançando sozinho.
settings-effects = Efeitos
settings-fade = Transição suave entre as fotos
settings-motion = Panorâmica e zoom lentos
settings-motion-subtle = Sutil
settings-motion-medium = Médio
settings-captions = Mostrar as legendas
settings-viewer-note = O visualizador em tela cheia (F11 no editor) sempre começa sem transição suave e sem panorâmica e zoom.
settings-music = Música
settings-music-play = Tocar música durante as apresentações
settings-music-another = Escolher outra…
settings-music-choose = Escolher música…
settings-movies-note = Os filmes feitos com fotos também usam essas configurações.

settings-export-note = Com o que a tela Exportar começa.
settings-export-size = Tamanho das fotos
settings-export-quality = Qualidade JPEG
settings-export-info = Informações nas cópias
settings-export-gps = Incluir o local da foto (GPS)
settings-export-gps-hint = Desative antes de compartilhar fotos se não quiser que outras pessoas vejam o local.
settings-export-exif = Incluir os dados da câmera (EXIF)
settings-export-exif-hint = Câmera, lente, exposição e data da foto, copiados do arquivo original. O local segue a configuração acima.
settings-creator = Autor
settings-creator-hint = Seu nome
settings-copyright = Direitos autorais
settings-copyright-hint = © Seu nome
settings-credit-note = Gravados em cada cópia (EXIF), no lugar dos do original. As cópias enviadas por e-mail também seguem essas configurações. A assinatura ou o logotipo é escolhido na tela Exportar.

settings-maps-online = Mapas online
settings-maps-use = Usar os mapas online (OpenStreetMap)
settings-maps-note = Mapas detalhados e busca de endereços. As suas fotos e os locais delas nunca são enviados; os servidores de mapas veem o seu endereço de internet, as áreas que você olha e os lugares que você procura. Desativado: somente o mapa-múndi e a lista de cidades integrados, e nenhuma conexão com a internet.
settings-maps-tiles = Blocos de mapa baixados
settings-maps-tiles-after = Os blocos de mapa são baixados novamente quando necessário.

settings-notes = Suas anotações
settings-notes-note = Anotações, legendas, tags, edições e álbuns ficam em um único arquivo; os seus arquivos de fotos nunca são alterados.
settings-thumbs = Miniaturas
settings-thumbs-after = As miniaturas são refeitas a partir das fotos quando necessário (mais lento por um tempo).
cache-on-disk = { $size } no disco
cache-measuring = Medindo…
cache-empty = Esvaziar

settings-ai-soon = Ainda não disponível. Previsto:
settings-ai-faces = Rostos
settings-ai-faces-text = detectar os rostos e agrupá-los por pessoa; quem dá nome aos grupos é você. Um download de cerca de 10 a 40 MB.
settings-ai-search = Busca inteligente
settings-ai-search-text = encontrar fotos descrevendo-as (“neve”, “bolo de aniversário”). Leve (inglês, cerca de 100 MB) ou multilíngue (cerca de 400 MB).
settings-ai-tags = Sugestões de tags
settings-ai-tags-text = tags sugeridas com base no conteúdo da foto.
settings-ai-local = Tudo será calculado neste computador; nada será enviado a lugar nenhum.

export-size-original = Tamanho original
export-size-very-large = Muito grande (2560 pixels)
export-size-large = Grande (1600 pixels)
export-size-small = Pequeno, para e-mail (800 pixels)

## Export view (src/export_view.rs)

filter-pictures = Fotos
view-back-editor = Editor
view-back-library = Biblioteca
view-back-hint = Voltar (Esc)

export-title = Exportar
export-count-all = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
export-count-some = { $n } de { $total } fotos
export-select-first = Selecione algumas fotos primeiro.
export-logo-unreadable = O logotipo { $path } não pode ser lido.
export-task = { $n ->
    [one] Exportando { $n } foto
   *[other] Exportando { $n } fotos
}
export-save-into = Salvar em
export-choose-folder = Escolher pasta…
export-choose-folder-hint = Escolher a pasta das cópias
export-safe = Nada é sobrescrito; os originais não são alterados.
export-go = { $n ->
    [one] Exportar { $n } foto
   *[other] Exportar { $n } fotos
}
export-dialog-title = Exportar as fotos para…

export-size = Tamanho
export-size-other = Outro:
export-size-pixels = { " " }pixels
export-size-note = O lado maior. As fotos nunca são ampliadas.
export-quality = Qualidade JPEG

export-signature = Assinatura
export-signature-on = Adicionar uma assinatura ou um logotipo
export-sig-text = Texto
export-sig-logo = Imagem (logotipo)
export-logo-other = Outra imagem…
export-logo-choose = Escolher uma imagem…
export-logo-dialog = Imagem do logotipo ou da assinatura
export-logo-note = Um PNG com fundo transparente dá o melhor resultado.
export-sig-text-hint = Seu nome, ou © Seu nome
export-sig-position = Posição
export-sig-size = Tamanho
export-sig-size-hint = Em relação ao lado menor da foto
export-sig-opacity = Opacidade
export-sig-colour = Cor
export-sig-shadow = Sombra
export-sig-shadow-hint = Um leve brilho para que o texto continue legível em qualquer foto

export-info = Informações nos arquivos
export-gps = Local da foto (GPS)
export-gps-hint = Desative antes de compartilhar fotos se não quiser que outras pessoas vejam o local.
export-exif = Dados da câmera (EXIF)
export-exif-hint = Câmera, lente, exposição e data da foto, copiados do arquivo original.
export-creator = Autor
export-creator-hint = Seu nome
export-copyright = Direitos autorais
export-copyright-hint = © Seu nome
export-credit-note = Gravados em cada cópia. A assinatura, o autor e os direitos autorais são lembrados para a próxima vez (também em Configurações, Exportação).

export-left-out = Excluída: clique para exportá-la também
export-leave-out = Clique para excluir esta foto

## Library: top bar, sidebar, grid (src/shell.rs)

nav-all = Todas as fotos
nav-picks = Escolhidas
nav-places = Locais
nav-places-hint = Suas fotos em um mapa
nav-albums = Álbuns
nav-people = Pessoas
nav-folders = Pastas
people-hint = Indique as pessoas pela barra de seleção.
scanning = Procurando fotos…

bar-import = Importar
bar-import-hint = Copiar fotos de um cartão de memória, de uma câmera ou de uma pasta para a sua biblioteca
bar-folders = Pastas
bar-folders-hint = As pastas mostradas na biblioteca, onde elas estão (nada é copiado)
bar-size-hint = Tamanho das fotos
bar-picks-hint = Mostrar somente as fotos escolhidas
bar-hide-rejects = Ocultar as fotos rejeitadas

sort-name = Nome do arquivo
sort-taken = Data da foto
sort-rating = Avaliação
sort-hint = Ordem das fotos: { $order }

filter-any = Todas
filter-any-no-rejects = Todas, exceto as rejeitadas
filter-min = { $n }+
filter-min-no-rejects = { $n }+, exceto as rejeitadas
filter-hint = Filtrar por avaliação; ocultar as fotos rejeitadas
filter-any-rating = Todas as avaliações
filter-or-more = { $rating } ou mais

album-rename = Renomear…
album-delete = Excluir o álbum
album-show-map = Mostrar no mapa
album-slideshow = Reproduzir como apresentação
album-deleted = O álbum “{ $name }” foi excluído. As fotos dele estão intactas.

empty-scanning = Procurando fotos…
empty-no-match = Nenhuma foto corresponde à sua busca ou ao seu filtro.
empty-album = Este álbum está vazio. Selecione fotos e adicione-as pela barra de seleção.
empty-person = Ninguém foi indicado ainda.
empty-picks = Nenhuma foto escolhida ainda. Passe o mouse sobre uma foto e clique na bandeira dela, ou pressione P.
empty-unreachable = Nenhuma foto foi encontrada nas pastas da sua biblioteca. Se elas estiverem em um disco não conectado, conecte-o e analise novamente.
empty-library = Sua biblioteca está vazia. Importe fotos ou adicione uma pasta que você já tenha.
empty-folders-hint = As pastas mostradas na biblioteca
empty-import = Importar fotos
empty-import-hint = Copiar fotos de um cartão de memória, de uma câmera ou de uma pasta
empty-add = Adicionar uma pasta
empty-add-hint = Mostrar as fotos de uma pasta onde elas estão, sem copiá-las

folder-play-hint = Reproduzir esta pasta como apresentação
folder-edit-hint = Alterar a descrição
folder-map-hint = Mostrar esta pasta no mapa
tile-pick-hint = Escolher (P)
tile-reject-hint = Rejeitar (X)

## Library: menus, bars, tray (src/library_view.rs)

rating-none = Sem avaliação
rating-stars = { $n ->
    [one] { $n } estrela
   *[other] { $n } estrelas
}
flag-pick = Escolher
flag-unpick = Desfazer a escolha
flag-reject = Rejeitar
flag-unreject = Desfazer a rejeição
flag-rating = Avaliação
album-new = Novo álbum…
menu-copy-edits = Copiar as edições
menu-paste-edits = Colar as edições
menu-paste-n = { $n ->
    [one] Colar { $n } edição
   *[other] Colar { $n } edições
}
copy-edits-select-one = Selecione apenas uma foto para copiar as edições dela.
menu-import = Importar fotos…
menu-add-folder = Adicionar uma pasta à biblioteca…
menu-folders = Pastas da biblioteca…
menu-scan = Analisar as pastas novamente
menu-select-all = Selecionar tudo
menu-select-none = Limpar a seleção
menu-rotate-left = Girar para a esquerda
menu-rotate-right = Girar para a direita
menu-trash-rejects = Mover as fotos rejeitadas para a lixeira…
menu-trash-rejects-hint = Remover de suas pastas os arquivos das fotos rejeitadas, após confirmação
menu-contact-sheet = Folha de contato…
menu-contact-sheet-hint = Páginas de fotos pequenas com um título e os nomes dos arquivos, para imprimir ou salvar em PDF
menu-undo-edits = Desfazer todas as edições
menu-settings = Configurações…
menu-about = Sobre o AlbumCraft…
menu-exit = Sair
edits-removed = { $n ->
    [one] As edições de { $n } foto foram removidas.
   *[other] As edições de { $n } fotos foram removidas.
}

status-library = Biblioteca: { $folders ->
    [one] { $folders } pasta
   *[other] { $folders } pastas
}, { $pictures ->
    [one] { $pictures } foto
   *[other] { $pictures } fotos
}, { $size } no disco
status-importing = Importando { $done } de { $total }…
status-stop = Parar
status-stop-hint = Parar após a foto atual (as já copiadas permanecem)
task-running = { $label }…
task-stopping = { $label }: parando…
task-cancel-hint = Parar após a foto atual
status-samples = Fotos de exemplo estão sendo mostradas. Use Importar ou Pastas para adicionar as suas.

selected-count = { $n ->
    [one] { $n } foto
   *[other] { $count } fotos
}
selected-all = { $n ->
    [one] { $pictures } selecionada
   *[other] { $pictures } selecionadas
}
selected-set = { $name } · { $pictures }
fallback-folder = Pasta
fallback-album = Álbum
tray-nothing = Nada selecionado
tray-click = Clique nas fotos para selecioná-las

output-web = Página web
output-web-tip = Criar uma galeria web com essas fotos
output-mail = E-mail
output-mail-tip = Pequenas cópias, anexadas a uma nova mensagem no seu programa de e-mail
output-print = Imprimir
output-print-tip = Imprimir essas fotos (Ctrl+P)
output-export = Exportar
output-export-tip = Salvar cópias editadas em uma pasta
output-collage = Colagem
output-collage-tip = Criar uma colagem ou uma folha de contato
output-movie = Filme
output-movie-tip = Criar um filme

bar-clear-hint = Limpar a seleção (Esc)
bar-pick-hint = Escolher / desfazer a escolha (P)
bar-reject-hint = Rejeitar / desfazer a rejeição (X)
bar-rotate-left = Girar para a esquerda (Ctrl+[)
bar-rotate-right = Girar para a direita (Ctrl+])
bar-tags = Tags e pessoas
bar-location = Definir o local: onde essas fotos foram tiradas, em um mapa
bar-to-album = Para um álbum
bar-clear = Limpar

ctx-open = Abrir no editor
ctx-slideshow = Reproduzir a apresentação a partir daqui
ctx-add-to-album = Adicionar a um álbum
ctx-remove-from-album = Remover do álbum
ctx-tags = Tags e pessoas…
ctx-location = Definir o local…
ctx-export = Exportar…
ctx-show-file = Mostrar no gerenciador de arquivos

## Names of looks, effects and edit steps (src/labels.rs)

look-vivid = Vívido
look-vivid-tip = Cores ricas e saturadas de filme para slides, com pretos profundos
look-soft-film = Filme suave
look-soft-film-tip = Delicado filme de retrato: contraste suave, quente e favorável à pele
look-polaroid = Instantâneo
look-polaroid-tip = Cópia desbotada de câmera instantânea: pretos elevados, realces quentes, sombras frias
look-cross-process = Processamento cruzado
look-cross-process-tip = Realces amarelos, sombras azul-esverdeadas e contraste marcante
look-lomo = Lomo
look-lomo-tip = Visual saturado de câmera de brinquedo, com cantos escuros
look-matte = Fosco
look-matte-tip = Pretos chapados e desbotados, cores atenuadas
look-teal-orange = Azul-esverdeado e laranja
look-teal-orange-tip = Correção de cor de cinema: sombras azul-esverdeadas, pele e realces quentes
look-high-key = High key
look-high-key-tip = Luminoso e leve, com sombras abertas
look-cool = Frio
look-cool-tip = Uma dominante mais fria, mais azul
look-warm = Quente
look-warm-tip = Uma dominante mais quente, dourada
look-bw = P&B
look-bw-tip = Preto e branco clássico
look-dramatic-bw = P&B dramático
look-dramatic-bw-tip = Preto e branco com filtro vermelho: céus escuros, forte contraste local
look-sepia = Sépia
look-sepia-tip = Tom marrom de foto antiga
look-soft-focus = Foco suave
look-soft-focus-tip = Centro nítido, bordas suaves e sonhadoras
look-glow = Brilho
look-glow-tip = Um suave halo luminoso
look-film-grain = Granulação de filme
look-film-grain-tip = Granulação no estilo de filme
look-focal-bw = P&B focal
look-focal-bw-tip = Cor no centro, preto e branco ao redor
effect-sharpen = Nitidez
effect-sepia = Sépia
effect-bw = P&B
effect-warmify = Aquecer
effect-film-grain = Granulação de filme
effect-tint = Matiz
effect-saturation = Saturação
effect-soft-focus = Foco suave
effect-glow = Brilho
effect-filtered-bw = P&B filtrado
effect-focal-bw = P&B focal
effect-graduated-tint = Matiz graduado
op-rotate-right = Girar para a direita
op-rotate-left = Girar para a esquerda
op-crop = Recortar
op-straighten = Endireitar { $angle }°
op-perspective-v = Perspectiva, vertical { $v }
op-perspective-h = Perspectiva, horizontal { $h }
op-perspective-both = Perspectiva { $v } / { $h }
op-redeye = Olhos vermelhos
op-lucky = Correção automática (sorte)
op-auto-contrast = Contraste automático
op-auto-color = Cor automática
op-tuning = Ajuste fino
op-look = { $look } { $amount } %
op-adjust = Ajuste
op-adjust-parts = Ajuste: { $parts }
adj-wb = balanço de branco
adj-exposure = exposição
adj-highlights = realces
adj-shadows = sombras
adj-blacks = pretos
adj-contrast = contraste
adj-saturation = saturação
adj-vibrance = vibração
adj-clarity = clareza
adj-sharpen = nitidez
adj-noise = redução de ruído
decor-vignette-light = Vinheta clara
decor-vignette = Vinheta
decor-instant = Moldura de instantâneo
decor-border = Borda
decor-shadow = Sombra projetada
decor-rounded = Cantos arredondados
decor-text = Texto “{ $text }”
wb-as-shot = Como foi tirada
wb-daylight = Luz do dia
wb-cloudy = Nublado
wb-shade = Sombra
wb-tungsten = Tungstênio
wb-fluorescent = Fluorescente
wb-flash = Flash
wb-custom = Personalizado
wb-auto = Automático
gallery-style-light = Grade clara
gallery-style-dark = Grade escura
gallery-style-masonry = Mosaico (fotos inteiras)
crop-free = Livre
crop-original = Formato original
crop-square = Quadrado
crop-4-3 = 4 × 3
crop-3-2 = 3 × 2 (cópia 10 × 15)
crop-16-9 = 16 × 9 (tela panorâmica)

## Editor (src/edit_view.rs)

edit-slider-hint =
    { $tip }
    Clique duplo para redefinir
edit-loading = A foto ainda está sendo carregada.
edit-straightened = Endireitada em { $angle }°.
edit-already-level = Esta foto já parece estar reta.
edit-no-horizon = Nenhum horizonte nem linha vertical nítida foi encontrado para endireitar.
edit-histogram = Histograma e informações da câmera
edit-unknown-camera = Câmera desconhecida
edit-35mm = (equivalente a 35 mm: { $mm } mm)
edit-focal = Distância focal: { $mm } mm
edit-iso = ISO: { $iso }
edit-no-camera = Nenhuma informação da câmera nesta foto
edit-clip-shadows = sombras { $pct } %
edit-clip-highlights = realces { $pct } %
edit-clipped = Corte: { $parts }
edit-clipped-hint = Parte da foto que é preto puro ou branco puro em pelo menos uma cor
edit-back = Voltar para a biblioteca
edit-info = Info
edit-info-hint = Propriedades, local e tags (I)
edit-side-by-side-hint = Mostrar as fotos lado a lado
edit-stacked-hint = Mostrar uma foto acima da outra
edit-focus-hint = Mudar a foto que está sendo editada (a que recebe as suas alterações)
edit-compare = Comparar
edit-compare-hint = Esta foto ao lado de outra, lado a lado
edit-original = Original
edit-original-hint = Mantenha pressionado para ver a foto sem as suas edições (ou mantenha pressionado O)
edit-slideshow = Apresentação
edit-fullscreen-hint = Tela cheia (F11): a partir desta foto, uma após a outra, com música
edit-next = Próxima foto
edit-previous = Foto anterior
edit-position = { $n } de { $total }
edit-tab-edit = Editar
edit-tab-presets = Predefinições
edit-tab-effects = Efeitos
edit-undo = Desfazer
edit-undo-hint = Desfazer a última alteração (Ctrl+Z)
edit-redo = Refazer
edit-list = Edições desta foto:
edit-list-none = (nenhuma: é o original)
edit-step-hint =
    { $label }
    Clique para voltar a esta etapa (as seguintes são removidas; Desfazer as traz de volta)
edit-copy-hint = Copiar essas edições para colá-las em outras fotos (Ctrl+Shift+C)
edit-paste-n = Colar { $n }
edit-paste-hint = Adicionar as edições copiadas a esta foto (Ctrl+Shift+V)
edit-basic = Básico
edit-advanced = Avançado
edit-wb-help = Clique em um elemento da foto que deveria ser cinza neutro ou branco.
edit-crop-help = Arraste as bordas ou os cantos da moldura, arraste dentro dela para movê-la, ou desenhe uma nova moldura. Enter aplica.
edit-crop-shape = Formato
edit-crop-orientation = Orientação
edit-landscape = Paisagem
edit-landscape-hint = Mais larga que alta
edit-portrait = Retrato
edit-portrait-hint = Mais alta que larga
edit-apply = Aplicar
edit-remove-crop = Remover o recorte
edit-remove-crop-hint = Mostrar novamente a foto inteira
edit-redeye-help = Desenhe uma moldura ao redor dos olhos vermelhos. Somente as pupilas vermelhas dentro da moldura são escurecidas.
edit-straighten-title = Endireitar
edit-straighten-help = Mova o controle deslizante até o horizonte ficar alinhado com a grade.
edit-oneclick = Correções com um clique
edit-lucky = Estou com sorte
edit-lucky-hint = Exposição, balanço de branco, ponto preto e um pouco de vibração, tudo de uma vez (depois refine em Avançado)
edit-auto-exposure = Exposição automática
edit-auto-exposure-hint = Clarear ou escurecer para uma exposição equilibrada
edit-auto-colour = Cor automática
edit-auto-colour-hint = Balanço de branco automático: remover uma dominante de cor
edit-auto-contrast = Contraste automático
edit-auto-contrast-hint = Esticar os tons mais escuros e mais claros
edit-auto-straighten = Endireitar automaticamente
edit-auto-straighten-hint = Endireitar um horizonte ou verticais inclinados
edit-tools = Ferramentas
edit-pick-neutral = Ponto neutro
edit-pick-neutral-hint = Clique em um elemento cinza para ajustar o balanço de branco
edit-sec-wb = BALANÇO DE BRANCO
edit-sec-light = LUZ
edit-sec-colour = COR
edit-sec-detail = DETALHE
edit-sec-geometry = GEOMETRIA
edit-wb-pick = Escolher
edit-wb-pick-hint = Clique em um elemento cinza da foto
edit-s-temperature = Temperatura
edit-s-temperature-tip = Mais azul (à esquerda) ou mais quente (à direita)
edit-s-tint = Matiz
edit-s-tint-tip = Mais verde (à esquerda) ou mais magenta (à direita)
edit-s-exposure = Exposição
edit-s-exposure-tip = Brilho geral, em pontos de exposição
edit-s-highlights = Realces
edit-s-highlights-tip = Áreas claras: à esquerda recupera detalhes, à direita clareia
edit-s-shadows = Sombras
edit-s-shadows-tip = Áreas escuras: à direita as abre
edit-s-blacks = Pretos
edit-s-blacks-tip = Ponto preto: à esquerda aprofunda, à direita eleva
edit-s-contrast = Contraste
edit-s-contrast-tip = Distância entre o escuro e o claro
edit-s-saturation = Saturação
edit-s-saturation-tip = Todas as cores
edit-s-vibrance = Vibração
edit-s-vibrance-tip = Principalmente as cores opacas; poupa a pele e as cores já vivas
edit-s-clarity = Clareza
edit-s-clarity-tip = Contraste local nos tons médios
edit-s-sharpen = Nitidez
edit-s-sharpen-tip = Detalhes finos mais nítidos
edit-s-noise = Ruído
edit-s-noise-tip = Redução de ruído: suaviza a granulação mantendo os contornos
edit-reset-all = Redefinir tudo
edit-crop-btn = Recortar…
edit-crop-btn-hint = Escolher a parte da foto que fica, ou alterar o recorte
edit-s-straighten-tip = Girar um pouco a foto para endireitar o horizonte (amplia para esconder os cantos)
edit-s-vertical = Vertical
edit-s-vertical-tip = Perspectiva: à direita alarga a parte de cima, para endireitar prédios que parecem cair para trás quando fotografados de baixo
edit-s-horizontal = Horizontal
edit-s-horizontal-tip = Perspectiva: à direita amplia o lado direito, para uma parede vista de lado
edit-intensity = Intensidade
edit-remove-preset = Remover a predefinição
edit-presets-help = Visuais para a foto inteira. Clique em um e depois ajuste a intensidade dele.
edit-effects-add = Adicionar sobre ou ao redor da foto
edit-add-instant = Moldura de instantâneo
edit-add-text = Texto
edit-your-text = Seu texto
edit-effects-note = Os efeitos são aplicados por cima de todas as outras edições. Adicione um para ajustá-lo aqui.
edit-p-amount = Intensidade
edit-p-size = Tamanho
edit-p-width = Largura
edit-p-bottom = Inferior
edit-p-colour = Cor
edit-p-darkness = Escuridão
edit-p-background = Fundo
edit-p-radius = Raio
edit-vignette-note = À esquerda do meio, os cantos são clareados em vez disso.
edit-halo = Brilho leve (legível em qualquer foto)
edit-remove-effect = Remover este efeito
edit-caption-hint = Adicione uma legenda!
edit-print-hint = Imprimir esta foto (Ctrl+P)
edit-album = Álbum
edit-zoom-hint = Zoom (100 % = os pixels reais da foto)
edit-zoom-slider-hint = Zoom: à esquerda = ajustada, no meio = 1:1, à direita = 400 % (a roda do mouse também funciona; arraste a foto para ampliar a 100 % e movê-la)
edit-one-to-one-hint = Tamanho original (100 %): um pixel da foto por pixel da tela
edit-fit = Ajustar
edit-fit-hint = Ajustar a foto inteira à janela (um clique duplo em uma foto ampliada faz o mesmo)
edit-people = Pessoas:
edit-person-remove = Remover a pessoa
edit-person-add = adicionar uma pessoa
edit-compare-none = Nenhuma outra foto nesta pasta
edit-compare-selected = Selecionada
edit-compare-swap-hint = Clique para editar esta foto em vez da outra
edit-view-loading = Carregando…
edit-sharpening = Aplicando nitidez…
edit-zoom-label = Zoom { $pct } %
edit-unreadable = Esta foto não pode ser lida.
edit-full-unreadable = A foto completa não pode ser lida.
edit-full-loading = Carregando a foto completa…
edit-draw-box = Arraste sobre a foto para desenhar uma moldura
edit-click-neutral = Clique em um elemento que deveria ser cinza ou branco
edit-wb-title = Balanço de branco

## Info panel, search, slideshow bar

info-hide-hint = Ocultar este painel (I)
info-properties = Propriedades
info-camera = Câmera
info-lens = Lente
info-taken = Tirada em
info-file-date = { $date } (data do arquivo)
info-size = Tamanho
info-megapixels = { $w } × { $h } ({ $mp } MP)
info-file = Arquivo
info-focal = Distância focal
info-focal-35 = { $f } mm (35 mm: { $e } mm)
info-aperture = Abertura
info-exposure = Exposição
info-folder = Pasta
info-sample = Foto de exemplo integrada
info-edits = Edições
info-no-exif = Nenhuma informação da câmera (EXIF) nesta foto.
info-exif = EXIF detalhado
info-flash = Flash
info-bias = Compensação de exposição
info-program = Programa
info-metering = Medição
info-distance = Distância
info-colour-space = Espaço de cor
info-gps = Posição GPS
info-digitized = Digitalizada
info-modified = Modificada
info-software = Software
info-preview = Visualização da câmera
info-preview-hint = { $w } × { $h } pixels, armazenada no arquivo
info-all-exif = Todos os dados EXIF ({ $n } entradas)
info-location = Local
info-set-in-app = Definido no AlbumCraft
info-from-gps = Do GPS da câmera
info-modify = Alterar…
info-modify-hint = Mover o marcador, renomear o local ou removê-lo
info-show-map-hint = Abrir Locais nesta foto
info-no-location = Nenhum local ainda.
info-set-location-hint = Encontrar o local ou clicar nele em um mapa
info-tags = Tags
info-no-tags = Nenhuma tag ainda.
info-remove-tag = Remover esta tag
info-add-tag = Adicionar uma tag…
info-coords = { $lat }° { $ns }, { $lon }° { $ew }
coord-n = N
coord-s = S
coord-e = L
coord-w = O
info-orient-2 = Espelhada
info-orient-3 = De cabeça para baixo
info-orient-4 = Espelhada, de cabeça para baixo
info-orient-5 = Espelhada, girada para a esquerda
info-orient-6 = Girada para a direita (câmera na vertical)
info-orient-7 = Espelhada, girada para a direita
info-orient-8 = Girada para a esquerda (câmera na vertical)
search-remove-hint = Remover da busca
search-hint = Buscar tags, locais, legendas…
search-and = e…
search-clear-hint = Limpar a busca
search-for = Buscar “{ $text }”
search-kind-tags = Tags
music-dialog-title = Música para a apresentação
filter-music = Música
ss-previous = Anterior (←)
ss-play-pause = Reproduzir / pausar (Espaço)
ss-next = Próxima (→)
ss-seconds = Segundos por foto
ss-interval-off = Desativado: as fotos só mudam com → ou com os botões (Configurações, Apresentação)
ss-fade = Transição
ss-motion = Movimento
ss-other-music-hint = Escolher outro arquivo de música
ss-exit-full = Sair da tela cheia (F11 ou Esc)
ss-exit = Sair (Esc)

## Messages of the window (src/app.rs)

save-failed-prefix = Não foi possível salvar a sua biblioteca:
save-failed =
    { $prefix } { $error }
    As suas últimas alterações estão, por enquanto, apenas nesta janela; o AlbumCraft continua tentando.
save-ok-again = A sua biblioteca voltou a ser salva.
lib-unreadable-kept =
    Não foi possível ler o arquivo da sua biblioteca ({ $error }).

    Ele foi guardado em:
    { $backup }

    O AlbumCraft foi iniciado com uma biblioteca vazia. As suas fotos não foram tocadas.
lib-unreadable-nosave =
    Não foi possível ler o arquivo da sua biblioteca ({ $error }).

    O AlbumCraft foi iniciado com uma biblioteca vazia e, desta vez, não salvará nenhuma alteração, para não sobrescrever o arquivo.
dialog-library = Biblioteca
crash-title = O AlbumCraft foi encerrado inesperadamente
crash-text =
    O AlbumCraft parou por causa de um erro na última execução. As alterações feitas nos últimos segundos podem ter se perdido.

    Os detalhes foram gravados em:
    { $log }

    Anexe este arquivo se for relatar o problema.
picasa-imported =
    { $n ->
        [one] Anotações, legendas e edições importadas do arquivo .picasa.ini de { $n } pasta.
       *[other] Anotações, legendas e edições importadas dos arquivos .picasa.ini de { $n } pastas.
    }
copy-no-edits = Esta foto não tem edições para copiar.
copied-edits =
    { $n ->
        [one] { $n } edição copiada. Selecione fotos e cole-a (Ctrl+Shift+V).
       *[other] { $n } edições copiadas. Selecione fotos e cole-as (Ctrl+Shift+V).
    }
paste-first = Copie primeiro as edições de uma foto (Ctrl+Shift+C).
pasted-edits =
    { $n ->
        [one] Edições coladas em { $n } foto.
       *[other] Edições coladas em { $n } fotos.
    }
no-pictures-to-show = Não há fotos para mostrar.
music-error = Sem música: { $error }
choose-folder-dialog = Escolha uma pasta de fotos
added-to-album =
    { $n ->
        [one] { $n } foto adicionada a “{ $name }”.
       *[other] { $n } fotos adicionadas a “{ $name }”.
    }
trash-samples = As fotos de exemplo não podem ser movidas para a lixeira.
trash-no-rejects = Nenhuma foto está rejeitada. Pressione X para rejeitar uma.
task-trash =
    { $n ->
        [one] Movendo { $n } foto para a lixeira
       *[other] Movendo { $n } fotos para a lixeira
    }
task-email =
    { $n ->
        [one] Preparando { $n } foto para um e-mail
       *[other] Preparando { $n } fotos para um e-mail
    }
default-title = Minhas fotos
gallery-folder-dialog = Criar a galeria web em…
task-gallery =
    { $n ->
        [one] Criando a galeria web de { $n } foto
       *[other] Criando a galeria web de { $n } fotos
    }
movie-save-dialog = Salvar o filme como…
filter-mp4 = Vídeo MP4
task-movie = Criando o filme
scan-truncated = Somente as primeiras { $n } fotos são mostradas: remova pastas da biblioteca para ver as demais.
selection-none = Nada selecionado
selection-one = { $name }     { $date }     { $w }x{ $h } pixels     { $size }
selection-several = { $pictures }     { $a }      { $size } no disco
selection-range = { $pictures }     de { $a } a { $b }     { $size } no disco

## Collage and contact sheet (src/collage_view.rs)

cstyle-pile = Pilha de fotos
cstyle-pile-tip = Fotos espalhadas, como cópias jogadas sobre uma mesa
cstyle-mosaic = Mosaico
cstyle-mosaic-tip = Fotos de tamanhos diferentes preenchem a página
cstyle-frame = Foto central
cstyle-frame-tip = Um mosaico ao redor de uma foto grande no centro
cstyle-grid = Grade
cstyle-grid-tip = Linhas e colunas regulares
cstyle-film = Tira de filme
cstyle-film-tip = Fotos em tiras de filme
cstyle-clothesline = Varal
cstyle-clothesline-tip = Fotos instantâneas presas com prendedores em cordas
cstyle-fan = Leque
cstyle-fan-tip = Abertas como um baralho
cstyle-shape = Forma
cstyle-shape-tip = Um coração, um círculo ou uma estrela feitos de fotos
cstyle-blend = Exposição múltipla
cstyle-blend-tip = As fotos aparecem umas através das outras (ideal com 2 a 4)
cstyle-sheet = Folha de contato
cstyle-sheet-tip = Páginas de fotos pequenas com um título e os nomes dos arquivos
shape-heart = Coração
shape-circle = Círculo
shape-star = Estrela
fmt-a4 = A4
fmt-a3 = A3
fmt-letter = Carta (EUA)
fmt-10x15 = 10 x 15 cm
fmt-13x18 = 13 x 18 cm
fmt-20x30 = 20 x 30 cm
fmt-square = Quadrado
fmt-wide = Tela (16:9)
grad-vertical = De cima para baixo
grad-diagonal = Diagonal
grad-radial = A partir do centro
collage-need-sheet = Selecione primeiro as fotos da folha de contato.
collage-need-two = Selecione pelo menos duas fotos para criar uma colagem.
collage-unreadable = Não foi possível ler esta foto.
collage-new-layout = Novo layout
collage-new-layout-hint = Outras posições, rotações e tamanhos
collage-shuffle = Embaralhar
collage-shuffle-hint = Colocar as fotos em ordem aleatória
collage-spacing = Espaço entre as fotos
border-none = Nenhuma
border-white = Branca
border-instant = Instantâneo
collage-frame-text = Texto nas molduras
cap-none = Nenhuma
cap-caption = Legenda
collage-shadows = Sombras
bg-gradient = Gradiente
bg-picture = Imagem
collage-to = para
collage-use-selected = Usar a foto selecionada
collage-use-selected-hint = Clique primeiro em uma foto da colagem
collage-choose-file = Escolher um arquivo…
collage-no-bg = Nenhuma imagem escolhida ainda
collage-bg-dialog = Imagem de fundo
collage-page = Página
collage-size = Tamanho da imagem
collage-px = { $px } pixels
collage-centre = Colocar no centro
collage-put-back = Recolocar
collage-put-back-hint = Desfazer os movimentos desta foto
collage-leave-out = Excluir
collage-leave-out-hint = Remover esta foto da colagem (Delete)
collage-title = Título
collage-no-title = Sem título
collage-no-subtitle = Sem segunda linha
collage-header = Título na primeira página
collage-under = Abaixo de cada foto
collage-date = Data
collage-pictures = Fotos
collage-page-numbers = Números de página
collage-sending = Enviando a página { $page } de { $total } para a impressora…
collage-count-pages =
    { $pictures }, { $pages ->
        [one] { $pages } página
       *[other] { $pages } páginas
    }
collage-no-printer = Nenhuma impressora disponível
collage-save-pdf = Salvar como PDF…
collage-save-picture = Salvar a imagem…
collage-save-picture-hint = Salvar a colagem como imagem JPEG ou PNG
collage-file-sheet = Folha de contato
collage-file-collage = Colagem
collage-save-pdf-title = Salvar como PDF
collage-save-picture-title = Salvar a colagem como imagem
filter-pdf = Documento PDF
filter-jpeg = Imagem JPEG
filter-png = Imagem PNG
collage-saving-sheet = Salvando a folha de contato…
collage-saving = Salvando a colagem…
task-save-collage = Salvando a colagem
collage-printed =
    { $n ->
        [one] { $n } página enviada para { $printer }.
       *[other] { $n } páginas enviadas para { $printer }.
    }
collage-print-failed = A impressão não foi concluída: { $error }
collage-hint-swap = Arraste uma foto sobre outra para trocá-las. Clique em uma foto para selecioná-la (Delete a exclui).
collage-hint-free = Arraste uma foto para movê-la, role sobre ela para girá-la. Clique em uma foto para selecioná-la (Delete a exclui).
collage-page-of = Página { $page } de { $total }

## Places map (src/map_view.rs)

map-search-still = As posições GPS das suas fotos ainda estão sendo procuradas: tente novamente em instantes.
map-none-located =
    { $n ->
        [one] Esta foto ainda não tem local. Selecione-a e clique em { $icon } (Definir o local) na barra de seleção para posicioná-la.
       *[other] Nenhuma destas { $n } fotos tem local ainda. Selecione-as e clique em { $icon } (Definir o local) na barra de seleção para posicioná-las.
    }
map-location-set =
    { $n ->
        [one] Local definido para { $n } foto.
       *[other] Local definido para { $n } fotos.
    }
map-location-removed =
    { $n ->
        [one] Local removido de { $n } foto.
       *[other] Local removido de { $n } fotos.
    }
map-nothing-online = Nada encontrado online.
map-set-location = Definir o local
map-sub-scope =
    { $n } de { $total ->
        [one] { $total } foto
       *[other] { $total } fotos
    } com local
map-sub-indexing = { $n } com local · procurando posições GPS…
map-sub-all =
    { $n ->
        [one] { $n } foto com local
       *[other] { $n } fotos com local
    }
map-title-scope = Locais · { $title }
map-show-whole = Mostrar toda a biblioteca
map-online = Mapa online
map-builtin = Mapa integrado
map-online-tip = O OpenStreetMap está sendo mostrado. Clique para usar somente o mapa integrado (sem conexão com a internet).
map-builtin-tip = O mapa-múndi integrado está sendo mostrado. Clique para mostrar os mapas detalhados do OpenStreetMap (baixa imagens de mapa da internet).
map-show-all = Mostrar tudo
map-show-all-hint = Ajustar o zoom para mostrar todas as fotos no mapa
map-search-online-hint = Buscar um lugar ou um endereço
map-search-builtin-hint = Buscar uma cidade ou coordenadas
map-no-city-builtin = Nenhuma cidade com esse nome na lista integrada.
map-no-city-hint = Nenhuma cidade com esse nome. Tente coordenadas, como 44,84, -0,58.
map-cities = CIDADES
map-cities-builtin = CIDADES (LISTA INTEGRADA)
map-searching = Buscando online…
map-places-osm = LUGARES E ENDEREÇOS (OPENSTREETMAP)
map-search-addresses = Buscar endereços online   (Enter)
map-where = Onde elas foram tiradas?
map-where-help = Busque um lugar acima ou clique no mapa para colocar um marcador. Depois você poderá arrastá-lo.
map-name-hint = Nome do local (opcional)
map-drag-pin = Arraste o marcador para ajustá-lo.
map-drag-pin-offline = Arraste o marcador para ajustá-lo. Os mapas online mostram as ruas.
map-set-n =
    { $n ->
        [one] Definir o local de { $n } foto
       *[other] Definir o local de { $n } fotos
    }
map-remove-location = Remover o local
map-remove-location-hint = Estas fotos ficarão sem local, mesmo que a câmera tenha registrado um (os arquivos não são alterados)
map-close-hint = Fechar (Esc)
map-change-location = Alterar o local…
map-select-in-library = Selecionar na biblioteca
map-select-in-library-hint = Selecionar estas fotos na biblioteca
map-show-hint = Mostrar no mapa
map-preparing = Preparando o mapa…
map-hover = { $pictures } · { $place }
map-credit-builtin = Mapa integrado: Natural Earth · GeoNames
map-problem = { $problem }. O mapa integrado está sendo mostrado.
map-empty-indexing = Procurando as posições GPS das suas fotos…
map-empty =
    Nenhuma foto com local ainda.

    As fotos de celulares e de câmeras com GPS aparecem aqui sozinhas. Para posicionar as demais, selecione-as e clique em { $icon } (Definir o local) na barra de seleção.

## Import (src/import_view.rs)

import-subtitle = Copiar fotos para a sua biblioteca
import-from = De
import-choose = Escolher…
import-refresh-hint = Procurar novamente cartões e discos, e reler a pasta
import-hide-dups = Ocultar as { $n } que já estão na biblioteca
import-dups-hint = Fotos com o mesmo conteúdo de uma foto da sua biblioteca, mesmo com outro nome
import-uncheck-all = Desmarcar todas
import-check-all = Marcar todas
import-copy-into = Copiar para
import-another-folder = Outra pasta…
import-name-hint = Nome da nova pasta, p. ex. Verão na Espanha
import-checked = { $chosen } de { $shown } marcadas. Nada é sobrescrito; os originais não são alterados.
import-go =
    { $n ->
        [one] Importar { $n } foto
       *[other] Importar { $n } fotos
    }
import-dest-dialog = Colocar a nova pasta em…
import-from-dialog = Importar fotos de…
import-pick-dest = Escolha onde colocar a nova pasta.
import-running = Uma importação ainda está em andamento: aguarde o término (veja a barra de status).
import-started =
    { $n ->
        [one] Importando { $n } foto para { $path }…
       *[other] Importando { $n } fotos para { $path }…
    }
import-listing-in = Procurando fotos em { $from }…
import-none-in = Nenhuma foto em { $path }.
import-choose-another = Escolher outra pasta…
import-heading = Importar de um cartão de memória, de uma câmera ou de uma pasta
import-copy-note = As fotos são copiadas para uma nova pasta da sua biblioteca. Os originais não são alterados.
import-no-cards = Nenhum cartão de memória nem câmera encontrado. Conecte um e clique em { $icon } acima.
import-instead = Para mostrar fotos onde elas já estão, sem copiá-las:
import-add-instead = adicione uma pasta à biblioteca
import-all-dups =
    { $n ->
        [one] A foto já está na sua biblioteca.
       *[other] Todas as { $n } fotos já estão na sua biblioteca.
    }
import-show-anyway = Mostrá-las mesmo assim
import-truncated = Somente as primeiras { $n } fotos são listadas.
import-group-hint = Marcar ou desmarcar todo este grupo
import-in-library = Na biblioteca
import-click-preview = Clique em uma foto para vê-la aqui
import-this = Importar esta foto
import-this-hint = Espaço
import-already = já está na sua biblioteca

## Print (src/print_view.rs)

print-select-first = Selecione primeiro as fotos que você quer imprimir.
print-page-layout = Layout da página
print-layout-full = Página inteira
print-shrink = Reduzir para ajustar
print-shrink-hint = A foto inteira é impressa, com branco onde as proporções diferem
print-crop = Recortar para ajustar
print-crop-hint = A foto preenche o formato de impressão; as bordas excedentes são cortadas
print-options = Opções de borda e de texto
print-border = Borda
print-no-border = Sem borda
print-thin-line = Linha preta fina
print-white-border = Borda branca
print-text-under = Texto abaixo de cada foto
print-no-text = Sem texto
print-printer = Impressora
print-no-printer-found = Nenhuma impressora encontrada
print-setup = Configurações da impressora…
print-setup-hint = Tamanho do papel, orientação e configurações próprias da impressora
print-paper = Papel
print-default = Padrão da impressora
print-no-answer = Nenhuma impressora respondeu: uma página A4 está sendo mostrada
print-paper-describe = { $dpi } dpi, { $w } x { $h } pol. ({ $orientation })
print-copies = Cópias por foto
print-copies-n =
    { $n ->
        [one] { $n } cópia
       *[other] { $n } cópias
    }
print-counts =
    { $pictures }, { $prints ->
        [one] { $prints } impressão
       *[other] { $prints } impressões
    }
print-cancel = Cancelar  { $icon }
print-go = Imprimir  { $icon }
print-review = Revisar
print-review-hint = Verificar se cada foto tem pixels suficientes para o tamanho de impressão
print-none-left = Não restou nenhuma foto para imprimir.
print-job-printed =
    { $n ->
        [one] { $n } página enviada para { $printer }.
       *[other] { $n } páginas enviadas para { $printer }.
    }
print-preview = Visualização
print-ready = Todas as suas fotos estão prontas para impressão.
print-too-small = Algumas das suas fotos são pequenas demais para serem impressas com boa qualidade.  Você pode remover essas fotos, imprimi-las mesmo assim, ou cancelar e mudar o tamanho de impressão.
print-quality-best = Qualidade ótima ({ $ppi } pixels/polegada)
print-quality-good = Boa qualidade ({ $ppi } pixels/polegada)
print-quality-bad = Qualidade ruim ({ $ppi } pixels/polegada)

## Results of background work (src/backend.rs)

backend-no-data-dir = Nenhuma pasta para os dados do aplicativo foi encontrada.
backend-cache-fail = Não foi possível esvaziar o cache ({ $error })
fail-export = Não foi possível exportar: erro inesperado.
fail-trash = Não foi possível mover as fotos para a lixeira: erro inesperado.
fail-email = Não foi possível preparar o e-mail: erro inesperado.
fail-gallery = Não foi possível criar a galeria web: erro inesperado.
fail-movie = Não foi possível criar o filme: erro inesperado
fail-collage = Não foi possível salvar a colagem: erro inesperado.
export-nothing = Nada foi exportado: { $error }.
export-result =
    { $ok } de { $total ->
        [one] { $total } foto exportada
       *[other] { $total } fotos exportadas
    } para { $path }
export-stopped = A exportação foi interrompida antes do fim.
export-failed-list = Não exportadas:
export-more = … e mais { $n }
gallery-ready =
    A galeria web está pronta:
    { $path }
gallery-left-out =
    { $n ->
        [one] { $n } foto não pôde ser lida e foi deixada de fora.
       *[other] { $n } fotos não puderam ser lidas e foram deixadas de fora.
    }
gallery-stopped = A galeria web não foi criada: interrupção solicitada.
gallery-failed = Não foi possível criar a galeria web: { $error }
movie-ready =
    O filme está pronto:
    { $path }
movie-stopped = O filme não foi criado: interrupção solicitada.
movie-failed = Não foi possível criar o filme: { $error }
collage-saved-many =
    { $n } imagens salvas, a primeira é:
    { $path }
collage-saved =
    Salvo:
    { $path }
collage-save-failed = Não foi possível salvar a colagem: { $error }
import-unexpected = A importação foi interrompida inesperadamente.
import-summary =
    { $n ->
        [one] { $n } foto importada para { $path }.
       *[other] { $n } fotos importadas para { $path }.
    }
import-already-there =
    { $n ->
        [one] { $n } já estava lá.
       *[other] { $n } já estavam lá.
    }
import-was-stopped = A importação foi interrompida antes do fim.
import-failed-count = { $n } não puderam ser copiadas:

## Dates, numbers and sizes (applied to the core crate by src/i18n.rs)

locale-months-short = jan., fev., mar., abr., mai., jun., jul., ago., set., out., nov., dez.
locale-months-long = janeiro, fevereiro, março, abril, maio, junho, julho, agosto, setembro, outubro, novembro, dezembro
locale-date = { $day } { $month } { $year }
locale-decimal = ,
locale-group = .
locale-unit-kb = KB
locale-unit-mb = MB
locale-unit-gb = GB

## Messages made by the core crate (src/msg.rs in the core; each has its English at the place that makes it)

import-name-empty = Digite um nome para a nova pasta.
import-name-chars = O nome de uma pasta não pode conter / \ : * ? " < > |
import-name-long = Este nome de pasta é longo demais.
import-drive = Disco { $letter }:
import-mkdir-fail = Não foi possível criar a pasta { $path }: { $error }
import-too-many = arquivos demais têm este nome
import-appeared = um arquivo com este nome apareceu durante a cópia
export-logo-bad = o logotipo { $path } não pode ser lido ({ $error })
export-unreadable = não foi possível ler a foto
gallery-too-many = galerias demais têm este nome aqui
gallery-none-read = nenhuma das fotos pôde ser lida
movie-no-pictures = nenhuma foto
movie-ffmpeg-start = Não foi possível iniciar o FFmpeg: { $error }
movie-ffmpeg-input = O FFmpeg não tem entrada
movie-ffmpeg-stopped = O FFmpeg parou ({ $error })
movie-ffmpeg-failed =
    O FFmpeg falhou:
    { $tail }
root-already = Esta pasta já está na biblioteca.
root-inside = Esta pasta já está na biblioteca, dentro de { $outer }.
root-excluded = That folder is inside { $folder }, which is left out of the library.
exclude-is-root = That is a library folder: remove it instead.
exclude-outside = That folder is not inside a library folder.
exclude-already = That folder is already left out.
online-tile-bad = não foi possível ler um bloco de mapa: { $error }
online-map-down = Mapa online inacessível ({ $error })
online-search-down = A busca online está inacessível ({ $error }).
online-search-odd = Resposta inesperada da busca online: { $error }
collage-nothing = não há nada para salvar
collage-extension = salve como .pdf, .png ou .jpg
collage-page-fail = não foi possível desenhar uma página
geo-near = Perto de { $name }
mail-no-pictures = nenhuma foto para anexar
mail-copies-fail = Não foi possível fazer as cópias para o e-mail: { $error }
mail-stopped = O e-mail não foi preparado: interrupção solicitada.
mail-none-read = Não foi possível fazer as cópias para o e-mail: nenhuma das fotos pôde ser lida.
mail-left-out =
    { $n ->
        [one] { $n } foto não pôde ser lida e foi deixada de fora.
       *[other] { $n } fotos não puderam ser lidas e foram deixadas de fora.
    }
mail-opened =
    Uma nova mensagem com { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    } foi aberta no seu programa de e-mail.{ $left_out }
mail-asked =
    Foi pedida ao seu programa de e-mail uma nova mensagem com { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    }. Se elas não estiverem anexadas, anexe-as a partir de:
    { $path }{ $left_out }
mail-open-fail =
    Não foi possível abrir o seu programa de e-mail ({ $error }).
    As pequenas cópias de { $n ->
        [one] { $n } foto
       *[other] { $n } fotos
    } estão em:
    { $path }
    Anexe-as à sua mensagem.{ $left_out }
mail-too-many = fotos demais
mail-no-mapi = nenhum programa de e-mail (mapi32.dll: { $error })
mail-no-attach = o programa de e-mail não aceita anexos (sem MAPISendMailW)
mail-refused = o programa de e-mail recusou a mensagem (erro MAPI { $code })
mail-open-mail = não foi possível abrir o Mail: { $error }
mail-open-mail-short = não foi possível abrir o Mail
mail-no-xdg = o xdg-email (do xdg-utils) não está instalado
mail-xdg-fail = não foi possível executar o xdg-email: { $error }
gallery-html-lang = pt-BR
gallery-count = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
gallery-close = Fechar
gallery-prev = Anterior
gallery-next = Próxima
sheet-count = { $n ->
    [one] { $n } foto
   *[other] { $n } fotos
}
