//! Faces: found by YuNet (built into the program), each described by a vector from SFace (the
//! downloadable *Faces* pack), so that faces of one person can be grouped.
//!
//! What is kept per face (`FaceRecord`, one record each in the `faces` table): where it is (box
//! and five landmarks, as fractions of the picture: unedited and turned the right way up), how
//! sure the detector was, how usable the face is (size, sharpness, how far it is turned) and,
//! with the Faces pack, its vector. Without the pack the faces are still found (a table of its
//! own, `DETECT_ONLY`), which is enough for "with people" kinds of filters; once the pack is
//! there the pictures are looked at again for the table with vectors.
//!
//! The networks' inputs and outputs, and the alignment, follow OpenCV's `FaceDetectorYN` and
//! `FaceRecognizerSF`, which the published networks were made for (checked against them on
//! real pictures: `real_faces_match_opencv`).

use crate::models::Pack;
use crate::runtime::{Network, Tensor};
use crate::search::{dequantise, normalise, quantise};
use crate::store::Spec;
use crate::Analyser;
use image::RgbaImage;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The detector, built into the program (`networks/README.md`).
pub const DETECTOR: &[u8] = include_bytes!("../networks/yunet-2023mar-1.onnx");
/// Its name in the tables.
const DETECTOR_MODEL: &str = "yunet-1";
/// The longest side of the picture the analyser asks for (the 1400 px preview size): faces are
/// cut out of it.
pub const INPUT_DIM: u32 = 1400;
/// The longest side the detector looks at (19 ms on one core; faces down to about 10 px).
pub const DETECT_DIM: u32 = 1024;
/// Detections below this are not faces (OpenCV's demo uses 0.9; lower ones are kept, marked by
/// their score, and the grouping decides what to do with them).
pub const MIN_SCORE: f32 = 0.6;
const NMS_IOU: f32 = 0.3;
/// At most this many faces per picture (the surest ones): a crowd is not a family album.
pub const MAX_FACES: usize = 64;
/// Values of a face vector.
pub const DIM: usize = 128;
/// The side of the aligned face given to the recogniser.
pub const ALIGNED: u32 = 112;
/// Where the eyes, nose and mouth corners go in the aligned face (the ArcFace layout, which
/// SFace uses).
const TEMPLATE: [[f32; 2]; 5] = [[38.2946, 51.6963], [73.5318, 51.5014], [56.0252, 71.7366], [41.5493, 92.3655], [70.7299, 92.2041]];
/// The eyes are this far apart in the aligned face: a face smaller than that in the picture
/// is cut from the full-size picture when it is larger.
const TEMPLATE_EYES: f32 = 35.2;
/// Faces whose eyes are closer than this in the best picture there is get no vector: too
/// small to tell people apart.
pub const MIN_EYES_FOR_VECTOR: f32 = 8.0;

/// The recogniser of a pack (the pack's `faces` section).
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// The network and its version, the name of the table of faces with vectors made with it.
    pub model: String,
    /// The network file, one of the pack's files.
    pub recogniser: String,
    /// Two faces this close (cosine) are one person, to start with.
    #[serde(default = "default_same")]
    pub same_person: f32,
}

fn default_same() -> f32 {
    0.45
}

impl Config {
    pub fn valid(&self, pack: &Pack) -> bool {
        let name_ok = !self.model.is_empty() && self.model.len() <= 32 && self.model.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        name_ok && pack.files.iter().any(|f| f.name == self.recogniser) && (0.0..1.0).contains(&self.same_person)
    }
}

/// A face as the detector gives it, in pixels of the picture it looked at.
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    /// Left, top, width, height.
    pub rect: [f32; 4],
    /// Right eye, left eye (as seen: the one on the left of the picture first), nose tip, mouth
    /// corners (left of the picture first).
    pub landmarks: [[f32; 2]; 5],
    pub score: f32,
}

impl Detection {
    fn scaled(&self, k: f32) -> Detection {
        Detection { rect: self.rect.map(|v| v * k), landmarks: self.landmarks.map(|[x, y]| [x * k, y * k]), score: self.score }
    }

    /// The distance between the eyes, in pixels.
    pub fn eyes(&self) -> f32 {
        let [a, b] = [self.landmarks[0], self.landmarks[1]];
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
    }
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let (x1, y1) = (a[0].max(b[0]), a[1].max(b[1]));
    let (x2, y2) = ((a[0] + a[2]).min(b[0] + b[2]), (a[1] + a[3]).min(b[1] + b[3]));
    let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let union = a[2] * a[3] + b[2] * b[3] - inter;
    if union > 0.0 { inter / union } else { 0.0 }
}

