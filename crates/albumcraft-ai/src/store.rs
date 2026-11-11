//! What the networks found in each picture: one file per kind of result and network, in
//! `<data dir>/AlbumCraft/analysis/`, made of records of one size that are only ever added.
//!
//! A picture is named by a number made from its path, with the date and size of its file: when
//! the file changes, its records no longer count and the picture is analysed again (the same
//! rule as the thumbnail cache). A picture can have several records (one per face) or a single
//! "nothing found" mark, so that it is not looked at again.
//!
//! These files can always be made again from the pictures, but that takes hours, so they live
//! with the user's data and not in the cache. A file that is damaged or of another format is
//! started again; a record half written when the computer stopped is dropped. The user's own
//! decisions (names, corrections) are never kept here: they belong in `library.json`.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"ACANALYS";
const FORMAT: u32 = 1;
const HEADER_LEN: usize = 64;
const MODEL_LEN: usize = 48;
/// Before each record's payload: the picture (16), its file's date (8) and size (8), flags (4).
const HEAD_LEN: usize = 36;
/// The record holds no result: the picture was analysed and nothing was found.
const NOTHING: u32 = 1;
/// The last record of its picture (one call of `put`): a group without it was cut short.
const LAST: u32 = 2;
/// The longest record accepted (a file's header is input like any other).
const MAX_RECORD: usize = 1 << 20;

/// Where the analysis files are kept.
pub fn analysis_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("analysis"))
}

/// A file's date (seconds) and size (bytes): what tells that a picture changed.
pub type Stamp = (i64, u64);

/// A file of results: what is in it and which network made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The kind of result: "embeddings", "faces". Lower-case letters, digits and "-".
    pub kind: String,
    /// The network and its version, like "mobileclip-s2-1": results of two networks cannot be
    /// mixed, so each has its own file, and the old one keeps working while the new one is made.
    pub model: String,
    /// Bytes per record.
    pub record_len: usize,
}

impl Spec {
    fn valid(&self) -> bool {
        let name = |s: &str, max: usize| !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        name(&self.kind, 32) && name(&self.model, MODEL_LEN) && self.record_len > 0 && self.record_len <= MAX_RECORD
    }

    fn file_name(&self) -> String {
        format!("{}.{}.bin", self.kind, self.model)
    }

    fn header(&self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[..8].copy_from_slice(MAGIC);
        h[8..12].copy_from_slice(&FORMAT.to_le_bytes());
        h[12..16].copy_from_slice(&(self.record_len as u32).to_le_bytes());
        // `valid` keeps the name within MODEL_LEN
        let model = self.model.as_bytes();
        let n = model.len().min(MODEL_LEN);
        h[16..16 + n].copy_from_slice(&model[..n]);
        h
    }
}

/// The number a picture is known by in these files (from its path; the same on every run).
pub fn picture_id(key: &str) -> u128 {
    let hash = Sha256::digest(key.as_bytes());
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash[..16]);
    u128::from_le_bytes(id)
}

struct Entry {
    stamp: Stamp,
    /// Its records, as indexes into `Table.records` (none: nothing was found).
    rows: Vec<u32>,
}

/// One file of results, read into memory; `put` adds to both.
pub struct Table {
    spec: Spec,
    path: PathBuf,
    /// Open for adding once something was added.
    file: Option<std::fs::File>,
    /// Every record's payload, one after the other (also those that no longer count).
    records: Vec<u8>,
    pictures: HashMap<u128, Entry>,
    /// Records in the file that no longer count (their picture changed), "nothing found" marks
    /// included: what `compact` would win back.
    dead: usize,
    /// Records in the file, marks included.
    in_file: usize,
}

impl Table {
    /// Open the file of `spec` in `dir`, or start it. Never fails because of what the file
    /// holds: a file that can't be used is replaced.
    pub fn open(dir: &Path, spec: &Spec) -> Result<Table, String> {
        if !spec.valid() {
            return Err(format!("not a valid analysis file: {} / {}", spec.kind, spec.model));
        }
        let path = dir.join(spec.file_name());
        let mut table = Table { spec: spec.clone(), path, file: None, records: vec![], pictures: HashMap::new(), dead: 0, in_file: 0 };
        let bytes = match std::fs::read(&table.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(e) => return Err(format!("{}: {e}", table.path.display())),
        };
        let whole = table.read(&bytes);
        if whole != bytes.len() as u64 {
            // a header that is not ours (whole == 0) or a cut-off end: keep only the good part
            let cut = std::fs::OpenOptions::new().write(true).open(&table.path).and_then(|f| f.set_len(whole));
            cut.map_err(|e| format!("{}: {e}", table.path.display()))?;
        }
        if table.dead > 1000 && table.dead > table.in_file / 4 {
            table.compact()?;
        }
        Ok(table)
    }

