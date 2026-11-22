//! Tuning the face grouping on a real library whose faces were marked by hand in its
//! `.picasa.ini` files (`faces=` with contact ids): how many marked faces are found, and how
//! well the groups follow the marks, for several settings.
//!
//! ```text
//! ALBUMCRAFT_FACES_PHOTOS=<folder> ALBUMCRAFT_FACES_PACK=<folder with sface-1.onnx>
//! ALBUMCRAFT_FACES_TABLE=<folder for the analysis, kept between runs>
//! [ALBUMCRAFT_FACES_OUT=<folder for contact sheets of the groups>]
//! cargo test --release -p albumcraft-core real_face_groups -- --ignored --nocapture
//! ```
//!
//! People are printed as numbers only. The table and the sheets hold the faces of the people in
//! those pictures: keep them to yourself and delete them after.

use crate::ai::faces::{FaceAnalyser, FaceRecord, table};
use crate::ai::store::{Table, picture_id};
use crate::face_groups::{Constraints, Face, FaceId, Groups, Params, proposals, regroup};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "jpg" | "jpeg")) {
            out.push(p);
        }
    }
}

fn stamp(p: &Path) -> Option<(i64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((t, m.len()))
}

/// left / top / right / bottom of a box written in the stored picture, in the upright one.
fn upright(r: [f32; 4], orientation: u8) -> [f32; 4] {
    let [l, t, rr, b] = r;
    match orientation {
        3 => [1.0 - rr, 1.0 - b, 1.0 - l, 1.0 - t],
        6 => [1.0 - b, l, 1.0 - t, rr],
        8 => [t, 1.0 - rr, b, 1.0 - l],
        _ => r,
    }
}

fn iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let i = w * h;
    let u = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - i;
    if u > 0.0 { i / u } else { 0.0 }
}

struct Scores {
    precision: f32,
    recall: f32,
    wrong: usize,
    labelled: usize,
    groups: usize,
    grouped: f32,
    people: Vec<(u64, usize, usize, f32)>,
    proposals: usize,
    good_proposals: usize,
}

fn score(groups: &Groups, labels: &HashMap<FaceId, u64>, faces: &[Face], props: &[(u32, u32, f32)]) -> Scores {
    let lab: Vec<(&FaceId, &u64)> = labels.iter().collect();
    let (mut tp, mut fp, mut fn_) = (0u64, 0u64, 0u64);
    for (x, (fa, la)) in lab.iter().enumerate() {
        for (fb, lb) in &lab[x + 1..] {
            let same_group = matches!((groups.of.get(fa), groups.of.get(fb)), (Some(a), Some(b)) if a == b);
            match (la == lb, same_group) {
                (true, true) => tp += 1,
                (false, true) => fp += 1,
                (true, false) => fn_ += 1,
                _ => {}
            }
        }
    }
    // in each group, the labelled faces that are not of its main person
    let mut by_group: HashMap<u32, HashMap<u64, usize>> = HashMap::new();
    for (f, l) in labels {
        if let Some(g) = groups.of.get(f) {
            *by_group.entry(*g).or_default().entry(*l).or_default() += 1;
        }
    }
    let wrong: usize = by_group.values().map(|c| c.values().sum::<usize>() - c.values().max().copied().unwrap_or(0)).sum();
    let main: HashMap<u32, u64> = by_group.iter().map(|(g, c)| (*g, *c.iter().max_by_key(|(_, n)| **n).unwrap().0)).collect();
    // each person with 10 marked faces or more: in how many groups, the share in the largest
    let mut per: HashMap<u64, Vec<FaceId>> = HashMap::new();
    for (f, l) in labels {
        per.entry(*l).or_default().push(*f);
    }
    let mut people: Vec<(u64, usize, usize, f32)> = per
        .iter()
        .filter(|(_, f)| f.len() >= 10)
        .map(|(l, f)| {
            let mut count: HashMap<u32, usize> = HashMap::new();
            for x in f {
                if let Some(g) = groups.of.get(x) {
                    *count.entry(*g).or_default() += 1;
                }
            }
            let largest = count.values().max().copied().unwrap_or(0);
            (*l, f.len(), count.len(), largest as f32 / f.len() as f32)
        })
        .collect();
    people.sort_by_key(|p| std::cmp::Reverse(p.1));
    let sizes = groups.members();
    let good_proposals = props.iter().filter(|(a, b, _)| matches!((main.get(a), main.get(b)), (Some(x), Some(y)) if x == y)).count();
    Scores {
        precision: tp as f32 / (tp + fp).max(1) as f32,
        recall: tp as f32 / (tp + fn_).max(1) as f32,
        wrong,
        labelled: labels.len(),
        groups: sizes.values().filter(|m| m.len() >= 3).count(),
        grouped: groups.of.len() as f32 / faces.len().max(1) as f32,
        people,
        proposals: props.len(),
        good_proposals,
    }
}