/// The faces in YuNet's outputs (`cls_8` … `kps_32`, given by name) for an input of
/// `width` × `height` (multiples of 32): scored, overlapping ones removed, surest first.
pub fn decode(output: &dyn Fn(&str) -> Option<Vec<f32>>, width: usize, height: usize, min_score: f32) -> Result<Vec<Detection>, String> {
    let mut found = vec![];
    for stride in [8usize, 16, 32] {
        let (cols, rows) = (width / stride, height / stride);
        let cells = cols * rows;
        let get = |kind: &str, per: usize| -> Result<Vec<f32>, String> {
            let v = output(&format!("{kind}_{stride}")).ok_or_else(|| format!("the detector gave no {kind}_{stride}"))?;
            if v.len() != cells * per {
                return Err(format!("the detector's {kind}_{stride} has {} values, not {}", v.len(), cells * per));
            }
            Ok(v)
        };
        let (cls, obj, bbox, kps) = (get("cls", 1)?, get("obj", 1)?, get("bbox", 4)?, get("kps", 10)?);
        let s = stride as f32;
        for i in 0..cells {
            let score = (cls[i].clamp(0.0, 1.0) * obj[i].clamp(0.0, 1.0)).sqrt();
            if score.is_nan() || score < min_score {
                continue;
            }
            let (r, c) = ((i / cols) as f32, (i % cols) as f32);
            let b = &bbox[i * 4..i * 4 + 4];
            let (cx, cy) = ((c + b[0]) * s, (r + b[1]) * s);
            let (w, h) = (b[2].exp() * s, b[3].exp() * s);
            let k = &kps[i * 10..i * 10 + 10];
            let landmarks = std::array::from_fn(|n| [(c + k[2 * n]) * s, (r + k[2 * n + 1]) * s]);
            let rect = [cx - w / 2.0, cy - h / 2.0, w, h];
            if rect.iter().chain(landmarks.iter().flatten()).all(|v| v.is_finite()) && w > 0.0 && h > 0.0 {
                found.push(Detection { rect, landmarks, score });
            }
        }
    }
    found.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<Detection> = vec![];
    for d in found {
        if kept.iter().all(|k| iou(&k.rect, &d.rect) < NMS_IOU) {
            kept.push(d);
        }
    }
    Ok(kept)
}

/// The detector's input: the picture as blue, green, red planes of 0…255 values, padded with
/// black to multiples of 32 (OpenCV's way).
pub fn detector_input(picture: &RgbaImage) -> (Tensor, usize, usize) {
    let (w, h) = (picture.width() as usize, picture.height() as usize);
    let (pw, ph) = (w.div_ceil(32).max(1) * 32, h.div_ceil(32).max(1) * 32);
    let mut data = vec![0f32; 3 * pw * ph];
    for (x, y, p) in picture.enumerate_pixels() {
        let i = y as usize * pw + x as usize;
        data[i] = p[2] as f32;
        data[pw * ph + i] = p[1] as f32;
        data[2 * pw * ph + i] = p[0] as f32;
    }
    (Tensor::F32 { shape: vec![1, 3, ph, pw], data }, pw, ph)
}

/// The similarity (turn, scale, shift; no mirroring) that best takes `from` to `to`, least
/// squares: `[a, -b, tx; b, a, ty]` as `[a, b, tx, ty]`.
pub fn similarity(from: &[[f32; 2]; 5], to: &[[f32; 2]; 5]) -> Option<[f32; 4]> {
    let mean = |p: &[[f32; 2]; 5]| {
        let (x, y) = p.iter().fold((0.0, 0.0), |(x, y), q| (x + q[0], y + q[1]));
        [x / 5.0, y / 5.0]
    };
    let (mf, mt) = (mean(from), mean(to));
    let (mut dot, mut cross, mut var) = (0f32, 0f32, 0f32);
    for (f, t) in from.iter().zip(to) {
        let (fx, fy, tx, ty) = (f[0] - mf[0], f[1] - mf[1], t[0] - mt[0], t[1] - mt[1]);
        dot += fx * tx + fy * ty;
        cross += fx * ty - fy * tx;
        var += fx * fx + fy * fy;
    }
    if var.is_nan() || var <= 1e-6 {
        return None;
    }
    let (a, b) = (dot / var, cross / var);
    Some([a, b, mt[0] - (a * mf[0] - b * mf[1]), mt[1] - (b * mf[0] + a * mf[1])])
}

