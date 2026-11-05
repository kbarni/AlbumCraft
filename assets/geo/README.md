# Built-in map data

Used by the Places map and the place search (`src/basemap.rs`, `src/geo.rs`). Both files are
compressed (deflate) and embedded in the program.

| File | What | Source | License |
|---|---|---|---|
| `basemap.bin` | Land, large lakes and country borders, 1:50 million | [Natural Earth](https://www.naturalearthdata.com/) (`ne_50m_land`, `ne_50m_lakes`, `ne_50m_admin_0_boundary_lines_land`) | Public domain |
| `cities.bin` | 34,000 places of 15,000 people or more: name, other names of the larger ones, region, country, position, population | [GeoNames](https://www.geonames.org/) (`cities15000`, `countryInfo`, `admin1CodesASCII`) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |

AlbumCraft shows the credit "Natural Earth · GeoNames" on the built-in map and in About.

## Rebuilding

Download the source files into one folder (the Natural Earth GeoJSON files from
`github.com/nvkelso/natural-earth-vector/tree/master/geojson`, the GeoNames files from
`download.geonames.org/export/dump/`, with `cities15000.zip` unpacked), then:

```sh
ALBUMCRAFT_GEO_SRC=<folder> cargo test build_basemap_data -- --ignored --nocapture
ALBUMCRAFT_GEO_SRC=<folder> cargo test build_city_data -- --ignored --nocapture
```

Online maps (only when the user turns them on) come from OpenStreetMap at run time and are not
part of this folder: © OpenStreetMap contributors, ODbL.
