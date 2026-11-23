//! Where a point of the unedited, upright picture lands on the edited picture, and back: the
//! quarter turns, the crop, the straighten and the perspective steps, in the order they stand.
//! Used to draw face boxes over the edited picture and to turn a box drawn there into the
//! picture's own frame. Steps that only change colours are skipped; a border or text added by
//! the Effects tab changes the frame but is not followed (boxes are drawn on the picture itself).

use crate::imaging::EditOp;

/// The sampling factors of the perspective step (same constants as `imaging::perspective`).
const KEYSTONE: f32 = 0.4;

fn straighten_scale(deg: f32, w: f32, h: f32) -> (f32, f32, f32) {
    let a = deg.to_radians();
    let ratio = w.max(h) / w.min(h).max(1.0);
    (a.cos().abs() + a.sin().abs() * ratio, a.cos(), a.sin())
}

fn persp_scale(v: f32, hz: f32) -> f32 {
    1.0 / ((1.0 + KEYSTONE * v.abs()) * (1.0 + KEYSTONE * hz.abs()))
}

fn clean(v: f32) -> f32 {
    if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 }
}

/// One step forward: `p` is normalised to the picture of size `size` before the step; returns
/// the point normalised to the picture after it, and that picture's size.
fn forward(op: &EditOp, p: (f32, f32), size: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    let (x, y) = p;
    match op {
        EditOp::Rotate(n) => match n.rem_euclid(4) {
            1 => ((1.0 - y, x), (size.1, size.0)),
            2 => ((1.0 - x, 1.0 - y), size),
            3 => ((y, 1.0 - x), (size.1, size.0)),
            _ => (p, size),
        },
        EditOp::Crop(r) => {
            let (x0, x1) = (r[0].min(r[2]).clamp(0.0, 1.0), r[0].max(r[2]).clamp(0.0, 1.0));
            let (y0, y1) = (r[1].min(r[3]).clamp(0.0, 1.0), r[1].max(r[3]).clamp(0.0, 1.0));
            let (cw, ch) = ((x1 - x0).max(1e-4), (y1 - y0).max(1e-4));
            (((x - x0) / cw, (y - y0) / ch), (size.0 * cw, size.1 * ch))
        }
        EditOp::Straighten(deg) => {
            let (s, ca, sa) = straighten_scale(*deg, size.0, size.1);
            // the picture samples the source at c + R(d / s): the other way round, d = s * R^-1(u)
            let (u, v) = ((x - 0.5) * size.0, (y - 0.5) * size.1);
            let (dx, dy) = (u * ca - v * sa, u * sa + v * ca);
            (((dx * s) / size.0 + 0.5, (dy * s) / size.1 + 0.5), size)
        }
        EditOp::Perspective { vertical, horizontal } => {
            let (v, hz) = (clean(*vertical), clean(*horizontal));
            if v == 0.0 && hz == 0.0 {
                return (p, size);
            }
            let s = persp_scale(v, hz);
            // source offset (in half-sides) from the output offset: u = n * f(n) * s; solved by iteration
            let (ux, uy) = ((x - 0.5) * 2.0, (y - 0.5) * 2.0);
            let (mut nx, mut ny) = (ux, uy);
            for _ in 0..30 {
                let fx = 1.0 + KEYSTONE * v * ny;
                let fy = 1.0 - KEYSTONE * hz * nx;
                nx = ux / (fx * s);
                ny = uy / (fy * s);
            }
            ((nx / 2.0 + 0.5, ny / 2.0 + 0.5), size)
        }
        _ => (p, size),
    }
}

/// One step backwards: `p` normalised to the picture after the step, `size` the size of the
/// picture before it; returns the point normalised to the picture before.
fn backward(op: &EditOp, p: (f32, f32), size: (f32, f32)) -> (f32, f32) {
    let (x, y) = p;
    match op {
        EditOp::Rotate(n) => match n.rem_euclid(4) {
            1 => (y, 1.0 - x),
            2 => (1.0 - x, 1.0 - y),
            3 => (1.0 - y, x),
            _ => p,
        },
        EditOp::Crop(r) => {
            let (x0, x1) = (r[0].min(r[2]).clamp(0.0, 1.0), r[0].max(r[2]).clamp(0.0, 1.0));
            let (y0, y1) = (r[1].min(r[3]).clamp(0.0, 1.0), r[1].max(r[3]).clamp(0.0, 1.0));
            (x0 + x * (x1 - x0), y0 + y * (y1 - y0))
        }
        EditOp::Straighten(deg) => {
            let (s, ca, sa) = straighten_scale(*deg, size.0, size.1);
            let (dx, dy) = ((x - 0.5) * size.0 / s, (y - 0.5) * size.1 / s);
            ((dx * ca + dy * sa) / size.0 + 0.5, (-dx * sa + dy * ca) / size.1 + 0.5)
        }
        EditOp::Perspective { vertical, horizontal } => {
            let (v, hz) = (clean(*vertical), clean(*horizontal));
            let s = persp_scale(v, hz);
            let (nx, ny) = ((x - 0.5) * 2.0, (y - 0.5) * 2.0);
            let (fx, fy) = (1.0 + KEYSTONE * v * ny, 1.0 - KEYSTONE * hz * nx);
            ((nx * fx * s) / 2.0 + 0.5, (ny * fy * s) / 2.0 + 0.5)
        }
        _ => p,
    }
}

fn moves(op: &EditOp) -> bool {
    matches!(op, EditOp::Rotate(_) | EditOp::Crop(_) | EditOp::Straighten(_) | EditOp::Perspective { .. })
}

