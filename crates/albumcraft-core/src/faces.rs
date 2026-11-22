//! Faces in the library: which face analyser the background analysis runs.
//!
//! Faces are looked for only when the user said yes (`Store.faces_enabled`, asked once, also
//! in Settings › AI features) and the AI features are on. With the *Faces* pack downloaded,
//! each face gets a vector (`ai::faces`, the table "faces.yunet-1-<recogniser>"); without it
//! they are only found ("faces.yunet-1"), which the pictures are looked at again for once the
//! pack is there. Grouping the faces by person comes after (`face_groups`, not yet).
//!
//! Face vectors are biometric data: they stay in the analysis folder of this computer
//! (`ai::store::analysis_dir`), never in `library.json`, sidecars or exported copies.

use crate::ai::faces::{FaceAnalyser, detect_only, table};
use crate::ai::models::{self, Pack};
use crate::ai::store::{Spec, Table};
use crate::analysis::{Analysis, SharedTable};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock};

/// The id of the Faces pack in the built-in list.
pub const PACK: &str = "faces";

/// The Faces pack of the built-in list.
pub fn pack() -> Option<&'static Pack> {
    models::manifest().pack(PACK).filter(|p| p.faces.is_some())
}

/// The Faces pack is on this computer.
pub fn pack_installed() -> bool {
    match (pack(), models::models_dir()) {
        (Some(p), Some(dir)) => p.installed(&dir),
        _ => false,
    }
}

/// Every table faces can be kept in: found only, and with the vectors of the built-in pack.
fn tables() -> Vec<Spec> {
    let mut all = vec![detect_only()];
    all.extend(pack().and_then(|p| p.faces.as_ref()).map(|c| table(Some(c))));
    all
}

type Loaded = (Arc<FaceAnalyser>, SharedTable);

/// The analysers made so far, by table (networks are loaded once per program).
fn analysers() -> std::sync::MutexGuard<'static, HashMap<String, Loaded>> {
    static ALL: LazyLock<Mutex<HashMap<String, Loaded>>> = LazyLock::new(Default::default);
    ALL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn analyser(analysis: &Analysis, with_pack: bool) -> Result<Loaded, String> {
    let dir = analysis.dir().ok_or_else(crate::smart_search::no_folder)?;
    let models = models::models_dir().ok_or_else(crate::smart_search::no_folder)?;
    let a = FaceAnalyser::new(if with_pack { pack().map(|p| (p, models.as_path())) } else { None });
    let spec = crate::ai::Analyser::table(&a);
    let mut all = analysers();
    if let Some(l) = all.get(&spec.model) {
        return Ok(l.clone());
    }
    let t = Arc::new(RwLock::new(Table::open(dir, &spec)?));
    let l = (Arc::new(a), t);
    all.insert(spec.model.clone(), l.clone());
    Ok(l)
}