/// Contact sheets of the largest groups: one strip of faces per group, cut from the pictures.
fn sheets(out: &Path, groups: &Groups, keys: &HashMap<u128, String>, records: &HashMap<FaceId, FaceRecord>) {
    std::fs::create_dir_all(out).unwrap();
    let mut list: Vec<(u32, Vec<FaceId>)> = groups.members().into_iter().collect();
    list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    list.truncate(40);
    const CELL: u32 = 72;
    const PER: usize = 24;
    list.par_iter().enumerate().for_each(|(rank, (g, faces))| {
        let shown: Vec<&FaceId> = faces.iter().step_by((faces.len() / PER).max(1)).take(PER).collect();
        let mut sheet = image::RgbImage::from_pixel(CELL * PER as u32, CELL, image::Rgb([40, 40, 40]));
        for (n, f) in shown.iter().enumerate() {
            let (Some(key), Some(r)) = (keys.get(&f.0), records.get(f)) else { continue };
            let Some(pic) = crate::imaging::load_source(key, crate::ai::faces::INPUT_DIM) else { continue };
            let (w, h) = (pic.width() as f32, pic.height() as f32);
            // the box with a margin, square
            let side = (r.rect[2] * w).max(r.rect[3] * h) * 1.3;
            let (cx, cy) = ((r.rect[0] + r.rect[2] / 2.0) * w, (r.rect[1] + r.rect[3] / 2.0) * h);
            let x = (cx - side / 2.0).clamp(0.0, (w - 1.0).max(0.0)) as u32;
            let y = (cy - side / 2.0).clamp(0.0, (h - 1.0).max(0.0)) as u32;
            let s = (side as u32).max(1).min(pic.width() - x).min(pic.height() - y);
            let crop = image::imageops::crop_imm(&pic, x, y, s, s).to_image();
            let small = image::imageops::resize(&crop, CELL, CELL, image::imageops::FilterType::Triangle);
            for (px, py, p) in small.enumerate_pixels() {
                sheet.put_pixel(n as u32 * CELL + px, py, image::Rgb([p[0], p[1], p[2]]));
            }
        }
        let _ = sheet.save(out.join(format!("group-{rank:02}-{g}-{}faces.jpg", faces.len())));
    });
}