/// The face turned and scaled so that its landmarks sit on the template: `ALIGNED` × `ALIGNED`,
/// bilinear, black outside the picture (OpenCV's `alignCrop`).
pub fn align(picture: &RgbaImage, landmarks: &[[f32; 2]; 5]) -> Option<image::RgbImage> {
    let [a, b, tx, ty] = similarity(landmarks, &TEMPLATE)?;
    // the inverse: picture point of each aligned pixel
    let det = a * a + b * b;
    let (ia, ib) = (a / det, -b / det);
    let (w, h) = (picture.width() as i64, picture.height() as i64);
    let px = |x: i64, y: i64| -> [f32; 3] {
        if x < 0 || y < 0 || x >= w || y >= h {
            return [0.0; 3];
        }
        let p = picture.get_pixel(x as u32, y as u32);
        [p[0] as f32, p[1] as f32, p[2] as f32]
    };
    let mut out = image::RgbImage::new(ALIGNED, ALIGNED);
    for (u, v, o) in out.enumerate_pixels_mut() {
        let (du, dv) = (u as f32 - tx, v as f32 - ty);
        let (sx, sy) = (ia * du - ib * dv, ib * du + ia * dv);
        let (x0, y0) = (sx.floor(), sy.floor());
        let (fx, fy) = (sx - x0, sy - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (p00, p10, p01, p11) = (px(x0, y0), px(x0 + 1, y0), px(x0, y0 + 1), px(x0 + 1, y0 + 1));
        for c in 0..3 {
            let top = p00[c] + (p10[c] - p00[c]) * fx;
            let bottom = p01[c] + (p11[c] - p01[c]) * fx;
            o[c] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
        }
    }
    Some(out)
}

/// How sharp an aligned face is: the variance of its brightness's Laplacian, inside the face
/// (blurred or out-of-focus faces score low).
pub fn sharpness(face: &image::RgbImage) -> f32 {
    let (w, h) = (face.width() as i64, face.height() as i64);
    let lum = |x: i64, y: i64| {
        let p = face.get_pixel(x as u32, y as u32);
        0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
    };
    let (mut sum, mut sq, mut n) = (0f64, 0f64, 0f64);
    // the middle of the face: eyes to mouth, without the black corners alignment can leave
    for y in (h / 4)..(h * 7 / 8) {
        for x in (w / 6)..(w * 5 / 6) {
            let l = (lum(x - 1, y) + lum(x + 1, y) + lum(x, y - 1) + lum(x, y + 1) - 4.0 * lum(x, y)) as f64;
            sum += l;
            sq += l * l;
            n += 1.0;
        }
    }
    if n == 0.0 { 0.0 } else { (sq / n - (sum / n).powi(2)).max(0.0) as f32 }
}

/// How far the face is turned sideways, from the landmarks: 0 facing the camera, about ±0.5
/// in profile (where the nose is between the eyes, along the line of the eyes, in eye distances
/// from their middle).
pub fn yaw(landmarks: &[[f32; 2]; 5]) -> f32 {
    let [e1, e2, nose] = [landmarks[0], landmarks[1], landmarks[2]];
    let (dx, dy) = (e2[0] - e1[0], e2[1] - e1[1]);
    let d2 = dx * dx + dy * dy;
    if d2.is_nan() || d2 <= 0.0 {
        return 1.0;
    }
    let mid = [(e1[0] + e2[0]) / 2.0, (e1[1] + e2[1]) / 2.0];
    ((nose[0] - mid[0]) * dx + (nose[1] - mid[1]) * dy) / d2
}

/// One face as kept.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceRecord {
    /// Left, top, width, height as fractions of the picture.
    pub rect: [f32; 4],
    /// As in `Detection`, fractions of the picture.
    pub landmarks: [[f32; 2]; 5],
    pub score: f32,
    /// The distance between the eyes in the largest picture looked at, in pixels.
    pub eyes: f32,
    pub sharpness: f32,
    pub yaw: f32,
    /// The face was cut from the full-size picture (it was small in the preview).
    pub from_full: bool,
    /// Unit length; none without the Faces pack or for a face too small.
    pub vector: Option<Vec<f32>>,
}