/// A point of the unedited upright picture (`size` in pixels, point as fractions) on the edited one.
pub fn to_edited(edits: &[EditOp], size: [f32; 2], p: (f32, f32)) -> (f32, f32) {
    let (mut p, mut sz) = (p, (size[0], size[1]));
    for op in edits.iter().filter(|o| moves(o)) {
        (p, sz) = forward(op, p, sz);
    }
    p
}

/// A point of the edited picture on the unedited one.
pub fn from_edited(edits: &[EditOp], size: [f32; 2], p: (f32, f32)) -> (f32, f32) {
    let ops: Vec<&EditOp> = edits.iter().filter(|o| moves(o)).collect();
    // the size before each step
    let mut sizes = vec![(size[0], size[1])];
    for op in &ops {
        let last = *sizes.last().unwrap_or(&(size[0], size[1]));
        sizes.push(forward(op, (0.5, 0.5), last).1);
    }
    let mut p = p;
    for (i, op) in ops.iter().enumerate().rev() {
        p = backward(op, p, sizes[i]);
    }
    p
}

/// A box (left, top, width, height as fractions of the unedited picture) on the edited picture:
/// the box around its four mapped corners. None when its centre is not on the edited picture
/// (cropped away).
pub fn rect_to_edited(edits: &[EditOp], size: [f32; 2], r: [f32; 4]) -> Option<[f32; 4]> {
    let corners = [(r[0], r[1]), (r[0] + r[2], r[1]), (r[0], r[1] + r[3]), (r[0] + r[2], r[1] + r[3])];
    let m: Vec<(f32, f32)> = corners.iter().map(|&c| to_edited(edits, size, c)).collect();
    let (x0, x1) = m.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = m.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    if !(0.0..=1.0).contains(&cx) || !(0.0..=1.0).contains(&cy) {
        return None;
    }
    Some([x0, y0, x1 - x0, y1 - y0])
}

/// A box drawn on the edited picture (left, top, width, height) in the unedited picture's frame.
pub fn rect_from_edited(edits: &[EditOp], size: [f32; 2], r: [f32; 4]) -> [f32; 4] {
    let corners = [(r[0], r[1]), (r[0] + r[2], r[1]), (r[0], r[1] + r[3]), (r[0] + r[2], r[1] + r[3])];
    let m: Vec<(f32, f32)> = corners.iter().map(|&c| from_edited(edits, size, c)).collect();
    let (x0, x1) = m.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = m.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let (x0, y0, x1, y1) = (x0.clamp(0.0, 1.0), y0.clamp(0.0, 1.0), x1.clamp(0.0, 1.0), y1.clamp(0.0, 1.0));
    [x0, y0, x1 - x0, y1 - y0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::apply_all;
    use image::{Rgba, RgbaImage};

    fn near(a: (f32, f32), b: (f32, f32), tol: f32) -> bool {
        (a.0 - b.0).abs() < tol && (a.1 - b.1).abs() < tol
    }

    /// A white picture with one black dot at the fractions `p`; where the dot is after the steps.
    fn dot_after(edits: &[EditOp], size: [u32; 2], p: (f32, f32)) -> (f32, f32) {
        let img = RgbaImage::from_fn(size[0], size[1], |x, y| {
            let (dx, dy) = (x as f32 + 0.5 - p.0 * size[0] as f32, y as f32 + 0.5 - p.1 * size[1] as f32);
            if dx * dx + dy * dy < 9.0 { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let out = apply_all(&img, edits);
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
        for (x, y, px) in out.enumerate_pixels() {
            let d = 255.0 - px[0] as f32;
            sx += (x as f32 + 0.5) * d;
            sy += (y as f32 + 0.5) * d;
            n += d;
        }
        (sx / n / out.width() as f32, sy / n / out.height() as f32)
    }

    #[test]
    fn the_mapping_follows_the_pixels() {
        let size = [400u32, 300];
        let fs = [400.0, 300.0];
        let p = (0.62, 0.35);
        let cases: Vec<Vec<EditOp>> = vec![
            vec![EditOp::Rotate(1)],
            vec![EditOp::Rotate(2)],
            vec![EditOp::Rotate(-1)],
            vec![EditOp::Crop([0.2, 0.1, 0.9, 0.8])],
            vec![EditOp::Straighten(4.0)],
            vec![EditOp::Straighten(-6.0), EditOp::Crop([0.1, 0.1, 0.95, 0.9])],
            vec![EditOp::Rotate(1), EditOp::Straighten(3.0), EditOp::Crop([0.0, 0.2, 0.8, 1.0])],
            vec![EditOp::Perspective { vertical: 0.7, horizontal: -0.3 }],
        ];
        for e in cases {
            let want = dot_after(&e, size, p);
            let got = to_edited(&e, fs, p);
            assert!(near(got, want, 0.012), "{e:?}: got {got:?} want {want:?}");
            let back = from_edited(&e, fs, got);
            assert!(near(back, p, 0.002), "{e:?}: back {back:?}");
        }
    }

    #[test]
    fn a_box_cropped_away_is_not_shown() {
        let e = [EditOp::Crop([0.5, 0.0, 1.0, 1.0])];
        assert_eq!(rect_to_edited(&e, [100.0, 100.0], [0.1, 0.1, 0.2, 0.2]), None);
        let r = rect_to_edited(&e, [100.0, 100.0], [0.6, 0.2, 0.2, 0.2]).unwrap();
        assert!((r[0] - 0.2).abs() < 1e-5 && (r[2] - 0.4).abs() < 1e-5);
        let back = rect_from_edited(&e, [100.0, 100.0], r);
        assert!((back[0] - 0.6).abs() < 1e-5 && (back[2] - 0.2).abs() < 1e-5);
    }
}