#[test]
#[ignore]
fn real_face_groups() {
    let photos = PathBuf::from(std::env::var("ALBUMCRAFT_FACES_PHOTOS").expect("ALBUMCRAFT_FACES_PHOTOS"));
    let pack_dir = PathBuf::from(std::env::var("ALBUMCRAFT_FACES_PACK").expect("ALBUMCRAFT_FACES_PACK"));
    let table_dir = PathBuf::from(std::env::var("ALBUMCRAFT_FACES_TABLE").expect("ALBUMCRAFT_FACES_TABLE"));
    std::fs::create_dir_all(&table_dir).unwrap();
    let pack = crate::faces::pack().unwrap();
    let a = FaceAnalyser::new(Some((pack, pack_dir.as_path())));
    let spec = table(pack.faces.as_ref());
    let mut t = Table::open(&table_dir, &spec).unwrap();

    // 1. the pictures, analysed once (kept in the table between runs)
    let mut files = vec![];
    walk(&photos, &mut files);
    let files: Vec<(String, (i64, u64))> = files.into_iter().filter_map(|p| Some((p.to_string_lossy().into_owned(), stamp(&p)?))).collect();
    let todo: Vec<&(String, (i64, u64))> = files.iter().filter(|(k, s)| !t.has(k, *s)).collect();
    println!("{} pictures, {} to analyse", files.len(), todo.len());
    let started = std::time::Instant::now();
    for (n, chunk) in todo.chunks(256).enumerate() {
        type Found = (String, (i64, u64), Vec<Vec<u8>>);
        let found: Vec<Found> = chunk
            .par_iter()
            .map(|(key, s)| {
                let recs = crate::imaging::load_source(key, crate::ai::faces::INPUT_DIM).map(|pic| {
                    let full = || crate::imaging::load_source(key, 4096);
                    a.faces(&pic, &full).map(|f| f.iter().map(FaceRecord::to_bytes).collect()).unwrap_or_default()
                });
                (key.clone(), *s, recs.unwrap_or_default())
            })
            .collect();
        for (k, s, r) in found {
            t.put(&k, s, &r).unwrap();
        }
        println!("  analysed {} of {} ({:.0} s)", ((n + 1) * 256).min(todo.len()), todo.len(), started.elapsed().as_secs_f32());
    }

    // 2. the faces, in the pictures' order
    let mut faces: Vec<Face> = vec![];
    let mut records: HashMap<FaceId, FaceRecord> = HashMap::new();
    let mut keys: HashMap<u128, String> = HashMap::new();
    let p0 = Params::default();
    let mut all_faces = 0;
    for (key, s) in &files {
        let pid = picture_id(key);
        keys.insert(pid, key.clone());
        for (i, r) in t.get(key, *s).unwrap_or_default().into_iter().enumerate() {
            let Some(r) = FaceRecord::from_bytes(r) else { continue };
            all_faces += 1;
            let id = (pid, i as u16);
            if let Some(v) = &r.vector {
                faces.push(Face { id, vector: v.clone(), good: p0.is_good(&r) });
            }
            records.insert(id, r);
        }
    }
    println!("{all_faces} faces, {} with a vector, {} good enough to start a group", faces.len(), faces.iter().filter(|f| f.good).count());

    // 3. the marked faces, matched to the faces found
    let mut labels: HashMap<FaceId, u64> = HashMap::new();
    let (mut marked, mut matched, mut matched_turned, mut turned_marks) = (0, 0, 0, 0);
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    for (k, _) in &files {
        dirs.insert(Path::new(k).parent().unwrap().to_path_buf());
    }
    let mut dirs: Vec<PathBuf> = dirs.into_iter().collect();
    dirs.sort();
    for d in &dirs {
        let Some(ini) = crate::library::read_picasa_ini(d) else { continue };
        let names: HashMap<String, String> = std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path().to_string_lossy().into_owned())).collect();
        for (file, rect, id) in &ini.faces {
            let Some(key) = names.get(file) else { continue };
            marked += 1;
            let pid = picture_id(key);
            let orientation = crate::imaging::jpeg_head(key).map_or(1, |h| h.orientation);
            if orientation != 1 {
                turned_marks += 1;
            }
            let found: Vec<(FaceId, [f32; 4])> = records.iter().filter(|(f, _)| f.0 == pid).map(|(f, r)| (*f, [r.rect[0], r.rect[1], r.rect[0] + r.rect[2], r.rect[1] + r.rect[3]])).collect();
            // the box as written, and turned the way the picture is shown
            let best = |r: [f32; 4]| found.iter().map(|(f, b)| (*f, iou(r, *b))).filter(|(_, s)| *s >= 0.3).max_by(|a, b| a.1.total_cmp(&b.1));
            let (plain, turned) = (best(*rect), if orientation != 1 { best(upright(*rect, orientation)) } else { None });
            if turned.is_some() && plain.is_none() {
                matched_turned += 1;
            }
            if let Some((f, _)) = turned.or(plain) {
                matched += 1;
                if records.get(&f).is_some_and(|r| r.vector.is_some()) {
                    labels.insert(f, *id);
                }
            }
        }
    }
    println!(
        "{marked} marked faces: {matched} found ({:.0} %), {matched_turned} of them only once the box is turned like the picture ({turned_marks} marks on turned pictures); {} with a vector, {} people",
        100.0 * matched as f32 / marked.max(1) as f32,
        labels.len(),
        labels.values().collect::<HashSet<_>>().len()
    );

    // 4. the settings tried
    let mut tried: Vec<(String, Params)> = vec![];
    for join in [0.40, 0.45, 0.50] {
        for seed in [0.45, 0.50, 0.55] {
            tried.push((format!("join {join:.2} seed {seed:.2}"), Params { join, seed, ..Params::default() }));
        }
    }
    for min_score in [0.6, 0.7, 0.75, 0.85] {
        tried.push((format!("default, min score {min_score:.2}"), Params { min_score, ..Params::default() }));
    }
    tried.push(("default, no non-face check".into(), Params { max_nonface: 9.0, ..Params::default() }));
    for good_yaw in [0.15, 0.2] {
        tried.push((format!("default, frontal {good_yaw:.2} to start"), Params { good_yaw, ..Params::default() }));
    }
    tried.push(("default, sharper and larger to start".into(), Params { good_eyes: 25.0, good_sharpness: 60.0, ..Params::default() }));
    tried.push(("default, support 1".into(), Params { support: 1, ..Params::default() }));
    tried.push(("default, any face starts a group".into(), Params { good_score: 0.0, good_eyes: 0.0, good_sharpness: 0.0, good_yaw: 9.0, ..Params::default() }));
    println!("{:<36} {:>9} {:>7} {:>6} {:>7} {:>5} {:>8} {:>10} {:>6}", "settings", "precision", "recall", "wrong", "groups", "junk", "grouped", "proposals", "ms");
    let mut best_groups = None;
    let nonface = crate::face_groups::nonface_direction(records.values());
    for (name, p) in &tried {
        let fs: Vec<Face> = faces.iter().filter(|f| records.get(&f.id).is_some_and(|r| p.is_grouped(r, nonface.as_deref()))).map(|f| Face { good: records.get(&f.id).is_some_and(|r| p.is_good(r)), ..f.clone() }).collect();
        let t0 = std::time::Instant::now();
        let g = regroup(&fs, &Groups::default(), &Constraints::default(), p);
        let ms = t0.elapsed().as_millis();
        let props = proposals(&fs, &g, &Constraints::default(), p, 3);
        let s = score(&g, &labels, &fs, &props);
        let junk = g
            .members()
            .values()
            .filter(|m| m.len() >= 3)
            .filter(|m| {
                let mut v: Vec<f32> = m.iter().filter_map(|f| records.get(f)).map(|r| r.score).collect();
                v.sort_by(f32::total_cmp);
                v[v.len() / 2] < 0.8
            })
            .count();
        println!(
            "{name:<36} {:>9.3} {:>7.3} {:>6} {:>7} {junk:>5} {:>7.0}% {:>4} ({:>3} ok) {ms:>6}",
            s.precision,
            s.recall,
            s.wrong,
            s.groups,
            s.grouped * 100.0,
            s.proposals,
            s.good_proposals
        );
        if *p == Params::default() {
            println!("  {} marked faces; per person with 10+ (faces, groups, share in the largest):", s.labelled);
            for (n, (_, count, groups, share)) in s.people.iter().enumerate() {
                println!("    person {n:>2}: {count:>4} faces, {groups:>2} groups, {:>3.0} % in the largest", share * 100.0);
            }
            // what the largest groups are made of
            let mut list: Vec<(u32, Vec<FaceId>)> = g.members().into_iter().collect();
            list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
            // what non-faces look like: the mean of the detections below 0.7 (mostly not faces)
            let mut junk = vec![0f32; 128];
            for r in records.values().filter(|r| r.score < 0.7) {
                if let Some(v) = &r.vector {
                    junk.iter_mut().zip(v).for_each(|(j, x)| *j += x);
                }
            }
            crate::ai::search::normalise(&mut junk);
            let vec_of: HashMap<FaceId, &Vec<f32>> = fs.iter().map(|f| (f.id, &f.vector)).collect();
            let cos = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
            let pct = |mut v: Vec<f32>, q: f32| {
                v.sort_by(f32::total_cmp);
                v.get(((v.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0.0)
            };
            let marked_junk: Vec<f32> = labels.keys().filter_map(|f| vec_of.get(f)).map(|v| cos(v, &junk)).collect();
            println!("  likeness to non-faces: marked faces p50 {:.2} p99 {:.2} max {:.2}", pct(marked_junk.clone(), 0.5), pct(marked_junk.clone(), 0.99), pct(marked_junk, 1.0));
            // how alike a group's faces are (mean of 400 pairs), for groups of marked people
            let coherence = |m: &[FaceId]| {
                let vs: Vec<&Vec<f32>> = m.iter().filter_map(|f| vec_of.get(f).copied()).collect();
                let (mut sum, mut n) = (0f32, 0);
                for a in 0..vs.len().min(20) {
                    for b in 0..vs.len().min(20) {
                        if a != b {
                            sum += cos(vs[a * vs.len() / vs.len().min(20)], vs[b * vs.len() / vs.len().min(20)]);
                            n += 1;
                        }
                    }
                }
                if n == 0 { 0.0 } else { sum / n as f32 }
            };
            println!("  rank group faces marked  score>=0.9  median score  median eyes  median sharpness  median |yaw|  alike  non-face");
            for (rank, (id, m)) in list.iter().take(40).enumerate() {
                let rs: Vec<&FaceRecord> = m.iter().filter_map(|f| records.get(f)).collect();
                let med = |mut v: Vec<f32>| {
                    v.sort_by(f32::total_cmp);
                    v.get(v.len() / 2).copied().unwrap_or(0.0)
                };
                let marked = m.iter().filter(|f| labels.contains_key(f)).count();
                let nonface = pct(m.iter().filter_map(|f| vec_of.get(f)).map(|v| cos(v, &junk)).collect(), 0.5);
                println!(
                    "  {rank:>4} {id:>5} {:>5} {marked:>6}  {:>9.0}%  {:>12.3}  {:>11.1}  {:>16.0}  {:>12.2}  {:>5.2}  {nonface:>8.2}",
                    m.len(),
                    100.0 * rs.iter().filter(|r| r.score >= 0.9).count() as f32 / rs.len().max(1) as f32,
                    med(rs.iter().map(|r| r.score).collect()),
                    med(rs.iter().map(|r| r.eyes).collect()),
                    med(rs.iter().map(|r| r.sharpness).collect()),
                    med(rs.iter().map(|r| r.yaw.abs()).collect()),
                    coherence(m),
                );
            }
            best_groups = Some(g);
        }
    }
    if let (Ok(out), Some(g)) = (std::env::var("ALBUMCRAFT_FACES_OUT"), best_groups) {
        sheets(Path::new(&out), &g, &keys, &records);
        println!("contact sheets of the 40 largest groups in {out}");
    }
}
