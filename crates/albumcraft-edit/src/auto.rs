//! The picture analysis behind the one-click fixes. Each fix gives slider values (an `Adjust`,
//! or the straighten angle and the vertical perspective), never a step of its own, so what it
//! did shows on the Advanced sliders and can be changed there.
//!
//! - Tone (`exposure`, `contrast`, `lucky`): the settings are tried on a sample of the picture
//!   with the very code that applies them (`develop::Curve`), and searched until the result has
//!   the wanted brightness, black point and white point.
//! - Colour (`white_balance`): the pixels that are nearly gray are found and brought towards
//!   gray; the colourful ones (grass, sky, a red wall) do not count, as they do in a plain
//!   average, and a picture without gray parts is left alone.
//! - Geometry (`geometry`): straight line segments are found, the upright ones give the point
//!   they run towards (their vanishing point), which gives both the tilt and how much the
//!   picture leans back. Without upright lines, one long level line (a horizon) gives the tilt.

use crate::develop::{Adjust, Curve, luma, to_linear_lut, wb_for_logs};
use image::RgbaImage;

// ------------------------------------------------------------------ tone

/// An even sample of the picture's pixels (about 20,000), to try settings on.
struct Probe {
    px: Vec<[u8; 3]>,
}

impl Probe {
    fn of(img: &RgbaImage) -> Probe {
        let n = img.width() as usize * img.height() as usize;
        let mut step = (n / 20_000).max(1);
        // not a divisor of the width, which would sample a few columns only
        while step > 1 && (img.width() as usize).is_multiple_of(step) {
            step += 1;
        }
        Probe { px: img.pixels().step_by(step).map(|p| [p[0], p[1], p[2]]).collect() }
    }

    /// The brightness (0…1 as on screen) of the sample after `a`, darkest first.
    fn lumas(&self, a: &Adjust) -> Vec<f32> {
        let curve = Curve::new(&a.clamped());
        let mut ys: Vec<f32> = self.px.iter().map(|p| curve.pixel(*p).map(|v| v.clamp(0.0, 1.0))).map(|c| luma(c[0], c[1], c[2])).collect();
        ys.sort_by(f32::total_cmp);
        ys
    }
}

fn mean(ys: &[f32]) -> f32 {
    if ys.is_empty() { 0.0 } else { ys.iter().sum::<f32>() / ys.len() as f32 }
}

/// The value below which the share `q` of the (sorted) values lies.
fn pct(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f32 * q.clamp(0.0, 1.0)).round() as usize]
}

