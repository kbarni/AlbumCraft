//! The online map's tile downloader for the window: tiles fetched on two worker threads (only
//! the tiles in view; requests for areas scrolled away are dropped) and kept as textures. Only
//! used after the user turned online maps on. The services themselves (tiles on disk, place
//! search) are `albumcraft_core::online`, re-exported here.

pub use albumcraft_core::online::*;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

const DOWNLOADS: usize = 2;
/// Tiles kept as textures.
const IN_MEMORY: usize = 384;

struct Queue {
    /// Tiles wanted now, most important first.
    wanted: VecDeque<TileId>,
    busy: HashSet<TileId>,
}

/// Map tiles: downloaded on worker threads, kept on disk and as textures.
pub struct Tiles {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    rx: Receiver<(TileId, Result<ColorImage, String>)>,
    textures: HashMap<TileId, (TextureHandle, u64)>,
    /// Tiles that failed, and when (tried again after a while).
    failed: HashMap<TileId, Instant>,
    frame: u64,
    /// The last download problem, for the map's status line.
    pub problem: Option<String>,
}

impl Tiles {
    pub fn new(ctx: &egui::Context) -> Tiles {
        let queue = Arc::new((Mutex::new(Queue { wanted: VecDeque::new(), busy: HashSet::new() }), Condvar::new()));
        let (tx, rx) = std::sync::mpsc::channel();
        let agent = agent();
        for _ in 0..DOWNLOADS {
            let (q, tx, ctx, agent) = (queue.clone(), tx.clone(), ctx.clone(), agent.clone());
            std::thread::spawn(move || worker(q, tx, ctx, agent));
        }
        std::thread::spawn(|| {
            if let Some(d) = tile_cache_dir() {
                prune(&d, CACHE_BYTES);
            }
        });
        Tiles { queue, rx, textures: HashMap::new(), failed: HashMap::new(), frame: 0, problem: None }
    }

    /// Take finished downloads. Call once per frame before `get`.
    pub fn begin_frame(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        while let Ok((id, result)) = self.rx.try_recv() {
            match result {
                Ok(img) => {
                    let tex = ctx.load_texture(format!("tile{}/{}/{}", id.z, id.x, id.y), img, TextureOptions::LINEAR);
                    self.textures.insert(id, (tex, self.frame));
                    self.failed.remove(&id);
                    self.problem = None;
                }
                Err(e) => {
                    self.failed.insert(id, Instant::now());
                    self.problem = Some(e);
                }
            }
        }
        if self.textures.len() > IN_MEMORY {
            let mut by_age: Vec<(u64, TileId)> = self.textures.iter().map(|(id, (_, f))| (*f, *id)).collect();
            by_age.sort_unstable_by_key(|(f, _)| *f);
            for (_, id) in by_age.into_iter().take(self.textures.len() - IN_MEMORY) {
                self.textures.remove(&id);
            }
        }
    }

    /// A tile's texture if it is here; otherwise None (call `want` with the missing ones).
    pub fn get(&mut self, id: TileId) -> Option<TextureHandle> {
        let frame = self.frame;
        self.textures.get_mut(&id).map(|(t, f)| {
            *f = frame;
            t.clone()
        })
    }

    /// Tiles are being downloaded or waiting to be.
    pub fn pending(&self) -> bool {
        let (lock, _) = &*self.queue;
        let q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        !q.wanted.is_empty() || !q.busy.is_empty()
    }

    /// Ask for these tiles (most important first), replacing what was asked before.
    pub fn want(&mut self, ids: Vec<TileId>) {
        let retry = Duration::from_secs(30);
        let (lock, cv) = &*self.queue;
        let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        q.wanted = ids
            .into_iter()
            .filter(|id| !self.textures.contains_key(id) && !q.busy.contains(id) && self.failed.get(id).is_none_or(|t| t.elapsed() > retry))
            .collect();
        if !q.wanted.is_empty() {
            cv.notify_all();
        }
    }
}

fn worker(queue: Arc<(Mutex<Queue>, Condvar)>, tx: Sender<(TileId, Result<ColorImage, String>)>, ctx: egui::Context, agent: ureq::Agent) {
    let dir = tile_cache_dir();
    loop {
        let id = {
            let (lock, cv) = &*queue;
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(id) = q.wanted.pop_front() {
                    q.busy.insert(id);
                    break id;
                }
                q = cv.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| load_tile(&agent, dir.as_deref(), id)))
            .map(|r| r.map(|img| ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())))
            .unwrap_or_else(|_| Err("a map tile could not be read".into()));
        {
            let (lock, _) = &*queue;
            lock.lock().unwrap_or_else(PoisonError::into_inner).busy.remove(&id);
        }
        if tx.send((id, result)).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}
