//! Timings on real pictures (ignored tests, a developer aid):
//!
//!     ALBUMCRAFT_PERF_DIR=/path/to/photos cargo test --release -p albumcraft-core perf_ -- --ignored --nocapture

use std::time::Instant;

fn sample_files(n: usize) -> Vec<String> {
    let Some(dir) = std::env::var_os("ALBUMCRAFT_PERF_DIR") else { return vec![] };
    let (folders, _) = crate::library::scan_roots(&[dir.to_string_lossy().to_string()], n);
    folders.into_iter().flat_map(|(_, f)| f).map(|f| f.0).filter(|f| { let l = f.to_lowercase(); l.ends_with(".jpg") || (std::env::var_os("ALBUMCRAFT_PERF_RAW").is_some() && crate::raw::is_raw(&l)) }).take(n).collect()
}

fn time(what: &str, files: &[String], f: impl Fn(&str)) {
    let t = Instant::now();
    for k in files {
        f(k);
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("{what:<34} {:>8.1} ms each   ({} files, {:.0} ms)", ms / files.len().max(1) as f64, files.len(), ms);
}

#[test]
#[ignore]
fn perf_pictures() {
    let files = sample_files(40);
    if files.is_empty() {
        println!("set ALBUMCRAFT_PERF_DIR");
        return;
    }
    // read every file once so the timings below measure work, not the disk
    time("read whole file (disk)", &files, |k| {
        let _ = std::fs::read(k);
    });
    time("oriented_dims", &files, |k| {
        let _ = crate::imaging::oriented_dims(k);
    });
    time("jpeg_head", &files, |k| {
        let _ = crate::imaging::jpeg_head(k);
    });
    time("embedded_preview 256", &files, |k| {
        assert!(crate::imaging::embedded_preview(k, 256).is_some(), "{k}: no usable preview");
    });
    time("exifinfo::read", &files, |k| {
        let _ = crate::exifinfo::read(k);
    });
    time("load_source 1400", &files, |k| {
        let _ = crate::imaging::load_source(k, 1400);
    });
    time("load_source 5000 (zoom: full decode)", &files, |k| {
        let _ = crate::imaging::load_source(k, 5000);
    });
    time("jpeg-decoder 1/8 scale", &files, |k| {
        let Ok(f) = std::fs::File::open(k) else { return };
        let mut d = jpeg_decoder::Decoder::new(std::io::BufReader::new(f));
        let _ = d.scale(256, 256);
        let _ = d.decode();
    });
    time("jpeg-decoder 1/2 scale", &files, |k| {
        let Ok(f) = std::fs::File::open(k) else { return };
        let mut d = jpeg_decoder::Decoder::new(std::io::BufReader::new(f));
        let _ = d.scale(1400, 1400);
        let _ = d.decode();
    });
    time("jpeg-decoder full", &files, |k| {
        let Ok(f) = std::fs::File::open(k) else { return };
        let mut d = jpeg_decoder::Decoder::new(std::io::BufReader::new(f));
        let _ = d.decode();
    });
}

/// Thumbnails of a whole folder on 6 threads, as the library's pool makes them (no disk cache).
#[test]
#[ignore]
fn perf_folder_thumbnails() {
    let files = sample_files(100_000);
    if files.is_empty() {
        println!("set ALBUMCRAFT_PERF_DIR");
        return;
    }
    let run = |what: &str, f: fn(&str, u32) -> Option<image::RgbaImage>| {
        let t = Instant::now();
        let next = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..6 {
                s.spawn(|| {
                    while let Some(k) = files.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
                        let _ = f(k, crate::library::THUMB_DIM);
                    }
                });
            }
        });
        println!("{what:<34} {} thumbnails in {:.1} s", files.len(), t.elapsed().as_secs_f64());
    };
    run("embedded preview (new)", crate::imaging::load_thumbnail);
    if std::env::var_os("ALBUMCRAFT_PERF_OLD").is_some() {
        run("full decode (old)", crate::imaging::load_source);
    }
}