/// The setting between `lo` and `hi` at which `f`, which grows with the setting, reaches
/// `target` (the nearer end when it never does).
fn solve(lo: f32, hi: f32, target: f32, f: impl Fn(f32) -> f32) -> f32 {
    if f(lo) >= target {
        return lo;
    }
    if f(hi) <= target {
        return hi;
    }
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..14 {
        let mid = (lo + hi) / 2.0;
        if f(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn round1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/// The average brightness a well-exposed picture has, about.
const MIDDLE: f32 = 0.45;

fn exposure_on(probe: &Probe, a: Adjust) -> Adjust {
    let mut a = Adjust { exposure: 0.0, shadows: 0.0, highlights: 0.0, ..a };
    if probe.px.is_empty() {
        return a;
    }
    let now = mean(&probe.lumas(&a));
    if now < MIDDLE {
        // most of the way to the middle: a dark scene stays a little dark
        let want = now + 0.7 * (MIDDLE - now);
        if want - now < 0.015 {
            return a;
        }
        let with = |ev: f32| probe.lumas(&Adjust { exposure: ev, ..a });
        let ev = solve(0.0, 2.5, want, |ev| mean(&with(ev)));
        // not further than where the brightest parts reach white
        let cap = solve(0.0, 2.5, 0.985, |ev| pct(&with(ev), 0.995));
        a.exposure = round1(ev.min(cap));
        // bright parts in the way (a bright sky, lamps at night): the shadows are lifted instead
        if want - mean(&probe.lumas(&a)) > 0.03 {
            a.shadows = solve(0.0, 60.0, want, |s| mean(&probe.lumas(&Adjust { shadows: s, ..a }))).round();
        }
    } else {
        // darkened with care: snow, a white wall or a misty day are bright and right
        let want = now - 0.35 * (now - MIDDLE);
        if now - want < 0.03 {
            return a;
        }
        a.exposure = round1(solve(-1.0, 0.0, want, |ev| mean(&probe.lumas(&Adjust { exposure: ev, ..a }))));
    }
    a
}

fn contrast_on(probe: &Probe, a: Adjust) -> Adjust {
    let mut a = Adjust { blacks: 0.0, contrast: 0.0, ..a };
    if probe.px.is_empty() {
        return a;
    }
    // the white point: brightened until the brightest parts are nearly white (never darkened)
    if pct(&probe.lumas(&a), 0.995) < 0.94 {
        let from = a.exposure;
        a.exposure = round1(solve(from, from + 1.5, 0.96, |ev| pct(&probe.lumas(&Adjust { exposure: ev, ..a }), 0.995)));
    }
    // the black point: a washed-out black is deepened
    // (not all the way: mist and haze have no black)
    if pct(&probe.lumas(&a), 0.005) > 0.03 {
        a.blacks = solve(-50.0, 0.0, 0.02, |b| pct(&probe.lumas(&Adjust { blacks: b, ..a }), 0.005)).round();
    }
    // the middle: a flat picture gets some contrast
    let ys = probe.lumas(&a);
    let spread = pct(&ys, 0.9) - pct(&ys, 0.1);
    a.contrast = ((0.5 - spread) * 100.0).clamp(0.0, 25.0).round();
    a
}

/// Auto exposure: the exposure (and, when bright parts are in the way, the shadows) that brings
/// the picture's brightness towards the middle. `a` is the picture's adjustment so far; the
/// picture given is the one without it.
pub fn exposure(img: &RgbaImage, a: Adjust) -> Adjust {
    exposure_on(&Probe::of(img), a)
}

/// Auto contrast: the black point, the white point (with the exposure) and some contrast for a
/// flat picture.
pub fn contrast(img: &RgbaImage, a: Adjust) -> Adjust {
    contrast_on(&Probe::of(img), a)
}

/// "I'm feeling lucky": white balance, exposure, contrast and a touch of vibrance. The detail
/// sliders and the saturation of `a` stay.
pub fn lucky(img: &RgbaImage, a: Adjust) -> Adjust {
    let (temp, tint) = white_balance(img);
    let probe = Probe::of(img);
    let a = exposure_on(&probe, Adjust { temp, tint, ..a });
    let a = contrast_on(&probe, a);
    Adjust { contrast: a.contrast.max(10.0), vibrance: 20.0, ..a }
}

// ------------------------------------------------------------------ colour

/// Auto white balance (temperature, tint): the nearly gray parts of the picture become gray.
pub fn white_balance(img: &RgbaImage) -> (f32, f32) {
    let lin = to_linear_lut();
    let n = img.width() as usize * img.height() as usize;
    let step = (n / 60_000).max(1);
    // mid-tones that are not clipped, in linear light; and how far from gray each one is:
    // (ln r/g, ln b/g), which a colour cast moves by the same amount for every pixel
    let mut rgb: Vec<[f32; 3]> = vec![];
    for p in img.pixels().step_by(step) {
        let c = [lin[p[0] as usize], lin[p[1] as usize], lin[p[2] as usize]];
        let y = luma(c[0], c[1], c[2]);
        if (0.02..0.9).contains(&y) && p[0] < 250 && p[1] < 250 && p[2] < 250 && c.iter().all(|v| *v > 0.002) {
            rgb.push(c);
        }
    }
    if rgb.len() < 50 {
        return (0.0, 0.0);
    }
    let off: Vec<(f32, f32)> = rgb.iter().map(|c| ((c[0] / c[1]).ln(), (c[2] / c[1]).ln())).collect();
    // the cluster of nearly gray pixels: looked for near gray (the camera has balanced the
    // picture already, so a cast is small), then around where they are, each time more narrowly
    let mut at = (0f32, 0f32);
    let mut share = 0f32;
    for radius in [0.3f32, 0.22, 0.16, 0.14] {
        let (mut su, mut sv, mut k) = (0f64, 0f64, 0u32);
        for (u, v) in &off {
            if (u - at.0).powi(2) + (v - at.1).powi(2) < radius * radius {
                su += f64::from(*u);
                sv += f64::from(*v);
                k += 1;
            }
        }
        share = k as f32 / off.len() as f32;
        if k < 30 {
            break;
        }
        at = ((su / f64::from(k)) as f32, (sv / f64::from(k)) as f32);
    }
    // nothing gray to go by (a sunset, a field, coloured lights): the colours are left alone,
    // any guess would be as likely to spoil them
    if share < 0.05 {
        return (0.0, 0.0);
    }
    // part of the way only, and further the more of the picture is nearly gray: stone, mist
    // and shade are rarely quite gray, and warm light is often meant
    let (u, v, trust) = (at.0, at.1, 0.6 * (share / 0.25).min(1.0));
    let (temp, tint) = wb_for_logs(u, 0.0, v);
    let (temp, tint) = ((temp * trust).round(), (tint * trust).round());
    // a hair off gray is not a cast
    if temp.abs() < 3.0 && tint.abs() < 3.0 { (0.0, 0.0) } else { (temp, tint) }
}

// ------------------------------------------------------------------ geometry

/// A straight line segment found in a picture, in half-heights of the picture from its centre
/// (x to the right, y down).
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub a: (f32, f32),
    pub b: (f32, f32),
}

impl Segment {
    fn len(&self) -> f32 {
        (self.b.0 - self.a.0).hypot(self.b.1 - self.a.1)
    }

    fn mid(&self) -> (f32, f32) {
        ((self.a.0 + self.b.0) / 2.0, (self.a.1 + self.b.1) / 2.0)
    }

    /// Degrees off upright (+ = the lower end is to the right of the upper one).
    fn off_upright(&self) -> f32 {
        fold((self.b.0 - self.a.0).atan2(self.b.1 - self.a.1).to_degrees())
    }

    /// Degrees off level (+ = going down to the right).
    fn off_level(&self) -> f32 {
        fold((self.b.1 - self.a.1).atan2(self.b.0 - self.a.0).to_degrees())
    }

    /// The line through the segment as (a, b, c) with a·x + b·y + c = 0 and a² + b² = 1.
    fn line(&self) -> [f64; 3] {
        let (dx, dy) = (f64::from(self.b.0 - self.a.0), f64::from(self.b.1 - self.a.1));
        let n = dx.hypot(dy).max(1e-12);
        let (a, b) = (dy / n, -dx / n);
        [a, b, -(a * f64::from(self.a.0) + b * f64::from(self.a.1))]
    }
}

/// An angle of a line (which has no direction) brought into −90…90 degrees.
fn fold(deg: f32) -> f32 {
    (deg + 90.0).rem_euclid(180.0) - 90.0
}

/// The straight line segments of a picture, found on a copy at most 640 pixels long: pixels on
/// an edge are joined with their neighbours on an edge of the same direction, and each such
/// region that is long, thin and straight is a segment.
pub fn segments(img: &RgbaImage) -> Vec<Segment> {
    const LONG: u32 = 640;
    let small;
    let img = if img.width().max(img.height()) > LONG {
        let (w, h) = crate::imaging::fit(img.width(), img.height(), LONG);
        small = image::imageops::resize(img, w, h, image::imageops::FilterType::Triangle);
        &small
    } else {
        img
    };
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w < 16 || h < 16 {
        return vec![];
    }
    let gray: Vec<f32> = img.pixels().map(|p| luma(f32::from(p[0]), f32::from(p[1]), f32::from(p[2]))).collect();
    // slope of the brightness at each pixel: size and direction
    const EDGE: f32 = 5.0;
    let mut mag = vec![0f32; w * h];
    let mut dir = vec![0f32; w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let at = |dx: usize, dy: usize| gray[(y + dy - 1) * w + x + dx - 1];
            let gx = (at(2, 0) + 2.0 * at(2, 1) + at(2, 2) - at(0, 0) - 2.0 * at(0, 1) - at(0, 2)) / 8.0;
            let gy = (at(0, 2) + 2.0 * at(1, 2) + at(2, 2) - at(0, 0) - 2.0 * at(1, 0) - at(2, 0)) / 8.0;
            mag[y * w + x] = gx.hypot(gy);
            dir[y * w + x] = gy.atan2(gx);
        }
    }
    let mut order: Vec<u32> = (0..(w * h) as u32).filter(|&i| mag[i as usize] > EDGE).collect();
    order.sort_unstable_by(|a, b| mag[*b as usize].total_cmp(&mag[*a as usize]));
    let mut used = vec![false; w * h];
    let min_len = (w.max(h) as f32 * 0.04).max(12.0);
    let tolerance = 22.5f32.to_radians();
    let half = h as f32 / 2.0;
    let mut out = vec![];
    let mut region: Vec<u32> = vec![];
    for &seed in &order {
        if used[seed as usize] {
            continue;
        }
        // grow from the strongest edge pixels
        region.clear();
        region.push(seed);
        used[seed as usize] = true;
        let (mut sx, mut sy) = (dir[seed as usize].cos(), dir[seed as usize].sin());
        let mut towards = dir[seed as usize];
        let mut k = 0;
        while k < region.len() {
            let (x, y) = (region[k] as usize % w, region[k] as usize / w);
            k += 1;
            for (nx, ny) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)] {
                if nx == 0 || ny == 0 || nx >= w - 1 || ny >= h - 1 {
                    continue;
                }
                let i = ny * w + nx;
                if used[i] || mag[i] <= EDGE {
                    continue;
                }
                let d = (dir[i] - towards + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                if d.abs() < tolerance {
                    used[i] = true;
                    region.push(i as u32);
                    sx += dir[i].cos();
                    sy += dir[i].sin();
                    towards = sy.atan2(sx);
                }
            }
        }
        if (region.len() as f32) < min_len {
            continue;
        }
        // the line that fits the region best (weighted by the strength of the edge)
        let (mut mx, mut my, mut total) = (0f64, 0f64, 0f64);
        for &i in &region {
            let m = f64::from(mag[i as usize]);
            mx += m * (i as usize % w) as f64;
            my += m * (i as usize / w) as f64;
            total += m;
        }
        let (mx, my) = (mx / total, my / total);
        let (mut cxx, mut cyy, mut cxy) = (0f64, 0f64, 0f64);
        for &i in &region {
            let m = f64::from(mag[i as usize]);
            let (dx, dy) = ((i as usize % w) as f64 - mx, (i as usize / w) as f64 - my);
            cxx += m * dx * dx;
            cyy += m * dy * dy;
            cxy += m * dx * dy;
        }
        let along = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        let (ax, ay) = (along.cos(), along.sin());
        let (mut t0, mut t1, mut n0, mut n1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &i in &region {
            let (dx, dy) = ((i as usize % w) as f64 - mx, (i as usize / w) as f64 - my);
            let (t, n) = (dx * ax + dy * ay, -dx * ay + dy * ax);
            t0 = t0.min(t);
            t1 = t1.max(t);
            n0 = n0.min(n);
            n1 = n1.max(n);
        }
        let (len, width) = ((t1 - t0) as f32, (n1 - n0) as f32);
        // long, thin (a curve gives a wide region) and filled along its length
        if len < min_len || width > 4.0 + 0.04 * len || (region.len() as f32) < 0.7 * len {
            continue;
        }
        let point = |t: f64| (((mx + t * ax) as f32 + 0.5 - w as f32 / 2.0) / half, ((my + t * ay) as f32 + 0.5 - h as f32 / 2.0) / half);
        out.push(Segment { a: point(t0), b: point(t1) });
    }
    out
}

/// What levels a picture: the `Straighten` angle in degrees and the vertical `Perspective`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub angle: f32,
    pub vertical: f32,
}