    /// Take in the bytes of the file; returns how many of them are good (0: start again).
    fn read(&mut self, bytes: &[u8]) -> u64 {
        if bytes.get(..HEADER_LEN) != Some(&self.spec.header()[..]) {
            return 0;
        }
        let size = HEAD_LEN + self.spec.record_len;
        let mut good = HEADER_LEN;
        // the records of the picture being read, until its LAST one is seen
        let mut group: Vec<&[u8]> = vec![];
        for rec in bytes[HEADER_LEN..].chunks_exact(size) {
            let (Some(id), Some(modified), Some(len), Some(flags)) = (le::<16>(rec, 0), le::<8>(rec, 16), le::<8>(rec, 24), le::<4>(rec, 32)) else { break };
            let id = u128::from_le_bytes(id);
            if group.first().is_some_and(|first| first[..32] != rec[..32]) {
                // the group before was cut short and another one follows: the file is not as
                // this program writes it; stop at the last good place
                break;
            }
            group.push(rec);
            if u32::from_le_bytes(flags) & LAST != 0 {
                let stamp = (i64::from_le_bytes(modified), u64::from_le_bytes(len));
                let found: Vec<&[u8]> = group.iter().filter(|r| le::<4>(r, 32).is_some_and(|f| u32::from_le_bytes(f) & NOTHING == 0)).map(|r| &r[HEAD_LEN..]).collect();
                self.in_file += group.len();
                self.remember(id, stamp, &found);
                good += group.len() * size;
                group.clear();
            }
        }
        good as u64
    }

    /// Keep these records as the picture's, in place of what it had.
    fn remember(&mut self, id: u128, stamp: Stamp, found: &[&[u8]]) {
        let mut rows = Vec::with_capacity(found.len());
        for payload in found {
            rows.push((self.records.len() / self.spec.record_len) as u32);
            self.records.extend_from_slice(payload);
        }
        if let Some(old) = self.pictures.insert(id, Entry { stamp, rows }) {
            self.dead += old.rows.len().max(1);
        }
    }

    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// The picture was analysed as it is now.
    pub fn has(&self, key: &str, stamp: Stamp) -> bool {
        self.pictures.get(&picture_id(key)).is_some_and(|e| e.stamp == stamp)
    }

    /// What was found in the picture as it is now: None when it was not analysed (or changed
    /// since), no records when nothing was found.
    pub fn get(&self, key: &str, stamp: Stamp) -> Option<Vec<&[u8]>> {
        let e = self.pictures.get(&picture_id(key)).filter(|e| e.stamp == stamp)?;
        Some(e.rows.iter().filter_map(|&r| self.record(r)).collect())
    }

    fn record(&self, row: u32) -> Option<&[u8]> {
        let at = (row as usize).checked_mul(self.spec.record_len)?;
        self.records.get(at..at.checked_add(self.spec.record_len)?)
    }

    /// How many pictures were analysed.
    pub fn pictures(&self) -> usize {
        self.pictures.len()
    }

    /// Every record that counts, with its picture (`picture_id`), in no particular order: what a
    /// search walks through.
    pub fn records(&self) -> impl Iterator<Item = (u128, &[u8])> {
        self.pictures.iter().flat_map(move |(id, e)| e.rows.iter().filter_map(move |&r| Some((*id, self.record(r)?))))
    }

    /// Record what was found in a picture (nothing: it is marked as looked at), in place of
    /// what was known of it. Written to the file at once.
    pub fn put(&mut self, key: &str, stamp: Stamp, found: &[Vec<u8>]) -> Result<(), String> {
        if let Some(bad) = found.iter().find(|r| r.len() != self.spec.record_len) {
            return Err(format!("a record of {} bytes where {} are expected", bad.len(), self.spec.record_len));
        }
        let id = picture_id(key);
        let found: Vec<&[u8]> = found.iter().map(Vec::as_slice).collect();
        let mut bytes = vec![];
        encode(id, stamp, &found, self.spec.record_len, &mut bytes);
        self.append(&bytes).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.in_file += found.len().max(1);
        self.remember(id, stamp, &found);
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if self.file.is_none() {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&self.path)?;
            if f.seek(SeekFrom::End(0))? == 0 {
                f.write_all(&self.spec.header())?;
            }
            self.file = Some(f);
        }
        let Some(f) = self.file.as_mut() else { return Ok(()) };
        // one write per picture, at the end: what is in the file before it is never touched
        let end = f.seek(SeekFrom::End(0))?;
        let written = f.write_all(bytes);
        if written.is_err() {
            // a full disk: don't leave half a picture behind
            let _ = f.set_len(end);
        }
        written
    }

    /// Write the file again without the records that no longer count (a temporary file, then
    /// renamed over the old one).
    pub fn compact(&mut self) -> Result<(), String> {
        let failed = |e: std::io::Error| format!("{}: {e}", self.path.display());
        let mut bytes = self.spec.header().to_vec();
        for (id, e) in &self.pictures {
            let found: Vec<&[u8]> = e.rows.iter().filter_map(|&r| self.record(r)).collect();
            encode(*id, e.stamp, &found, self.spec.record_len, &mut bytes);
        }
        self.file = None;
        let tmp = self.path.with_extension("bin.tmp");
        std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &self.path)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            failed(e)
        })?;
        // read it back in, so that memory and file agree again
        let spec = self.spec.clone();
        let path = std::mem::take(&mut self.path);
        *self = Table { spec, path, file: None, records: vec![], pictures: HashMap::new(), dead: 0, in_file: 0 };
        self.read(&bytes);
        Ok(())
    }

    /// Bytes of the file on disk.
    pub fn file_bytes(&self) -> u64 {
        (HEADER_LEN + self.in_file * (HEAD_LEN + self.spec.record_len)) as u64
    }
}