const HAS_VECTOR: u32 = 1;
const FROM_FULL: u32 = 2;
/// Bytes of a kept face.
pub const RECORD_LEN: usize = 4 * 4 + 10 * 4 + 4 * 4 + 4 + crate::search::record_len(DIM);

impl FaceRecord {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(RECORD_LEN);
        let mut f = |v: f32| out.extend_from_slice(&v.to_le_bytes());
        self.rect.iter().copied().for_each(&mut f);
        self.landmarks.iter().flatten().copied().for_each(&mut f);
        f(self.score);
        f(self.eyes);
        f(self.sharpness);
        f(self.yaw);
        let flags = if self.vector.is_some() { HAS_VECTOR } else { 0 } | if self.from_full { FROM_FULL } else { 0 };
        out.extend_from_slice(&flags.to_le_bytes());
        match &self.vector {
            Some(v) if v.len() == DIM => out.extend(quantise(v)),
            _ => out.resize(RECORD_LEN, 0),
        }
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<FaceRecord> {
        if b.len() != RECORD_LEN {
            return None;
        }
        let f = |i: usize| f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
        let flags = u32::from_le_bytes(b[72..76].try_into().unwrap());
        let vector = if flags & HAS_VECTOR != 0 { Some(dequantise(&b[76..], DIM)?) } else { None };
        let r = FaceRecord {
            rect: [f(0), f(1), f(2), f(3)],
            landmarks: std::array::from_fn(|n| [f(4 + 2 * n), f(5 + 2 * n)]),
            score: f(14),
            eyes: f(15),
            sharpness: f(16),
            yaw: f(17),
            from_full: flags & FROM_FULL != 0,
            vector,
        };
        (r.rect.iter().chain(r.landmarks.iter().flatten()).all(|v| v.is_finite()) && r.score.is_finite()).then_some(r)
    }
}

/// The table of faces: with vectors of `recogniser`, or found only.
pub fn table(recogniser: Option<&Config>) -> Spec {
    let model = match recogniser {
        Some(c) => format!("{DETECTOR_MODEL}-{}", c.model),
        None => DETECTOR_MODEL.to_string(),
    };
    Spec { kind: "faces".into(), model, record_len: RECORD_LEN }
}

/// The table of faces found without vectors.
pub fn detect_only() -> Spec {
    table(None)
}

/// The recogniser's input: red, green, blue planes of 0…255 values.
fn recogniser_input(face: &image::RgbImage) -> Tensor {
    let n = (ALIGNED * ALIGNED) as usize;
    let mut data = vec![0f32; 3 * n];
    for (i, p) in face.pixels().enumerate() {
        for c in 0..3 {
            data[c * n + i] = p[c] as f32;
        }
    }
    Tensor::F32 { shape: vec![1, 3, ALIGNED as usize, ALIGNED as usize], data }
}

/// Finding faces and, with a recogniser, describing them: what the background analysis runs.
pub struct FaceAnalyser {
    recogniser: Option<(Config, PathBuf)>,
    detector: OnceLock<Result<Network, String>>,
    describer: OnceLock<Result<Network, String>>,
}

impl FaceAnalyser {
    /// With `pack` (a Faces pack in the models folder `dir`), faces get vectors.
    pub fn new(pack: Option<(&Pack, &Path)>) -> FaceAnalyser {
        let recogniser = pack.and_then(|(p, dir)| {
            let cfg = p.faces.clone().filter(|c| c.valid(p))?;
            let file = dir.join(&cfg.recogniser);
            Some((cfg, file))
        });
        FaceAnalyser { recogniser, detector: OnceLock::new(), describer: OnceLock::new() }
    }

    pub fn recogniser(&self) -> Option<&Config> {
        self.recogniser.as_ref().map(|(c, _)| c)
    }

    fn detector(&self) -> Result<&Network, String> {
        self.detector.get_or_init(|| Network::load_bytes("YuNet", DETECTOR, 2)).as_ref().map_err(String::clone)
    }

    fn describer(&self) -> Option<Result<&Network, String>> {
        let (_, file) = self.recogniser.as_ref()?;
        Some(self.describer.get_or_init(|| Network::load_threads(file, 2)).as_ref().map_err(String::clone))
    }