/// Import: list ALBUMCRAFT_PERF_DIR against a library of ALBUMCRAFT_PERF_LIBRARY, then copy it
/// into ALBUMCRAFT_PERF_DEST (removed afterwards).
#[test]
#[ignore]
fn perf_import() {
    let (Some(src), Some(dest)) = (std::env::var_os("ALBUMCRAFT_PERF_DIR"), std::env::var_os("ALBUMCRAFT_PERF_DEST")) else {
        println!("set ALBUMCRAFT_PERF_DIR and ALBUMCRAFT_PERF_DEST");
        return;
    };
    let library: Vec<(String, u64)> = match std::env::var_os("ALBUMCRAFT_PERF_LIBRARY") {
        Some(l) => {
            let (folders, _) = crate::library::scan_roots(&[l.to_string_lossy().to_string()], 1_000_000);
            folders.into_iter().flat_map(|(_, f)| f).map(|(k, _, b)| (k, b)).collect()
        }
        None => vec![],
    };
    let t = Instant::now();
    let (found, _) = crate::import::list(std::path::Path::new(&src), &library);
    let dups = found.iter().filter(|c| c.duplicate).count();
    println!("list {} pictures ({} duplicates, library of {}): {:.2} s", found.len(), dups, library.len(), t.elapsed().as_secs_f64());
    let dest = std::path::PathBuf::from(dest).join("albumcraft-perf-import");
    let _ = std::fs::remove_dir_all(&dest);
    let files: Vec<String> = found.iter().map(|c| c.path.clone()).collect();
    let bytes: u64 = found.iter().map(|c| c.bytes).sum();
    let t = Instant::now();
    let r = crate::import::copy_into(&files, &dest, &|_, _| {}, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    let s = t.elapsed().as_secs_f64();
    println!("copy {} files, {:.0} MB: {:.1} s ({:.0} MB/s), {} failed", r.copied.len(), bytes as f64 / 1e6, s, bytes as f64 / 1e6 / s, r.failed.len());
    let t = Instant::now();
    let again = crate::import::copy_into(&files, &dest, &|_, _| {}, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    println!("copy again (all already there: {}): {:.2} s", again.already_there, t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
#[ignore]
fn perf_raw() {
    let files = sample_files(20);
    if files.is_empty() {
        println!("set ALBUMCRAFT_PERF_DIR and ALBUMCRAFT_PERF_RAW=1");
        return;
    }
    time("read whole file (disk)", &files, |k| {
        let _ = std::fs::read(k);
    });
    time("raw preview_dims", &files, |k| {
        let _ = crate::raw::preview_dims(k);
    });
    time("raw thumbnail 256", &files, |k| {
        let _ = crate::imaging::load_thumbnail(k, 256);
    });
    time("raw preview 1400", &files, |k| {
        let _ = crate::imaging::load_source(k, 1400);
    });
}

#[test]
#[ignore]
fn perf_thumb_cache() {
    let files = sample_files(40);
    if files.is_empty() {
        return;
    }
    let thumbs: Vec<_> = files.iter().filter_map(|k| crate::imaging::load_thumbnail(k, 256).map(|t| (k.clone(), t))).collect();
    let t = Instant::now();
    for (k, img) in &thumbs {
        crate::thumbcache::put(k, 256, img);
    }
    println!("thumbcache put  {:.1} ms each", t.elapsed().as_secs_f64() * 1000.0 / thumbs.len() as f64);
    let t = Instant::now();
    for (k, _) in &thumbs {
        assert!(crate::thumbcache::get(k, 256).is_some());
    }
    println!("thumbcache get  {:.1} ms each", t.elapsed().as_secs_f64() * 1000.0 / thumbs.len() as f64);
}

/// The embedded preview shows the same picture, the same way up, as decoding the file.
#[test]
#[ignore]
fn perf_check_embedded_previews() {
    let files = sample_files(100_000);
    let (mut used, mut turned, mut worst) = (0, 0, 0.0f64);
    for k in &files {
        let Some(fast) = crate::imaging::embedded_preview(k, 256) else { continue };
        let Some(slow) = crate::imaging::load_source(k, 256) else { continue };
        used += 1;
        if crate::imaging::jpeg_head(k).is_some_and(|h| h.orientation != 1) {
            turned += 1;
        }
        assert_eq!(fast.dimensions(), slow.dimensions(), "{k}");
        let diff: f64 = fast.pixels().zip(slow.pixels()).map(|(a, b)| (0..3).map(|c| (a[c] as f64 - b[c] as f64).abs()).sum::<f64>()).sum::<f64>()
            / (fast.width() * fast.height() * 3) as f64;
        if diff > worst {
            worst = diff;
            if let Some(out) = std::env::var_os("ALBUMCRAFT_PERF_WORST") {
                let out = std::path::PathBuf::from(out);
                let _ = fast.save(out.join("worst-embedded.png"));
                let _ = slow.save(out.join("worst-decoded.png"));
                println!("worst so far: {k} {diff:.1}");
            }
        }
        if diff > 8.0 {
            println!("{diff:.1}  {k}");
        }
        assert!(diff < 20.0, "{k}: mean difference {diff:.1}");
    }
    println!("{used} of {} pictures use the embedded preview ({turned} turned); worst mean difference {worst:.1} / 255", files.len());
}
