//! Smart search: a network that turns a picture into a vector, and a sentence into a vector of
//! the same space (CLIP-style). A picture and a sentence that match have vectors pointing the
//! same way, so ranking the library for "red car in the snow" is one dot product per picture,
//! and so is "more like this picture".
//!
//! A search pack holds three files: the image encoder, the text encoder and the tokenizer
//! (`tokenizer.json`). What they expect is described in the pack's `search` section of
//! `models.json` (`Config`). The networks are loaded the first time they are needed; the
//! large text encoder only when a search is made.
//!
//! Vectors are kept with 8 bits per value (`quantise`): ranking with them gives the same order
//! as with full numbers, and the library's file is a quarter of the size.

use crate::models::Pack;
use crate::runtime::{Network, Tensor};
use crate::store::Spec;
use crate::Analyser;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What a search pack's networks expect; the `search` section of a pack in `models.json`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// The name its vectors are kept under (`store::Spec::model`), like "mobileclip-s2-1".
    /// Changing the network means changing it: vectors of two networks can't be compared.
    pub model: String,
    /// The pack's files holding the image encoder, the text encoder and the tokenizer.
    pub image: String,
    pub text: String,
    pub tokenizer: String,
    /// Values per vector.
    pub dim: usize,
    /// The side of the square picture the image encoder takes.
    pub size: u32,
    /// The whole picture is scaled to that square, out of shape (SigLIP), instead of its
    /// middle square being kept (CLIP).
    #[serde(default)]
    pub squash: bool,
    /// Per colour: the value is (x / 255 - mean) / std.
    #[serde(default = "zeros")]
    pub mean: [f32; 3],
    #[serde(default = "ones")]
    pub std: [f32; 3],
    /// Words (tokens) the text encoder takes, padded with `pad_id`.
    pub context: usize,
    #[serde(default)]
    pub pad_id: i64,
    /// The sentence the typed text is put into: "{}" is replaced by it.
    #[serde(default = "default_prompt")]
    pub prompt: String,
    /// The text is written in lower case before it is cut into words.
    #[serde(default)]
    pub lowercase: bool,
    /// ASCII punctuation is left out and "_" is a space (open_clip's "canonicalize", which
    /// the SigLIP networks were trained with).
    #[serde(default)]
    pub no_punctuation: bool,
    /// It understands other languages than English.
    #[serde(default)]
    pub multilingual: bool,
    /// The score below which a picture does not show what was typed (each network has its
    /// own scale: measured on pictures with and without the subject).
    #[serde(default)]
    pub min_score: f32,
    /// How far below the best match a typed text's matches are shown at first.
    #[serde(default = "default_margin")]
    pub text_margin: f32,
}

fn default_margin() -> f32 {
    0.05
}

fn zeros() -> [f32; 3] {
    [0.0; 3]
}
fn ones() -> [f32; 3] {
    [1.0; 3]
}
fn default_prompt() -> String {
    "a photo of {}".into()
}

/// The most values a vector may have (a file's settings are input like any other).
const MAX_DIM: usize = 4096;

impl Config {
    /// Well formed, and its files are files of `pack`.
    pub fn valid(&self, pack: &Pack) -> bool {
        let has = |name: &str| pack.files.iter().any(|f| f.name == name);
        let model_ok = !self.model.is_empty() && self.model.len() <= 48 && self.model.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        model_ok
            && has(&self.image)
            && has(&self.text)
            && has(&self.tokenizer)
            && (1..=MAX_DIM).contains(&self.dim)
            && (16..=1024).contains(&self.size)
            && (2..=512).contains(&self.context)
            && self.std.iter().all(|s| s.is_finite() && *s > 0.0)
            && self.mean.iter().all(|m| m.is_finite())
            && self.prompt.contains("{}")
    }

    /// The file the library's vectors are kept in.
    pub fn table(&self) -> Spec {
        Spec { kind: "embeddings".into(), model: self.model.clone(), record_len: record_len(self.dim) }
    }
}

/// Bytes of a kept vector: its scale (f32), then one signed byte per value.
pub fn record_len(dim: usize) -> usize {
    4 + dim
}