    /// The faces of a picture, in its pixels, surest first.
    pub fn detect(&self, picture: &RgbaImage) -> Result<Vec<Detection>, String> {
        let net = self.detector()?;
        let (w, h) = picture.dimensions();
        if w < 16 || h < 16 {
            return Ok(vec![]);
        }
        let k = (DETECT_DIM as f32 / w.max(h) as f32).min(1.0);
        let small;
        let looked = if k < 1.0 {
            small = image::imageops::resize(picture, ((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1), image::imageops::FilterType::Triangle);
            &small
        } else {
            picture
        };
        let (input, pw, ph) = detector_input(looked);
        let name = net.inputs().first().map(|p| p.name.clone()).ok_or("the detector has no input")?;
        let out = net.run(&[(&name, input)])?;
        let named = |n: &str| net.outputs().iter().position(|p| p.name == n).and_then(|i| out.get(i)).and_then(|t| t.floats()).map(<[f32]>::to_vec);
        let mut found = decode(&named, pw, ph, MIN_SCORE)?;
        found.truncate(MAX_FACES);
        // back to the picture's pixels (sx / sy: the rounding of the smaller size)
        let (sx, sy) = (w as f32 / looked.width() as f32, h as f32 / looked.height() as f32);
        Ok(found
            .into_iter()
            .map(|d| Detection { rect: [d.rect[0] * sx, d.rect[1] * sy, d.rect[2] * sx, d.rect[3] * sy], landmarks: d.landmarks.map(|[x, y]| [x * sx, y * sy]), score: d.score })
            .collect())
    }

    /// The unit-length vector of an aligned face.
    pub fn describe(&self, face: &image::RgbImage) -> Result<Vec<f32>, String> {
        let net = self.describer().ok_or("no face recogniser")??;
        let name = net.inputs().first().map(|p| p.name.clone()).ok_or("the recogniser has no input")?;
        let out = net.run(&[(&name, recogniser_input(face))])?;
        let mut v = out.into_iter().next().and_then(|t| if let Tensor::F32 { data, .. } = t { Some(data) } else { None }).ok_or("the recogniser gave no vector")?;
        if v.len() != DIM || !v.iter().all(|x| x.is_finite()) {
            return Err(format!("the recogniser gave {} values, not {DIM}", v.len()));
        }
        normalise(&mut v);
        Ok(v)
    }

    /// What is kept of the faces of `picture`; `full` gives the picture at its full size, asked
    /// for only when a face is small in `picture`.
    pub fn faces(&self, picture: &RgbaImage, full: &dyn Fn() -> Option<RgbaImage>) -> Result<Vec<FaceRecord>, String> {
        let found = self.detect(picture)?;
        let (w, h) = (picture.width() as f32, picture.height() as f32);
        let mut larger: Option<Option<(RgbaImage, f32)>> = None;
        let mut out = Vec::with_capacity(found.len());
        for d in found {
            let mut src: (&RgbaImage, Detection) = (picture, d.clone());
            let mut from_full = false;
            if self.recogniser.is_some() && d.eyes() < TEMPLATE_EYES {
                let big = larger.get_or_insert_with(|| full().filter(|f| f.width() > picture.width()).map(|f| {
                    let k = f.width() as f32 / w;
                    (f, k)
                }));
                if let Some((f, k)) = big {
                    src = (f, d.scaled(*k));
                    from_full = true;
                }
            }
            let face = align(src.0, &src.1.landmarks);
            let eyes = src.1.eyes();
            let vector = match (&face, self.recogniser.is_some() && eyes >= MIN_EYES_FOR_VECTOR) {
                (Some(f), true) => Some(self.describe(f)?),
                _ => None,
            };
            out.push(FaceRecord {
                rect: [d.rect[0] / w, d.rect[1] / h, d.rect[2] / w, d.rect[3] / h],
                landmarks: d.landmarks.map(|[x, y]| [x / w, y / h]),
                score: d.score,
                eyes,
                sharpness: face.as_ref().map(sharpness).unwrap_or(0.0),
                yaw: yaw(&d.landmarks),
                from_full,
                vector,
            });
        }
        Ok(out)
    }
}

impl Analyser for FaceAnalyser {
    fn table(&self) -> Spec {
        table(self.recogniser())
    }

    fn input_dim(&self) -> u32 {
        INPUT_DIM
    }

    fn analyse(&self, picture: &RgbaImage) -> Result<Vec<Vec<u8>>, String> {
        self.analyse_full(picture, &|| None)
    }