/// A picture's records as they stand in the file (a single "nothing found" mark when there are
/// none), added to `out`.
fn encode(id: u128, stamp: Stamp, found: &[&[u8]], record_len: usize, out: &mut Vec<u8>) {
    let head = |flags: u32, out: &mut Vec<u8>| {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&stamp.0.to_le_bytes());
        out.extend_from_slice(&stamp.1.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
    };
    if found.is_empty() {
        head(NOTHING | LAST, out);
        out.resize(out.len() + record_len, 0);
    }
    for (i, payload) in found.iter().enumerate() {
        head(if i + 1 == found.len() { LAST } else { 0 }, out);
        out.extend_from_slice(payload);
    }
}

fn le<const N: usize>(rec: &[u8], at: usize) -> Option<[u8; N]> {
    rec.get(at..at + N)?.try_into().ok()
}

/// Remove every file of one kind of result ("faces": the user asked to delete all face data).
pub fn remove_kind(dir: &Path, kind: &str) -> Result<(), String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Ok(()) };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.strip_prefix(kind).is_some_and(|rest| rest.starts_with('.')) {
            std::fs::remove_file(e.path()).map_err(|err| format!("{}: {err}", e.path().display()))?;
        }
    }
    Ok(())
}

/// The files of one kind of result in `dir`, as (network, bytes on disk).
pub fn files_of(dir: &Path, kind: &str) -> Vec<(String, u64)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut out = vec![];
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(model) = name.strip_prefix(kind).and_then(|r| r.strip_prefix('.')).and_then(|r| r.strip_suffix(".bin")) else { continue };
        let mut magic = [0u8; 8];
        let ours = std::fs::File::open(e.path()).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && &magic == MAGIC;
        if ours {
            out.push((model.to_string(), e.metadata().map(|m| m.len()).unwrap_or(0)));
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("albumcraft-ai-store-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn spec() -> Spec {
        Spec { kind: "embeddings".into(), model: "test-1".into(), record_len: 4 }
    }

    #[test]
    fn results_are_kept_across_runs() {
        let dir = temp("kept");
        let mut t = Table::open(&dir, &spec()).unwrap();
        assert!(!dir.exists(), "nothing is written before there is something to keep");
        assert_eq!(t.get("/p/a.jpg", (10, 100)), None);
        t.put("/p/a.jpg", (10, 100), &[vec![1, 2, 3, 4]]).unwrap();
        t.put("/p/faces.jpg", (11, 200), &[vec![5; 4], vec![6; 4], vec![7; 4]]).unwrap();
        t.put("/p/empty.jpg", (12, 300), &[]).unwrap();
        for t in [&t, &Table::open(&dir, &spec()).unwrap()] {
            assert_eq!(t.pictures(), 3);
            assert_eq!(t.get("/p/a.jpg", (10, 100)), Some(vec![&[1u8, 2, 3, 4][..]]));
            assert_eq!(t.get("/p/faces.jpg", (11, 200)).map(|r| r.len()), Some(3));
            assert_eq!(t.get("/p/empty.jpg", (12, 300)), Some(vec![]));
            assert!(t.has("/p/empty.jpg", (12, 300)));
            // the file changed: what was found no longer counts
            assert_eq!(t.get("/p/a.jpg", (99, 100)), None);
            assert!(!t.has("/p/a.jpg", (10, 101)));
            assert_eq!(t.records().count(), 4);
            assert_eq!(t.file_bytes(), std::fs::metadata(dir.join("embeddings.test-1.bin")).unwrap().len());
        }
        assert!(t.put("/p/b.jpg", (1, 1), &[vec![1, 2, 3]]).is_err(), "a record of another size is refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_changed_picture_replaces_its_results() {
        let dir = temp("changed");
        let mut t = Table::open(&dir, &spec()).unwrap();
        t.put("/p/a.jpg", (10, 100), &[vec![1; 4], vec![2; 4]]).unwrap();
        t.put("/p/a.jpg", (20, 150), &[vec![3; 4]]).unwrap();
        let again = Table::open(&dir, &spec()).unwrap();
        for t in [&t, &again] {
            assert_eq!(t.get("/p/a.jpg", (10, 100)), None);
            assert_eq!(t.get("/p/a.jpg", (20, 150)), Some(vec![&[3u8; 4][..]]));
            assert_eq!((t.pictures(), t.records().count(), t.dead), (1, 1, 2));
        }
        let before = t.file_bytes();
        t.compact().unwrap();
        assert!(t.file_bytes() < before);
        assert_eq!(t.dead, 0);
        t.put("/p/b.jpg", (1, 1), &[vec![9; 4]]).unwrap();
        let again = Table::open(&dir, &spec()).unwrap();
        assert_eq!(again.get("/p/a.jpg", (20, 150)), Some(vec![&[3u8; 4][..]]));
        assert_eq!(again.get("/p/b.jpg", (1, 1)), Some(vec![&[9u8; 4][..]]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The computer stopped while a picture's records were being written, or the file is not
    /// ours: no crash, the good part is kept, and the file is whole again afterwards.
    #[test]
    fn a_cut_or_foreign_file_is_repaired() {
        let dir = temp("cut");
        let file = dir.join("embeddings.test-1.bin");
        let mut t = Table::open(&dir, &spec()).unwrap();
        t.put("/p/a.jpg", (10, 100), &[vec![1; 4]]).unwrap();
        t.put("/p/faces.jpg", (11, 200), &[vec![5; 4], vec![6; 4]]).unwrap();
        drop(t);
        let whole = std::fs::read(&file).unwrap();
        // every possible cut: never a panic, the first picture survives once it is complete
        for cut in 0..whole.len() {
            std::fs::write(&file, &whole[..cut]).unwrap();
            let mut t = Table::open(&dir, &spec()).unwrap();
            let first_done = cut >= HEADER_LEN + HEAD_LEN + 4;
            assert_eq!(t.has("/p/a.jpg", (10, 100)), first_done, "cut at {cut}");
            assert!(!t.has("/p/faces.jpg", (11, 200)), "half a picture does not count (cut at {cut})");
            t.put("/p/new.jpg", (1, 1), &[vec![8; 4]]).unwrap();
            let again = Table::open(&dir, &spec()).unwrap();
            assert!(again.has("/p/new.jpg", (1, 1)) && again.has("/p/a.jpg", (10, 100)) == first_done, "cut at {cut}");
        }
        // another network's file under this name, and rubbish
        for foreign in [Spec { model: "other-2".into(), ..spec() }.header().to_vec(), vec![0xff; 500], b"hello".to_vec()] {
            std::fs::write(&file, &foreign).unwrap();
            let mut t = Table::open(&dir, &spec()).unwrap();
            assert_eq!(t.pictures(), 0);
            t.put("/p/a.jpg", (10, 100), &[vec![1; 4]]).unwrap();
            assert!(Table::open(&dir, &spec()).unwrap().has("/p/a.jpg", (10, 100)));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn kinds_are_listed_and_removed() {
        let dir = temp("kinds");
        let faces = Spec { kind: "faces".into(), model: "sface-1".into(), record_len: 8 };
        Table::open(&dir, &faces).unwrap().put("/p/a.jpg", (1, 1), &[vec![0; 8]]).unwrap();
        Table::open(&dir, &spec()).unwrap().put("/p/a.jpg", (1, 1), &[vec![0; 4]]).unwrap();
        assert_eq!(files_of(&dir, "faces"), [("sface-1".to_string(), (HEADER_LEN + HEAD_LEN + 8) as u64)]);
        remove_kind(&dir, "faces").unwrap();
        assert!(files_of(&dir, "faces").is_empty());
        assert_eq!(files_of(&dir, "embeddings").len(), 1);
        // names that could leave the folder are refused
        for bad in ["", "../x", "A", "a b"] {
            assert!(Table::open(&dir, &Spec { model: bad.into(), ..spec() }).is_err());
            assert!(Table::open(&dir, &Spec { kind: bad.into(), ..spec() }).is_err());
        }
        assert!(Table::open(&dir, &Spec { record_len: 0, ..spec() }).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
