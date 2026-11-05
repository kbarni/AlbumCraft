//! The built-in world map, used when online maps are off and under the online tiles while they
//! load: land, large lakes and country borders from Natural Earth (1:50 million, public domain),
//! stored compressed in `assets/geo/basemap.bin`.
//!
//! The shapes are projected to world coordinates (see `geo::to_world`) and cut into triangles once,
//! on a background thread; each frame only the shapes in view are moved to the screen.

use crate::map_view::Viewport;
use eframe::egui::{self, Color32, Pos2, Shape, Stroke};
use std::sync::OnceLock;

static DATA: &[u8] = include_bytes!("../assets/geo/basemap.bin");
const MAGIC: &[u8] = b"ACMAP1";
/// Coordinates are stored in units of 1/10,000 degree (about 11 m).
const UNIT: f64 = 1e-4;

struct Poly {
    /// [min x, min y, max x, max y] in world coordinates
    bbox: [f32; 4],
    verts: Vec<[f32; 2]>,
    tris: Vec<u32>,
}

struct Line {
    bbox: [f32; 4],
    pts: Vec<[f32; 2]>,
}

pub struct Basemap {
    land: Vec<Poly>,
    lakes: Vec<Poly>,
    borders: Vec<Line>,
}

static MAP: OnceLock<Basemap> = OnceLock::new();

/// The map if it is ready. The first call starts preparing it on another thread and asks for a
/// repaint when it is done.
pub fn get(ctx: &egui::Context) -> Option<&'static Basemap> {
    static STARTED: std::sync::Once = std::sync::Once::new();
    if let Some(m) = MAP.get() {
        return Some(m);
    }
    let ctx = ctx.clone();
    STARTED.call_once(move || {
        std::thread::spawn(move || {
            let map = std::panic::catch_unwind(|| decode(DATA)).ok().flatten().unwrap_or(Basemap { land: vec![], lakes: vec![], borders: vec![] });
            let _ = MAP.set(map);
            ctx.request_repaint();
        });
    });
    None
}

// ---------------------------------------------------------------------------------------------
// reading the data

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn varint(&mut self) -> Option<u64> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = *self.data.get(self.pos)?;
            self.pos += 1;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
        None
    }

    fn signed(&mut self) -> Option<i64> {
        let v = self.varint()?;
        Some((v >> 1) as i64 ^ -((v & 1) as i64))
    }

    /// A count, capped so that damaged data can't ask for a huge allocation.
    fn count(&mut self) -> Option<usize> {
        usize::try_from(self.varint()?).ok().filter(|&n| n <= 1 << 22)
    }

    /// One ring of points, in world coordinates.
    fn ring(&mut self) -> Option<Vec<[f32; 2]>> {
        let n = self.count()?;
        let mut pts = Vec::with_capacity(n);
        let (mut lon, mut lat) = (0i64, 0i64);
        for _ in 0..n {
            lon = lon.checked_add(self.signed()?)?;
            lat = lat.checked_add(self.signed()?)?;
            let at = crate::geo::LatLon::new(lat as f64 * UNIT, lon as f64 * UNIT)?;
            let [x, y] = crate::geo::to_world(at);
            pts.push([x as f32, y as f32]);
        }
        Some(pts)
    }
}

fn bbox(pts: &[[f32; 2]]) -> [f32; 4] {
    pts.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
}

fn polygons(r: &mut Reader) -> Option<Vec<Poly>> {
    let n = r.count()?;
    let mut out = Vec::with_capacity(n);
    let mut earcut = earcut::Earcut::new();
    for _ in 0..n {
        let rings = r.count()?;
        let mut verts: Vec<[f32; 2]> = vec![];
        let mut holes: Vec<u32> = vec![];
        for k in 0..rings {
            let ring = r.ring()?;
            if k > 0 {
                holes.push(u32::try_from(verts.len()).ok()?);
            }
            verts.extend(ring);
        }
        if verts.len() < 3 {
            continue;
        }
        let mut tris: Vec<u32> = vec![];
        earcut.earcut(verts.iter().map(|p| [p[0] as f64, p[1] as f64]), &holes, &mut tris);
        out.push(Poly { bbox: bbox(&verts), verts, tris });
    }
    Some(out)
}

fn decode(packed: &[u8]) -> Option<Basemap> {
    let data = miniz_oxide::inflate::decompress_to_vec_with_limit(packed, 64 << 20).ok()?;
    let mut r = Reader { data: data.strip_prefix(MAGIC)?, pos: 0 };
    let land = polygons(&mut r)?;
    let lakes = polygons(&mut r)?;
    let n = r.count()?;
    let mut borders = Vec::with_capacity(n);
    for _ in 0..n {
        let pts = r.ring()?;
        if pts.len() >= 2 {
            borders.push(Line { bbox: bbox(&pts), pts });
        }
    }
    Some(Basemap { land, lakes, borders })
}

// ---------------------------------------------------------------------------------------------
// drawing