    fn analyse_full(&self, picture: &RgbaImage, full: &dyn Fn() -> Option<RgbaImage>) -> Result<Vec<Vec<u8>>, String> {
        Ok(self.faces(picture, full)?.iter().map(FaceRecord::to_bytes).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// YuNet outputs with nothing in them, for an input of `w` × `h`.
    fn empty_outputs(w: usize, h: usize) -> std::collections::HashMap<String, Vec<f32>> {
        let mut m = std::collections::HashMap::new();
        for s in [8, 16, 32] {
            let cells = (w / s) * (h / s);
            m.insert(format!("cls_{s}"), vec![0.0; cells]);
            m.insert(format!("obj_{s}"), vec![0.0; cells]);
            m.insert(format!("bbox_{s}"), vec![0.0; cells * 4]);
            m.insert(format!("kps_{s}"), vec![0.0; cells * 10]);
        }
        m
    }

    fn put(m: &mut std::collections::HashMap<String, Vec<f32>>, stride: usize, w: usize, row: usize, col: usize, score: f32, bbox: [f32; 4]) {
        let i = row * (w / stride) + col;
        m.get_mut(&format!("cls_{stride}")).unwrap()[i] = score;
        m.get_mut(&format!("obj_{stride}")).unwrap()[i] = score;
        m.get_mut(&format!("bbox_{stride}")).unwrap()[i * 4..i * 4 + 4].copy_from_slice(&bbox);
        let kps: Vec<f32> = (0..10).map(|n| n as f32 * 0.1).collect();
        m.get_mut(&format!("kps_{stride}")).unwrap()[i * 10..i * 10 + 10].copy_from_slice(&kps);
    }

    #[test]
    fn detections_are_decoded_scored_and_overlaps_removed() {
        let (w, h) = (64, 32);
        let mut m = empty_outputs(w, h);
        // stride 8, cell (row 1, col 2): centre (2.5, 1.5) * 8 = (20, 12), size e^0 * 8 = 8
        put(&mut m, 8, w, 1, 2, 0.9, [0.5, 0.5, 0.0, 0.0]);
        // the same face seen again a little less surely, and a weak one
        put(&mut m, 8, w, 1, 3, 0.8, [-0.4, 0.5, 0.0, 0.0]);
        put(&mut m, 16, w, 0, 3, 0.5, [0.5, 0.5, 0.0, 0.0]);
        // a second face elsewhere, larger: stride 32, cell (0, 1), size e^1 * 32
        put(&mut m, 32, w, 0, 1, 0.7, [0.5, 0.5, 1.0, 1.0]);
        let get = |n: &str| m.get(n).cloned();
        let found = decode(&get, w, h, MIN_SCORE).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
        let a = &found[0];
        assert!((a.score - 0.9).abs() < 1e-6);
        assert!(a.rect.iter().zip([16.0, 8.0, 8.0, 8.0]).all(|(x, y)| (x - y).abs() < 1e-4), "{:?}", a.rect);
        assert!((a.landmarks[0][0] - 16.0).abs() < 1e-4 && (a.landmarks[0][1] - 8.8).abs() < 1e-4, "{:?}", a.landmarks);
        let b = &found[1];
        let side = 1f32.exp() * 32.0;
        assert!((b.rect[2] - side).abs() < 1e-3 && (b.rect[0] - (48.0 - side / 2.0)).abs() < 1e-3);
        // outputs of the wrong size or missing are errors
        let mut bad = m.clone();
        bad.get_mut("kps_16").unwrap().pop();
        assert!(decode(&|n| bad.get(n).cloned(), w, h, MIN_SCORE).is_err());
        assert!(decode(&|_| None, w, h, MIN_SCORE).is_err());
    }

    #[test]
    fn the_detector_input_is_bgr_and_padded() {
        let mut p = RgbaImage::new(33, 2);
        p.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
        let (t, w, h) = detector_input(&p);
        assert_eq!((w, h), (64, 32));
        assert_eq!(t.shape(), [1, 3, 32, 64]);
        let d = t.floats().unwrap();
        assert_eq!((d[0], d[64 * 32], d[2 * 64 * 32]), (30.0, 20.0, 10.0));
        assert_eq!(d[40], 0.0, "black beyond the picture");
    }

    #[test]
    fn the_similarity_is_found_exactly() {
        // the template turned by 30°, scaled by 2.5 and moved
        let (a, b) = (2.5 * 30f32.to_radians().cos(), 2.5 * 30f32.to_radians().sin());
        let moved = TEMPLATE.map(|[x, y]| [a * x - b * y + 40.0, b * x + a * y - 7.0]);
        let back = similarity(&moved, &TEMPLATE).unwrap();
        for (m, t) in moved.iter().zip(TEMPLATE) {
            let x = back[0] * m[0] - back[1] * m[1] + back[2];
            let y = back[1] * m[0] + back[0] * m[1] + back[3];
            assert!((x - t[0]).abs() < 1e-3 && (y - t[1]).abs() < 1e-3);
        }
        assert!(similarity(&[[1.0, 1.0]; 5], &TEMPLATE).is_none(), "all points in one place");
    }

    #[test]
    fn aligning_puts_the_landmarks_on_the_template() {
        // a picture with a bright dot on each landmark of a face twice the template's size,
        // turned a little
        let (a, b) = (2.0 * 10f32.to_radians().cos(), 2.0 * 10f32.to_radians().sin());
        let marks = TEMPLATE.map(|[x, y]| [a * x - b * y + 60.0, b * x + a * y + 20.0]);
        let mut p = RgbaImage::from_pixel(400, 400, image::Rgba([0, 0, 0, 255]));
        for [x, y] in marks {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    p.put_pixel((x.round() as i32 + dx) as u32, (y.round() as i32 + dy) as u32, image::Rgba([255, 255, 255, 255]));
                }
            }
        }
        let face = align(&p, &marks).unwrap();
        assert_eq!(face.dimensions(), (ALIGNED, ALIGNED));
        for [x, y] in TEMPLATE {
            assert!(face.get_pixel(x.round() as u32, y.round() as u32)[0] > 200, "landmark at {x},{y}");
        }
        assert!(face.get_pixel(5, 5)[0] < 20 && face.get_pixel(56, 30)[0] < 20);
    }