/// The most the automatic fix turns a picture, in degrees: more is a picture taken that way.
const MAX_TILT: f32 = 15.0;

/// Auto straighten: the angle and vertical perspective that make the picture's upright lines
/// upright (or, without any, its long level lines level). `None` when there are no clear lines.
/// The picture given is the one before its straighten and perspective steps.
pub fn geometry(img: &RgbaImage) -> Option<Geometry> {
    let segs = segments(img);
    let (w, h) = (img.width().max(1) as f32, img.height().max(1) as f32);
    let found = upright_lines(&segs, w / h).or_else(|| level_lines(&segs, w / h).map(|angle| Geometry { angle, vertical: 0.0 }))?;
    Some(Geometry { angle: round1(found.angle), vertical: (found.vertical * 100.0).round() / 100.0 })
}

/// How strongly the perspective sliders bend the picture (`imaging::perspective`).
const KEYSTONE: f32 = 0.4;

/// From the upright lines: the point they all run towards gives the tilt (the direction of that
/// point from the centre) and the vertical perspective (how near it is).
fn upright_lines(segs: &[Segment], aspect: f32) -> Option<Geometry> {
    let mut up: Vec<&Segment> = segs.iter().filter(|s| s.off_upright().abs() <= 25.0).collect();
    let total: f32 = up.iter().map(|s| s.len()).sum();
    // at least four, and together as long as the picture is high
    if up.len() < 4 || total < 2.0 {
        return None;
    }
    up.sort_by(|a, b| b.len().total_cmp(&a.len()));
    let lines: Vec<[f64; 3]> = up.iter().map(|s| s.line()).collect();
    // how much line length agrees with a point (x, y, w): the segments that point at it
    let agree = |p: &[f64; 3]| -> (f32, Vec<usize>) {
        let sin_tol = 1.5f64.to_radians().sin();
        let mut ids = vec![];
        let mut sum = 0f32;
        for (i, s) in up.iter().enumerate() {
            let m = s.mid();
            let (dx, dy) = (p[0] - p[2] * f64::from(m.0), p[1] - p[2] * f64::from(m.1));
            let n = dx.hypot(dy);
            if n < 1e-9 {
                continue;
            }
            let (sx, sy) = (f64::from(s.b.0 - s.a.0), f64::from(s.b.1 - s.a.1));
            let sn = sx.hypot(sy).max(1e-12);
            if ((dx * sy - dy * sx) / (n * sn)).abs() < sin_tol {
                ids.push(i);
                sum += s.len();
            }
        }
        (sum, ids)
    };
    // the point two of the longer segments meet at, for every pair: the one most others agree with
    let top = lines.len().min(40);
    let mut best: Option<(f32, [f64; 3])> = None;
    for i in 0..top {
        for j in i + 1..top {
            let (a, b) = (lines[i], lines[j]);
            let p = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            if n < 1e-12 {
                continue;
            }
            let p = p.map(|v| v / n);
            let (sum, _) = agree(&p);
            if best.is_none_or(|(s, _)| sum > s) {
                best = Some((sum, p));
            }
        }
    }
    let (_, mut p) = best?;
    // the point that fits all agreeing segments best, twice
    for _ in 0..2 {
        let (_, ids) = agree(&p);
        if ids.len() < 3 {
            break;
        }
        let mut m = [[0f64; 3]; 3];
        for &i in &ids {
            let (l, wt) = (lines[i], f64::from(up[i].len()));
            for r in 0..3 {
                for c in 0..3 {
                    m[r][c] += wt * l[r] * l[c];
                }
            }
        }
        p = smallest_eigenvector(m);
    }
    let (sum, ids) = agree(&p);
    // most of the upright lines have to agree, or there is no common direction (trees, people)
    if ids.len() < 4 || sum < 1.5 || sum < 0.5 * total {
        return None;
    }
    // where the point is, from the centre: its direction is "up" (or "down"), its distance the
    // perspective
    let far = p[2].abs() < 1e-9;
    let (px, py) = if far { (p[0], p[1]) } else { (p[0] / p[2], p[1] / p[2]) };
    let dist = if far { f64::INFINITY } else { px.hypot(py) };
    let above = py < 0.0;
    let (ux, uy) = if above { (-px, -py) } else { (px, py) };
    let angle = (ux.atan2(uy).to_degrees()) as f32;
    if angle.abs() > MAX_TILT {
        return None;
    }
    // nearer than this, the lines are not uprights leaning back but a road, rails or a corridor
    if dist < 1.5 {
        return None;
    }
    // the straighten step zooms in, which moves the point away by as much
    let a = angle.to_radians().abs();
    let ratio = aspect.max(1.0 / aspect);
    let d = dist as f32 * (a.cos() + a.sin() * ratio);
    // the perspective step makes upright the lines that meet 1 / (K·v·(1 + K·|v|)) half-heights
    // from the centre
    let v = if d.is_finite() { (-1.0 + (1.0 + 4.0 / d).sqrt()) / (2.0 * KEYSTONE) } else { 0.0 };
    let v = if v < 0.03 { 0.0 } else { v.min(1.0) };
    Some(Geometry { angle: if angle.abs() < 0.15 { 0.0 } else { angle }, vertical: if above { v } else { -v } })
}