impl Basemap {
    /// Paint the map into the viewport: sea, land, lakes, then borders (when zoomed in enough).
    pub fn paint(&self, painter: &egui::Painter, view: &Viewport) {
        let c = crate::theme::t();
        painter.rect_filled(view.rect, 0, c.map_water);
        for k in view.world_copies() {
            let fill = |polys: &[Poly], color: Color32| {
                let mut mesh = egui::Mesh::default();
                for p in polys.iter().filter(|p| view.sees(p.bbox, k)) {
                    let base = mesh.vertices.len() as u32;
                    for v in &p.verts {
                        mesh.colored_vertex(view.screen_of_f32(*v, k), color);
                    }
                    mesh.indices.extend(p.tris.iter().map(|i| base + i));
                }
                if !mesh.is_empty() {
                    painter.add(Shape::mesh(mesh));
                }
            };
            fill(&self.land, c.map_land);
            fill(&self.lakes, c.map_water);
            if view.zoom() >= 2.0 {
                let stroke = Stroke::new(if view.zoom() >= 5.0 { 1.2 } else { 0.8 }, c.map_border);
                for l in self.borders.iter().filter(|l| view.sees(l.bbox, k)) {
                    let pts: Vec<Pos2> = l.pts.iter().map(|p| view.screen_of_f32(*p, k)).collect();
                    painter.add(Shape::line(pts, stroke));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_map_decodes() {
        let m = decode(DATA).expect("the built-in map decodes");
        assert!(m.land.len() > 1000 && m.lakes.len() > 100 && m.borders.len() > 100);
        for p in m.land.iter().chain(&m.lakes) {
            assert_eq!(p.tris.len() % 3, 0);
            assert!(p.tris.iter().all(|&i| (i as usize) < p.verts.len()));
        }
        // most land is filled (a failed triangulation gives no triangles)
        let filled = m.land.iter().filter(|p| !p.tris.is_empty()).count();
        assert!(filled * 100 / m.land.len() > 95, "{filled} of {}", m.land.len());
    }

    #[test]
    fn damaged_data_is_refused_not_a_crash() {
        assert!(decode(b"").is_none());
        assert!(decode(&miniz_oxide::deflate::compress_to_vec(b"ACMAP1\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff", 6)).is_none());
        assert!(decode(&miniz_oxide::deflate::compress_to_vec(b"ACMAP1\x05\x01", 6)).is_none());
        let mut full = miniz_oxide::inflate::decompress_to_vec(DATA).unwrap();
        full.truncate(full.len() / 2);
        assert!(decode(&miniz_oxide::deflate::compress_to_vec(&full, 6)).is_none());
    }

    /// Builds `assets/geo/basemap.bin` from Natural Earth GeoJSON files (ne_50m_land,
    /// ne_50m_lakes, ne_50m_admin_0_boundary_lines_land) in the folder named by ALBUMCRAFT_GEO_SRC.
    #[test]
    #[ignore]
    fn build_basemap_data() {
        use serde_json::Value;
        let src = std::path::PathBuf::from(std::env::var("ALBUMCRAFT_GEO_SRC").expect("set ALBUMCRAFT_GEO_SRC"));
        let read = |n: &str| -> Value { serde_json::from_slice(&std::fs::read(src.join(n)).unwrap()).unwrap() };
        fn varint(out: &mut Vec<u8>, mut v: u64) {
            while v >= 0x80 {
                out.push((v as u8) | 0x80);
                v >>= 7;
            }
            out.push(v as u8);
        }
        fn signed(out: &mut Vec<u8>, v: i64) {
            varint(out, ((v << 1) ^ (v >> 63)) as u64);
        }
        fn ring(out: &mut Vec<u8>, pts: &Value) -> usize {
            let pts: Vec<(i64, i64)> = pts
                .as_array()
                .unwrap()
                .iter()
                .map(|p| ((p[0].as_f64().unwrap() / UNIT).round() as i64, (p[1].as_f64().unwrap() / UNIT).round() as i64))
                .collect();
            // drop repeated points (after rounding) and the closing copy of the first one
            let mut kept: Vec<(i64, i64)> = vec![];
            for p in pts {
                if kept.last() != Some(&p) {
                    kept.push(p);
                }
            }
            if kept.len() > 1 && kept.first() == kept.last() {
                kept.pop();
            }
            varint(out, kept.len() as u64);
            let (mut lon, mut lat) = (0, 0);
            for (x, y) in &kept {
                signed(out, x - lon);
                signed(out, y - lat);
                (lon, lat) = (*x, *y);
            }
            kept.len()
        }
        let polygons = |out: &mut Vec<u8>, file: &str| -> usize {
            let mut polys: Vec<Value> = vec![];
            for f in read(file)["features"].as_array().unwrap() {
                let g = &f["geometry"];
                match g["type"].as_str() {
                    Some("Polygon") => polys.push(g["coordinates"].clone()),
                    Some("MultiPolygon") => polys.extend(g["coordinates"].as_array().unwrap().iter().cloned()),
                    _ => {}
                }
            }
            varint(out, polys.len() as u64);
            let mut n = 0;
            for p in &polys {
                let rings = p.as_array().unwrap();
                varint(out, rings.len() as u64);
                for r in rings {
                    n += ring(out, r);
                }
            }
            n
        };
        let mut out = MAGIC.to_vec();
        let land = polygons(&mut out, "ne_50m_land.geojson");
        let lakes = polygons(&mut out, "ne_50m_lakes.geojson");
        let mut lines: Vec<Value> = vec![];
        for f in read("ne_50m_admin_0_boundary_lines_land.geojson")["features"].as_array().unwrap() {
            let g = &f["geometry"];
            match g["type"].as_str() {
                Some("LineString") => lines.push(g["coordinates"].clone()),
                Some("MultiLineString") => lines.extend(g["coordinates"].as_array().unwrap().iter().cloned()),
                _ => {}
            }
        }
        varint(&mut out, lines.len() as u64);
        let borders: usize = lines.iter().map(|l| ring(&mut out, l)).sum();
        let packed = miniz_oxide::deflate::compress_to_vec(&out, 10);
        let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/geo/basemap.bin");
        std::fs::write(&dest, &packed).unwrap();
        eprintln!("points: land {land}, lakes {lakes}, borders {borders}; {} bytes -> {} bytes", out.len(), packed.len());
    }
}