    #[test]
    fn sharp_faces_score_higher_and_turned_faces_show_it() {
        let mut sharp = image::RgbImage::new(ALIGNED, ALIGNED);
        for (x, y, p) in sharp.enumerate_pixels_mut() {
            let v = if (x / 2 + y / 2) % 2 == 0 { 230 } else { 20 };
            *p = image::Rgb([v, v, v]);
        }
        let blurred = image::imageops::blur(&sharp, 3.0);
        let flat = image::RgbImage::from_pixel(ALIGNED, ALIGNED, image::Rgb([128, 128, 128]));
        assert!(sharpness(&sharp) > 10.0 * sharpness(&blurred));
        assert_eq!(sharpness(&flat), 0.0);
        assert!(yaw(&TEMPLATE).abs() < 0.02, "facing the camera");
        let mut turned = TEMPLATE;
        turned[2][0] = TEMPLATE[1][0];
        assert!(yaw(&turned) > 0.4, "nose under one eye: in profile");
    }

    #[test]
    fn records_keep_what_was_found() {
        let mut v: Vec<f32> = (0..DIM).map(|i| (i as f32 - 60.0) / 100.0).collect();
        normalise(&mut v);
        let r = FaceRecord {
            rect: [0.1, 0.2, 0.05, 0.07],
            landmarks: std::array::from_fn(|n| [n as f32 * 0.01, 0.5]),
            score: 0.93,
            eyes: 41.5,
            sharpness: 120.0,
            yaw: -0.1,
            from_full: true,
            vector: Some(v.clone()),
        };
        let b = r.to_bytes();
        assert_eq!(b.len(), RECORD_LEN);
        let back = FaceRecord::from_bytes(&b).unwrap();
        assert_eq!((back.rect, back.landmarks, back.score, back.eyes, back.from_full), (r.rect, r.landmarks, r.score, r.eyes, true));
        let cos: f32 = back.vector.unwrap().iter().zip(&v).map(|(a, b)| a * b).sum();
        assert!(cos > 0.999);
        let none = FaceRecord { vector: None, from_full: false, ..r };
        let back = FaceRecord::from_bytes(&none.to_bytes()).unwrap();
        assert!(back.vector.is_none() && !back.from_full);
        assert!(FaceRecord::from_bytes(&b[1..]).is_none());
        assert_eq!(table(None).model, "yunet-1");
        let cfg = Config { model: "sface-1".into(), recogniser: "s.onnx".into(), same_person: 0.45 };
        assert_eq!(table(Some(&cfg)).model, "yunet-1-sface-1");
        assert_eq!(table(Some(&cfg)).record_len, RECORD_LEN);
    }