/// From the level lines: the tilt of the one long line across the picture (a horizon, a shore).
/// Lines that are only parallel do not count: fences, roofs and benches seen at an angle are
/// level in the world and tilted in the picture.
fn level_lines(segs: &[Segment], aspect: f32) -> Option<f32> {
    let level: Vec<&Segment> = segs.iter().filter(|s| s.off_level().abs() <= MAX_TILT).collect();
    // for each segment, the pieces of line that continue it: same tilt, on the same line
    let mut best: Option<(f32, f32)> = None;
    for s in &level {
        let (tilt, line) = (s.off_level(), s.line());
        let (mut sum, mut tilts) = (0f32, 0f32);
        for o in &level {
            let m = o.mid();
            let away = (line[0] * f64::from(m.0) + line[1] * f64::from(m.1) + line[2]).abs();
            if (o.off_level() - tilt).abs() <= 1.0 && away < 0.03 {
                sum += o.len();
                tilts += o.off_level() * o.len();
            }
        }
        if best.is_none_or(|(b, _)| sum > b) {
            best = Some((sum, tilts / sum.max(1e-6)));
        }
    }
    let (sum, tilt) = best?;
    // together at least half the picture's width (which is 2 × aspect half-heights), and more
    // of it the larger the tilt: a real horizon is rarely far off level, a hillside, a roof seen
    // from the side or a shore running away often is
    // (and past 5 degrees it is not taken for a horizon at all)
    if tilt.abs() > 5.0 || sum < 2.0 * aspect * (0.5 + tilt.abs() / 10.0).min(0.9) {
        return None;
    }
    Some(if tilt.abs() < 0.15 { 0.0 } else { -tilt })
}