/// Make the background analysis look for faces (`on`), with vectors when the Faces pack is
/// downloaded, or not at all. Called by the window when either changes. What was found stays
/// in its file either way.
pub fn use_faces(analysis: &Analysis, on: bool) -> Result<(), String> {
    let with_pack = pack_installed();
    let wanted = on.then(|| if with_pack { pack().and_then(|p| p.faces.as_ref()).map(|c| table(Some(c))) } else { Some(detect_only()) }).flatten();
    for spec in tables() {
        if Some(&spec) != wanted.as_ref() {
            analysis.remove(&spec);
        }
    }
    if !with_pack {
        // the recogniser's network leaves memory once its pack is removed
        analysers().retain(|_, (a, _)| a.recogniser().is_none());
    }
    if wanted.is_none() {
        return Ok(());
    }
    let (a, t) = analyser(analysis, with_pack)?;
    analysis.add_with(a, t);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_faces_pack_is_built_in_with_its_recogniser() {
        let p = pack().expect("a Faces pack in models.json");
        let cfg = p.faces.as_ref().unwrap();
        assert!(cfg.valid(p));
        assert!(p.files.iter().any(|f| f.name == cfg.recogniser));
        let t = tables();
        assert_eq!(t.len(), 2);
        assert_ne!(t[0], t[1], "found-only faces and faces with vectors are kept apart");
    }

    /// The face analysis on real pictures, as the background analysis runs it (1400 px, the
    /// full-size picture for small faces): speed and what is found.
    /// `ALBUMCRAFT_FACES_PHOTOS=<folder> ALBUMCRAFT_FACES_PACK=<folder with sface-1.onnx>
    /// [ALBUMCRAFT_FACES_COUNT=200] cargo test --release -p albumcraft-core real_faces_analysis
    /// -- --ignored --nocapture`. Two threads, like the background analysis.
    #[test]
    #[ignore]
    fn real_faces_analysis() {
        use crate::ai::Analyser;
        let photos = std::env::var("ALBUMCRAFT_FACES_PHOTOS").expect("ALBUMCRAFT_FACES_PHOTOS");
        let dir = std::path::PathBuf::from(std::env::var("ALBUMCRAFT_FACES_PACK").expect("ALBUMCRAFT_FACES_PACK"));
        let count: usize = std::env::var("ALBUMCRAFT_FACES_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(200);
        let mut files: Vec<String> = walk(std::path::Path::new(&photos));
        files.sort();
        // spread over the folders rather than the first ones
        let step = (files.len() / count.max(1)).max(1);
        let files: Vec<String> = files.into_iter().step_by(step).take(count).collect();
        let a = Arc::new(FaceAnalyser::new(Some((pack().unwrap(), dir.as_path()))));
        let started = std::time::Instant::now();
        let results: Vec<(u128, Vec<crate::ai::faces::FaceRecord>)> = std::thread::scope(|s| {
            let chunks: Vec<&[String]> = files.chunks(files.len().div_ceil(2).max(1)).collect();
            let handles: Vec<_> = chunks
                .into_iter()
                .map(|chunk| {
                    let a = a.clone();
                    s.spawn(move || {
                        chunk
                            .iter()
                            .filter_map(|key| {
                                let t = std::time::Instant::now();
                                let picture = crate::imaging::load_source(key, a.input_dim())?;
                                let full = || crate::imaging::load_source(key, 4096);
                                let found = a.faces(&picture, &full).unwrap();
                                Some((t.elapsed().as_millis(), found))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let wall = started.elapsed().as_secs_f32();
        let faces: Vec<&crate::ai::faces::FaceRecord> = results.iter().flat_map(|(_, f)| f).collect();
        let with = |p: &dyn Fn(&crate::ai::faces::FaceRecord) -> bool| faces.iter().filter(|f| p(f)).count();
        let mut ms: Vec<u128> = results.iter().map(|(t, _)| *t).collect();
        ms.sort_unstable();
        println!(
            "{} pictures in {wall:.1} s on 2 threads ({:.0} ms each; one picture: median {} ms, slowest {} ms); {} with faces; {} faces: {} with a vector, {} cut from the full-size picture, {} scored 0.9 or more",
            results.len(),
            wall * 1000.0 / results.len().max(1) as f32,
            ms.get(ms.len() / 2).copied().unwrap_or(0),
            ms.last().copied().unwrap_or(0),
            results.iter().filter(|(_, f)| !f.is_empty()).count(),
            faces.len(),
            with(&|f| f.vector.is_some()),
            with(&|f| f.from_full),
            with(&|f| f.score >= 0.9),
        );
        println!("50 000 pictures would take about {:.1} h", wall / results.len().max(1) as f32 * 50_000.0 / 3600.0);
    }

    fn walk(dir: &std::path::Path) -> Vec<String> {
        let mut out = vec![];
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "jpg" | "jpeg")) {
                out.push(p.to_string_lossy().into_owned());
            }
        }
        out
    }
}