    /// The built-in detector runs and finds nothing in an empty picture.
    #[test]
    fn the_built_in_detector_runs() {
        if !crate::runtime::available() {
            return;
        }
        let a = FaceAnalyser::new(None);
        let blank = RgbaImage::from_pixel(1400, 900, image::Rgba([128, 128, 128, 255]));
        assert!(a.detect(&blank).unwrap().is_empty());
        assert!(a.analyse(&blank).unwrap().is_empty());
        assert!(a.analyse(&RgbaImage::new(4, 4)).unwrap().is_empty(), "too small to look at");
        assert_eq!(a.table(), detect_only());
    }

    /// Against OpenCV's own face classes: `networks/reference_faces.py` writes, for pictures of
    /// a folder, what `FaceDetectorYN` and `FaceRecognizerSF` find
    /// (`ALBUMCRAFT_FACES_REFERENCE=<that file> ALBUMCRAFT_FACES_PACK=<folder with
    /// sface-1.onnx> cargo test --release -p albumcraft-ai real_faces -- --ignored
    /// --nocapture`).
    #[test]
    #[ignore]
    fn real_faces_match_opencv() {
        #[derive(Deserialize)]
        struct Picture {
            path: String,
            width: u32,
            height: u32,
            faces: Vec<RefFace>,
        }
        #[derive(Deserialize)]
        struct RefFace {
            rect: [f32; 4],
            landmarks: [[f32; 2]; 5],
            score: f32,
            vector: Vec<f32>,
        }
        let reference = std::env::var("ALBUMCRAFT_FACES_REFERENCE").expect("ALBUMCRAFT_FACES_REFERENCE");
        let pack = std::env::var("ALBUMCRAFT_FACES_PACK").expect("ALBUMCRAFT_FACES_PACK");
        let pictures: Vec<Picture> = serde_json::from_str(&std::fs::read_to_string(reference).unwrap()).unwrap();
        let cfg = Config { model: "sface-2021dec-1".into(), recogniser: "sface-1.onnx".into(), same_person: 0.45 };
        let a = FaceAnalyser { recogniser: Some((cfg, Path::new(&pack).join("sface-1.onnx"))), detector: OnceLock::new(), describer: OnceLock::new() };
        let (mut faces, mut worst_px, mut worst_cos, mut missed) = (0, 0f32, 1f32, 0);
        let started = std::time::Instant::now();
        for p in &pictures {
            // the same pixels OpenCV was given, and the detector alone at that size
            let img = image::open(&p.path).unwrap().to_rgba8();
            assert_eq!(img.dimensions(), (p.width, p.height));
            let net = a.detector().unwrap();
            let (input, pw, ph) = detector_input(&img);
            let out = net.run(&[("input", input)]).unwrap();
            let named = |n: &str| net.outputs().iter().position(|q| q.name == n).and_then(|i| out[i].floats()).map(<[f32]>::to_vec);
            let found = decode(&named, pw, ph, MIN_SCORE).unwrap();
            for r in &p.faces {
                faces += 1;
                let Some(d) = found.iter().find(|d| iou(&d.rect, &r.rect) > 0.5) else {
                    missed += 1;
                    continue;
                };
                let px = d.rect.iter().zip(&r.rect).chain(d.landmarks.iter().flatten().zip(r.landmarks.iter().flatten())).map(|(x, y)| (x - y).abs()).fold(0f32, f32::max);
                worst_px = worst_px.max(px);
                assert!((d.score - r.score).abs() < 1e-3);
                let v = a.describe(&align(&img, &d.landmarks).unwrap()).unwrap();
                let mut rv = r.vector.clone();
                normalise(&mut rv);
                worst_cos = worst_cos.min(v.iter().zip(&rv).map(|(x, y)| x * y).sum());
            }
        }
        println!("{} pictures, {faces} faces, {missed} not found, worst box / landmark difference {worst_px:.3} px, worst vector cosine {worst_cos:.4}, {:.0} ms per picture", pictures.len(), started.elapsed().as_millis() as f32 / pictures.len().max(1) as f32);
        assert_eq!(missed, 0);
        assert!(worst_px < 0.5);
        assert!(worst_cos > 0.99, "the alignment differs from OpenCV's");
    }
}