/// The eigenvector of the smallest eigenvalue of a symmetric 3 × 3 matrix (Jacobi rotations).
fn smallest_eigenvector(mut m: [[f64; 3]; 3]) -> [f64; 3] {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..30 {
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            if m[p][q].abs() < 1e-18 {
                continue;
            }
            let theta = 0.5 * (2.0 * m[p][q]).atan2(m[q][q] - m[p][p]);
            let (c, s) = (theta.cos(), theta.sin());
            // m = Jᵀ m J, v = v J
            for k in 0..3 {
                let (a, b) = (m[k][p], m[k][q]);
                m[k][p] = c * a - s * b;
                m[k][q] = s * a + c * b;
            }
            for k in 0..3 {
                let (a, b) = (m[p][k], m[q][k]);
                m[p][k] = c * a - s * b;
                m[q][k] = s * a + c * b;
            }
            for row in &mut v {
                let (a, b) = (row[p], row[q]);
                row[p] = c * a - s * b;
                row[q] = s * a + c * b;
            }
        }
    }
    let k = (0..3).min_by(|a, b| m[*a][*a].total_cmp(&m[*b][*b])).unwrap_or(0);
    [v[0][k], v[1][k], v[2][k]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::{EditOp, apply, apply_all};
    use image::Rgba;

    fn flat(rgb: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(64, 48, Rgba([rgb[0], rgb[1], rgb[2], 255]))
    }

    /// The picture with every value brought from 0…255 into `lo`…`hi`.
    fn squeezed(img: &RgbaImage, lo: f32, hi: f32) -> RgbaImage {
        let mut out = img.clone();
        for p in out.pixels_mut() {
            for c in 0..3 {
                p[c] = (lo + f32::from(p[c]) / 255.0 * (hi - lo)) as u8;
            }
        }
        out
    }

    fn adjusted(img: &RgbaImage, a: &Adjust) -> RgbaImage {
        let mut out = img.clone();
        crate::develop::apply(&mut out, a);
        out
    }

    #[test]
    fn white_balance_goes_by_the_gray_parts_not_the_grass() {
        // a third gray tones, the rest strong green; all with a yellow cast
        let cast = |p: [u8; 3]| Rgba([p[0], p[1], (f32::from(p[2]) * 0.9) as u8, 255]);
        let img = RgbaImage::from_fn(150, 100, |x, y| if x < 50 { cast([60 + (y as u8), 60 + (y as u8), 60 + (y as u8)]) } else { cast([40, 150, 30]) });
        let (temp, tint) = white_balance(&img);
        assert!(temp < -5.0, "temp {temp}");
        // the gray is nearer gray (part of the way), and the green did not pull it towards magenta
        let spread = |i: &RgbaImage| {
            let p = i.get_pixel(10, 50);
            p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2])
        };
        let fixed = adjusted(&img, &Adjust { temp, tint, ..Default::default() });
        assert!(spread(&fixed) * 2 <= spread(&img), "{} -> {} with temp {temp}, tint {tint}", spread(&img), spread(&fixed));
        // a picture of grass only is left alone
        assert_eq!(white_balance(&RgbaImage::from_fn(60, 40, |x, _| cast([40 + (x as u8 % 8), 150, 30]))), (0.0, 0.0));
        // a picture without a cast is left alone
        let clean = RgbaImage::from_fn(150, 100, |x, y| if x < 50 { Rgba([60 + (y as u8), 60 + (y as u8), 60 + (y as u8), 255]) } else { Rgba([40, 150, 30, 255]) });
        assert_eq!(white_balance(&clean), (0.0, 0.0));
        assert_eq!(white_balance(&flat([0, 0, 0])), (0.0, 0.0));
    }

    #[test]
    fn exposure_brightens_a_dark_picture_and_leaves_a_good_one() {
        let good = crate::imaging::sample_image(3, 160, 120);
        let mut dark = good.clone();
        for p in dark.pixels_mut() {
            for c in 0..3 {
                p[c] /= 4;
            }
        }
        let a = exposure(&dark, Adjust::default());
        assert!(a.exposure > 0.5, "{a:?}");
        let before = mean(&Probe::of(&dark).lumas(&Adjust::default()));
        let after = mean(&Probe::of(&dark).lumas(&a));
        assert!(after > before + 0.1 && after < 0.6, "{before} -> {after}");
        // asked again on the result, less happens each time: the fix settles
        let again = exposure(&adjusted(&dark, &a), Adjust::default());
        assert!(again.exposure >= 0.0 && again.exposure <= 0.6 && again.exposure < a.exposure, "{a:?} then {again:?}");
        // the other sliders stay
        let kept = exposure(&dark, Adjust { vibrance: 30.0, temp: 12.0, ..Default::default() });
        assert_eq!((kept.vibrance, kept.temp), (30.0, 12.0));
    }

    #[test]
    fn contrast_stretches_a_washed_out_picture() {
        let full = RgbaImage::from_fn(256, 64, |x, _| Rgba([x as u8, x as u8, x as u8, 255]));
        let img = squeezed(&full, 28.0, 190.0);
        let a = contrast(&img, Adjust::default());
        assert!(a.blacks < -20.0 && a.exposure > 0.2, "{a:?}");
        let ys = Probe::of(&img).lumas(&a);
        assert!(pct(&ys, 0.005) < 0.12 && pct(&ys, 0.995) > 0.9, "{} … {}", pct(&ys, 0.005), pct(&ys, 0.995));
        // a picture from black to white keeps its black and white points
        let a = contrast(&full, Adjust::default());
        assert_eq!((a.blacks, a.exposure), (0.0, 0.0), "{a:?}");
    }

    #[test]
    fn lucky_gives_slider_values() {
        let img = squeezed(&crate::imaging::sample_image(7, 160, 120), 20.0, 150.0);
        let a = lucky(&img, Adjust { sharpen: 40.0, ..Default::default() });
        assert!(a.exposure > 0.0 && a.vibrance == 20.0 && a.contrast >= 10.0 && a.sharpen == 40.0, "{a:?}");
        assert_eq!(a, a.clamped());
    }

    /// A picture with strong horizontal stripes, turned by `deg`.
    fn stripes(deg: f32) -> RgbaImage {
        let img = RgbaImage::from_fn(400, 300, |_, y| if (y / 15) % 2 == 0 { Rgba([230, 230, 230, 255]) } else { Rgba([30, 30, 30, 255]) });
        apply(img, &EditOp::Straighten(deg))
    }

    #[test]
    fn level_lines_level_a_tilted_picture() {
        for tilt in [4.0, -3.0] {
            let tilted = stripes(tilt);
            let fix = geometry(&tilted).expect("lines found");
            assert!((fix.angle + tilt).abs() < 0.6 && fix.vertical == 0.0, "tilt {tilt}: {fix:?}");
            let fixed = apply(tilted, &EditOp::Straighten(fix.angle));
            let left = geometry(&fixed).expect("lines found");
            assert!(left.angle.abs() < 0.5, "tilt {tilt}: left {left:?}");
        }
        assert_eq!(geometry(&stripes(0.0)).map(|g| g.angle), Some(0.0));
    }

    /// The front of a building: rows of dark windows on a light wall.
    fn front() -> RgbaImage {
        RgbaImage::from_fn(480, 360, |x, y| if (x % 60) > 22 && (y % 60) > 22 { Rgba([40, 50, 70, 255]) } else { Rgba([215, 205, 190, 255]) })
    }

    #[test]
    fn upright_lines_stand_a_leaning_building_up() {
        // seen from below (narrower at the top) and tilted
        for (lean, tilt) in [(-0.5, 0.0), (-0.4, -3.0), (0.3, 2.0), (0.0, 5.0)] {
            let seen = apply_all(&front(), &[EditOp::Perspective { vertical: lean, horizontal: 0.0 }, EditOp::Straighten(tilt)]);
            let fix = geometry(&seen).expect("lines found");
            assert!((fix.angle + tilt).abs() < 0.7, "lean {lean}, tilt {tilt}: {fix:?}");
            assert!(fix.vertical * lean <= 0.0 && (fix.vertical.abs() - lean.abs()).abs() < 0.2, "lean {lean}, tilt {tilt}: {fix:?}");
            let fixed = apply_all(&seen, &[EditOp::Straighten(fix.angle), EditOp::Perspective { vertical: fix.vertical, horizontal: 0.0 }]);
            let left = geometry(&fixed).expect("lines found");
            assert!(left.angle.abs() < 0.5 && left.vertical.abs() < 0.1, "lean {lean}, tilt {tilt}: fix {fix:?}, left {left:?}");
        }
    }

    #[test]
    fn nothing_found_without_lines() {
        assert_eq!(geometry(&flat([90, 120, 150])), None);
        // a road running to the middle of the picture is not a leaning building
        let road = RgbaImage::from_fn(400, 300, |x, y| {
            let half = (y as f32 - 150.0).max(0.0) * 0.9;
            if y > 150 && (x as f32 - 200.0).abs() < half { Rgba([60, 60, 60, 255]) } else { Rgba([170, 200, 170, 255]) }
        });
        assert!(geometry(&road).is_none_or(|g| g.vertical == 0.0), "{:?}", geometry(&road));
    }

    #[test]
    fn eigenvector_of_the_smallest_eigenvalue() {
        // eigenvalues 1, 2 and 4 with known directions
        let m = [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 4.0]];
        let v = smallest_eigenvector(m);
        let s = std::f64::consts::FRAC_1_SQRT_2;
        assert!((v[0].abs() - s).abs() < 1e-9 && (v[1].abs() - s).abs() < 1e-9 && v[2].abs() < 1e-9 && v[0] * v[1] < 0.0, "{v:?}");
    }
}