/// A vector made unit length (all zeros stays zeros).
pub fn normalise(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 && n.is_finite() {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

/// A vector as kept: its largest value maps to ±127.
pub fn quantise(v: &[f32]) -> Vec<u8> {
    let max = v.iter().fold(0f32, |m, x| if x.is_finite() { m.max(x.abs()) } else { m });
    let scale = if max > 0.0 { max / 127.0 } else { 0.0 };
    let mut out = Vec::with_capacity(record_len(v.len()));
    out.extend_from_slice(&scale.to_le_bytes());
    for x in v {
        let q = if scale > 0.0 && x.is_finite() { (x / scale).round().clamp(-127.0, 127.0) as i8 } else { 0 };
        out.push(q as u8);
    }
    out
}

/// The vector of a kept record (None: not a record of this length).
pub fn dequantise(record: &[u8], dim: usize) -> Option<Vec<f32>> {
    if record.len() != record_len(dim) {
        return None;
    }
    let scale = f32::from_le_bytes(record.get(..4)?.try_into().ok()?);
    Some(record[4..].iter().map(|&b| b as i8 as f32 * scale).collect())
}

/// How well a kept record matches `query` (a unit vector): the cosine of their angle, about
/// 0.1 for unrelated pictures and 0.3 for a good match with a sentence.
pub fn score(query: &[f32], record: &[u8]) -> f32 {
    let Some((scale, values)) = record.split_first_chunk::<4>() else { return f32::NEG_INFINITY };
    if values.len() != query.len() {
        return f32::NEG_INFINITY;
    }
    let dot: f32 = values.iter().zip(query).map(|(&b, q)| b as i8 as f32 * q).sum();
    dot * f32::from_le_bytes(*scale)
}

/// The picture as the image encoder takes it: the shorter side resized to `size` and the
/// middle square kept (or the whole picture squashed into the square), one plane per colour.
pub fn pixels(picture: &image::RgbaImage, cfg: &Config) -> Tensor {
    let size = cfg.size.max(1);
    let (w, h) = picture.dimensions();
    let (w, h) = (w.max(1), h.max(1));
    let k = size as f64 / w.min(h) as f64;
    let (rw, rh) = if cfg.squash { (size, size) } else { (((w as f64 * k).round() as u32).max(size), ((h as f64 * k).round() as u32).max(size)) };
    let resized = image::imageops::resize(picture, rw, rh, image::imageops::FilterType::Triangle);
    let (x0, y0) = ((rw - size) / 2, (rh - size) / 2);
    let n = (size * size) as usize;
    let mut data = vec![0f32; 3 * n];
    for y in 0..size {
        for x in 0..size {
            let p = resized.get_pixel(x0 + x, y0 + y);
            let i = (y * size + x) as usize;
            for c in 0..3 {
                data[c * n + i] = (p[c] as f32 / 255.0 - cfg.mean[c]) / cfg.std[c];
            }
        }
    }
    Tensor::F32 { shape: vec![1, 3, size as usize, size as usize], data }
}

/// The word numbers of a sentence, padded or cut to `context`. Cutting keeps the last one (the
/// end mark the encoder reads its result at).
pub fn tokens(tokenizer: &tokenizers::Tokenizer, text: &str, cfg: &Config) -> Result<Vec<i64>, String> {
    let mut text = if cfg.lowercase { text.to_lowercase() } else { text.to_string() };
    if cfg.no_punctuation {
        text = text.replace('_', " ").chars().filter(|c| !c.is_ascii_punctuation()).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    }
    let sentence = cfg.prompt.replacen("{}", text.trim(), 1);
    let enc = tokenizer.encode(sentence, true).map_err(|e| e.to_string())?;
    let mut ids: Vec<i64> = enc.get_ids().iter().map(|&i| i as i64).collect();
    // a tokenizer may pad by itself: what follows the first pad is not the sentence
    if let Some(pad) = enc.get_attention_mask().iter().position(|&m| m == 0) {
        ids.truncate(pad);
    }
    let context = cfg.context.max(1);
    if ids.len() > context {
        let last = ids.last().copied().unwrap_or(cfg.pad_id);
        ids.truncate(context);
        if let Some(end) = ids.last_mut() {
            *end = last;
        }
    }
    ids.resize(context, cfg.pad_id);
    Ok(ids)
}

/// The first `F32` output of a network as a unit vector of `dim` values.
fn vector(out: Vec<Tensor>, dim: usize) -> Result<Vec<f32>, String> {
    let first = out.into_iter().next().ok_or("the network gave no output")?;
    let mut v = first.floats().ok_or("the network gave no numbers")?.to_vec();
    if v.len() != dim {
        return Err(format!("the network gave {} values where {dim} are expected", v.len()));
    }
    if v.iter().any(|x| !x.is_finite()) {
        return Err("the network gave values that are not numbers".into());
    }
    normalise(&mut v);
    Ok(v)
}

/// A search pack on this computer: its networks, loaded when first needed.
pub struct Model {
    pub pack_id: String,
    pub config: Config,
    dir: PathBuf,
    image: OnceLock<Result<Network, String>>,
    text: OnceLock<Result<(Network, tokenizers::Tokenizer), String>>,
}

impl Model {
    /// The pack's model in the models folder `dir` (nothing is read yet). None when the pack
    /// is not a search pack.
    pub fn new(pack: &Pack, dir: &Path) -> Option<Model> {
        let config = pack.search.clone().filter(|c| c.valid(pack))?;
        Some(Model { pack_id: pack.id.clone(), config, dir: dir.to_path_buf(), image: OnceLock::new(), text: OnceLock::new() })
    }

    fn image_net(&self) -> Result<&Network, String> {
        self.image.get_or_init(|| Network::load_threads(&self.dir.join(&self.config.image), 2)).as_ref().map_err(String::clone)
    }

    fn text_net(&self) -> Result<&(Network, tokenizers::Tokenizer), String> {
        self.text
            .get_or_init(|| {
                let tok = tokenizers::Tokenizer::from_file(self.dir.join(&self.config.tokenizer)).map_err(|e| format!("the tokenizer of {} can't be read: {e}", self.pack_id))?;
                Ok((Network::load(&self.dir.join(&self.config.text))?, tok))
            })
            .as_ref()
            .map_err(String::clone)
    }

    /// The text encoder is loaded (a search was made).
    pub fn text_loaded(&self) -> bool {
        self.text.get().is_some_and(Result::is_ok)
    }

    /// The vector of a picture (unedited, the right way up).
    pub fn encode_image(&self, picture: &image::RgbaImage) -> Result<Vec<f32>, String> {
        let net = self.image_net()?;
        let name = net.inputs().first().map(|p| p.name.clone()).ok_or("the image network has no input")?;
        vector(net.run(&[(&name, pixels(picture, &self.config))])?, self.config.dim)
    }

    /// The vector of a sentence, put into the pack's prompt ("a photo of {}").
    pub fn encode_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let (net, tok) = self.text_net()?;
        let ids = tokens(tok, text, &self.config)?;
        let context = ids.len();
        let mask: Vec<i64> = ids.iter().map(|&i| (i != self.config.pad_id) as i64).collect();
        let mut inputs = vec![];
        for port in net.inputs() {
            let data = if port.name.contains("mask") { mask.clone() } else { ids.clone() };
            inputs.push((port.name.as_str(), Tensor::I64 { shape: vec![1, context], data }));
        }
        if inputs.is_empty() {
            return Err("the text network has no input".into());
        }
        vector(net.run(&inputs)?, self.config.dim)
    }
}

impl Analyser for Model {
    fn table(&self) -> Spec {
        self.config.table()
    }

    fn input_dim(&self) -> u32 {
        self.config.size
    }

    fn analyse(&self, picture: &image::RgbaImage) -> Result<Vec<Vec<u8>>, String> {
        Ok(vec![quantise(&self.encode_image(picture)?)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelFile;

    fn config() -> Config {
        Config {
            model: "test-1".into(),
            image: "i.onnx".into(),
            text: "t.onnx".into(),
            tokenizer: "tok.json".into(),
            dim: 4,
            size: 16,
            squash: false,
            mean: [0.5; 3],
            std: [0.25; 3],
            context: 6,
            pad_id: 0,
            prompt: "a photo of {}".into(),
            lowercase: true,
            no_punctuation: false,
            multilingual: false,
            min_score: 0.0,
            text_margin: 0.05,
        }
    }

    fn pack(search: Option<Config>) -> Pack {
        let file = |name: &str| ModelFile { name: name.into(), size: 1, sha256: "0".repeat(64) };
        Pack {
            id: "search-test".into(),
            title: "T".into(),
            about: String::new(),
            licence: "MIT".into(),
            version: 1,
            release: "models-1".into(),
            files: vec![file("i.onnx"), file("t.onnx"), file("tok.json")],
            search,
        }
    }

    /// A word-level tokenizer that knows six words and adds a start and an end mark.
    fn tokenizer() -> tokenizers::Tokenizer {
        let json = r#"{
          "version": "1.0", "truncation": null, "padding": null, "added_tokens": [],
          "normalizer": null,
          "pre_tokenizer": {"type": "Whitespace"},
          "post_processor": {"type": "TemplateProcessing",
             "single": [{"SpecialToken": {"id": "<s>", "type_id": 0}}, {"Sequence": {"id": "A", "type_id": 0}}, {"SpecialToken": {"id": "</s>", "type_id": 0}}],
             "pair": [{"Sequence": {"id": "A", "type_id": 0}}],
             "special_tokens": {"<s>": {"id": "<s>", "ids": [1], "tokens": ["<s>"]}, "</s>": {"id": "</s>", "ids": [2], "tokens": ["</s>"]}}},
          "decoder": null,
          "model": {"type": "WordLevel", "vocab": {"<pad>": 0, "<s>": 1, "</s>": 2, "<unk>": 3, "a": 4, "photo": 5, "of": 6, "red": 7, "car": 8, "snow": 9}, "unk_token": "<unk>"}
        }"#;
        tokenizers::Tokenizer::from_bytes(json.as_bytes()).unwrap()
    }

    #[test]
    fn a_pack_is_a_search_pack_only_with_a_sound_config() {
        let p = pack(Some(config()));
        assert!(config().valid(&p));
        assert!(Model::new(&p, Path::new("/nowhere")).is_some());
        assert!(Model::new(&pack(None), Path::new("/nowhere")).is_none());
        for bad in [
            Config { image: "other.onnx".into(), ..config() },
            Config { model: "Bad Name".into(), ..config() },
            Config { dim: 0, ..config() },
            Config { std: [0.0, 1.0, 1.0], ..config() },
            Config { prompt: "no place for the text".into(), ..config() },
            Config { context: 100_000, ..config() },
        ] {
            assert!(!bad.valid(&p), "{bad:?}");
        }
        assert_eq!(config().table(), Spec { kind: "embeddings".into(), model: "test-1".into(), record_len: 8 });
    }

    #[test]
    fn sentences_are_put_in_the_prompt_padded_and_cut() {
        let tok = tokenizer();
        // "a photo of Red car" → <s> a photo of red car </s>: 7 words, cut to 6 keeping the end
        assert_eq!(tokens(&tok, "Red car", &config()).unwrap(), [1, 4, 5, 6, 7, 2]);
        assert_eq!(tokens(&tok, "  snow ", &Config { context: 8, ..config() }).unwrap(), [1, 4, 5, 6, 9, 2, 0, 0]);
        assert_eq!(tokens(&tok, "snow", &Config { prompt: "{}".into(), ..config() }).unwrap(), [1, 9, 2, 0, 0, 0]);
        // case is kept when the network wants it
        assert_eq!(tokens(&tok, "Snow", &Config { prompt: "{}".into(), lowercase: false, ..config() }).unwrap(), [1, 3, 2, 0, 0, 0]);
        assert_eq!(tokens(&tok, "", &Config { prompt: "{}".into(), ..config() }).unwrap(), [1, 2, 0, 0, 0, 0]);
        // punctuation left out (when the network was trained so)
        assert_eq!(tokens(&tok, "snow!", &Config { prompt: "{}".into(), no_punctuation: true, ..config() }).unwrap(), tokens(&tok, "snow", &Config { prompt: "{}".into(), ..config() }).unwrap());
    }

    #[test]
    fn pictures_are_cut_to_the_middle_square_and_scaled() {
        // 32 × 16: left quarter black, the middle white, right quarter black
        let mut pic = image::RgbaImage::from_pixel(32, 16, image::Rgba([0, 0, 0, 255]));
        for y in 0..16 {
            for x in 8..24 {
                pic.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        let cfg = Config { size: 8, ..config() };
        let t = pixels(&pic, &cfg);
        assert_eq!(t.shape(), [1, 3, 8, 8]);
        // the middle square is white: (1 - 0.5) / 0.25 = 2 (its edge columns blend a little
        // with the black beside it when scaled)
        let v = t.floats().unwrap();
        assert!((0..3 * 8).all(|row| (1..7).all(|x| (v[row * 8 + x] - 2.0).abs() < 1e-3)), "{v:?}");
        assert!(v.iter().all(|&x| x > 1.0), "nothing of the black sides is kept whole");
        // any shape, even one pixel
        let tiny = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        assert_eq!(pixels(&tiny, &cfg).shape(), [1, 3, 8, 8]);
        // squashed: the black sides are kept, as the two outer columns of each side
        let v = pixels(&pic, &Config { squash: true, ..cfg }).floats().unwrap().to_vec();
        assert!((0..3 * 8).all(|row| v[row * 8] < -1.9 && v[row * 8 + 7] < -1.9 && (v[row * 8 + 4] - 2.0).abs() < 1e-3), "{v:?}");
    }

    #[test]
    fn kept_vectors_rank_like_full_ones() {
        let mut a = vec![0.9, 0.1, -0.3, 0.2];
        normalise(&mut a);
        let r = quantise(&a);
        assert_eq!(r.len(), record_len(4));
        let back = dequantise(&r, 4).unwrap();
        assert!(a.iter().zip(&back).all(|(x, y)| (x - y).abs() < 0.01));
        assert!((score(&a, &r) - 1.0).abs() < 0.01);
        let mut b = vec![-0.2, 0.8, 0.1, 0.1];
        normalise(&mut b);
        assert!(score(&a, &r) > score(&b, &r));
        // zeros, a wrong length, not numbers: never a crash
        assert!(score(&a, &quantise(&[0.0; 4])) == 0.0);
        assert!(score(&a, &r[..5]) == f32::NEG_INFINITY);
        assert!(dequantise(&r[..5], 4).is_none());
        assert_eq!(quantise(&[f32::NAN, 1.0])[4..], [0, 127]);
    }

    /// A real search pack, exported to `ALBUMCRAFT_SEARCH_PACK=<folder>` with a
    /// `reference.json` of vectors computed by the original model, and the files named
    /// image.onnx, text.onnx, tokenizer.json.
    #[test]
    #[ignore]
    fn real_pack_matches_its_reference() {
        let Ok(dir) = std::env::var("ALBUMCRAFT_SEARCH_PACK") else { return };
        let dir = PathBuf::from(dir);
        let reference: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("reference.json")).unwrap()).unwrap();
        let size = reference["size"].as_u64().unwrap() as u32;
        let three = |v: &serde_json::Value| -> [f32; 3] { std::array::from_fn(|i| v[i].as_f64().unwrap() as f32) };
        let context = reference["context"].as_u64().unwrap() as usize;
        let dim = reference["image_vector"].as_array().unwrap().len();
        let cfg = Config { model: "real-1".into(), image: "image.onnx".into(), text: "text.onnx".into(), tokenizer: "tokenizer.json".into(), dim, size, squash: false, mean: three(&reference["mean"]), std: three(&reference["std"]), context, pad_id: 0, prompt: "{}".into(), lowercase: true, no_punctuation: false, multilingual: false, min_score: 0.0, text_margin: 0.05 };
        let model = Model { pack_id: "real".into(), config: cfg.clone(), dir: dir.clone(), image: OnceLock::new(), text: OnceLock::new() };
        let floats = |v: &serde_json::Value| v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect::<Vec<f32>>();
        for (i, text) in reference["texts"].as_array().unwrap().iter().enumerate() {
            let (_, tok) = model.text_net().unwrap();
            let want: Vec<i64> = reference["tokens"][i].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
            assert_eq!(tokens(tok, text.as_str().unwrap(), &cfg).unwrap(), want, "{text}");
            let v = model.encode_text(text.as_str().unwrap()).unwrap();
            let r = floats(&reference["text_vectors"][i]);
            let cos: f32 = v.iter().zip(&r).map(|(a, b)| a * b).sum();
            assert!(cos > 0.999, "{text}: {cos}");
        }
        // the reference picture is given as the network takes it, so it is checked through run()
        let net = model.image_net().unwrap();
        let mut rng_free = vec![];
        // numpy's default_rng(1).random cannot be made here: the vector was saved with its input
        if let Some(px) = reference.get("pixels") {
            rng_free = floats(px);
        }
        if !rng_free.is_empty() {
            let s = size as usize;
            let out = vector(net.run(&[("pixels", Tensor::F32 { shape: vec![1, 3, s, s], data: rng_free })]).unwrap(), dim).unwrap();
            let r = floats(&reference["image_vector"]);
            let cos: f32 = out.iter().zip(&r).map(|(a, b)| a * b).sum();
            assert!(cos > 0.999, "image: {cos}");
        }
        let start = std::time::Instant::now();
        let pic = image::RgbaImage::from_fn(256, 170, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]));
        for _ in 0..10 {
            model.analyse(&pic).unwrap();
        }
        eprintln!("image encoder: {:?} per picture", start.elapsed() / 10);
        let start = std::time::Instant::now();
        for _ in 0..10 {
            model.encode_text("red car in the snow").unwrap();
        }
        eprintln!("text encoder: {:?} per sentence", start.elapsed() / 10);
    }
}
