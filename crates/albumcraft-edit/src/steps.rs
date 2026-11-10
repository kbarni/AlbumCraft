//! Editing a picture's list of edit steps the way the editor's panels do: one adjustment, one
//! look, one straighten, one perspective and one crop per picture, each changed where it stands
//! in the list instead of stacking a new step every time.
//!
//! The list itself (`Vec<EditOp>`, applied in order by `imaging::apply_all`) stays what is
//! saved, so lists made before (or by other means: pasted edits, imported notes, commands) keep
//! working: when a list holds several steps of a kind, the last one is "the" one edited here,
//! the earlier ones stay as they are.

use crate::develop::Adjust;
use crate::imaging::EditOp;
use crate::looks::Look;

fn last(edits: &[EditOp], is: impl Fn(&EditOp) -> bool) -> Option<usize> {
    edits.iter().rposition(is)
}

/// Steps that move or cut the picture (the others only change colours or draw on it).
fn is_geometry(op: &EditOp) -> bool {
    matches!(op, EditOp::Rotate(_) | EditOp::Crop(_) | EditOp::Straighten(_) | EditOp::Perspective { .. } | EditOp::Redeye(_))
}

/// The picture's adjustment (all neutral when it has none).
pub fn adjust_of(edits: &[EditOp]) -> Adjust {
    match last(edits, |e| matches!(e, EditOp::Adjust(_))).and_then(|i| edits.get(i)) {
        Some(EditOp::Adjust(a)) => *a,
        _ => Adjust::default(),
    }
}

/// Set the picture's adjustment: changed where it stands, added at the end when there is none,
/// removed when it is neutral.
pub fn set_adjust(edits: &mut Vec<EditOp>, a: Adjust) {
    let a = a.clamped();
    match last(edits, |e| matches!(e, EditOp::Adjust(_))) {
        Some(i) if a.is_neutral() => {
            edits.remove(i);
        }
        Some(i) => edits[i] = EditOp::Adjust(a),
        None if a.is_neutral() => {}
        None => edits.push(EditOp::Adjust(a)),
    }
}

/// The picture's look and its intensity.
pub fn look_of(edits: &[EditOp]) -> Option<(Look, f32)> {
    match last(edits, |e| matches!(e, EditOp::Look { .. })).and_then(|i| edits.get(i)) {
        Some(EditOp::Look { look, amount }) => Some((*look, *amount)),
        _ => None,
    }
}

/// Choose a look (it takes the place of the picture's look) or, with None, remove it.
pub fn set_look(edits: &mut Vec<EditOp>, look: Option<(Look, f32)>) {
    let at = last(edits, |e| matches!(e, EditOp::Look { .. }));
    match (at, look) {
        (Some(i), Some((look, amount))) => edits[i] = EditOp::Look { look, amount: amount.clamp(0.0, 1.0) },
        (Some(i), None) => {
            edits.remove(i);
        }
        (None, Some((look, amount))) => {
            // under the effects drawn on the picture (borders, text), which stay on top
            let at = edits.iter().position(|e| matches!(e, EditOp::Decor(_))).unwrap_or(edits.len());
            edits.insert(at, EditOp::Look { look, amount: amount.clamp(0.0, 1.0) });
        }
        (None, None) => {}
    }
}

/// The steps without the picture's look (what the look previews start from).
pub fn without_look(edits: &[EditOp]) -> Vec<EditOp> {
    let mut v = edits.to_vec();
    set_look(&mut v, None);
    v
}

/// The straighten angle and the perspective (vertical, horizontal); zero when not set.
pub fn geometry_of(edits: &[EditOp]) -> (f32, (f32, f32)) {
    let angle = edits.iter().rev().find_map(|e| if let EditOp::Straighten(a) = e { Some(*a) } else { None }).unwrap_or(0.0);
    let persp = edits
        .iter()
        .rev()
        .find_map(|e| if let EditOp::Perspective { vertical, horizontal } = e { Some((*vertical, *horizontal)) } else { None })
        .unwrap_or((0.0, 0.0));
    (angle, persp)
}

/// Set the straighten or the perspective step (`op` is one of those): changed where it stands;
/// a new one goes before the crop, so that the crop keeps cutting the straightened picture;
/// back to zero removes it.
pub fn set_geometry(edits: &mut Vec<EditOp>, op: EditOp) {
    let neutral = match op {
        EditOp::Straighten(a) => a.abs() < 0.05,
        EditOp::Perspective { vertical, horizontal } => vertical.abs() < 0.005 && horizontal.abs() < 0.005,
        _ => return,
    };
    let kind = std::mem::discriminant(&op);
    match last(edits, |e| std::mem::discriminant(e) == kind) {
        Some(i) if neutral => {
            edits.remove(i);
        }
        Some(i) => edits[i] = op,
        None if neutral => {}
        None => {
            let at = last(edits, |e| matches!(e, EditOp::Crop(_))).unwrap_or_else(|| crop_place(edits));
            edits.insert(at, op);
        }
    }
}

/// Where a new crop goes: after the steps that move or cut the picture, before the others.
fn crop_place(edits: &[EditOp]) -> usize {
    last(edits, is_geometry).map_or(0, |i| i + 1)
}

/// The picture's crop box (x0, y0, x1, y1 as fractions of the picture at that step).
pub fn crop_of(edits: &[EditOp]) -> Option<[f32; 4]> {
    match last(edits, |e| matches!(e, EditOp::Crop(_))).and_then(|i| edits.get(i)) {
        Some(EditOp::Crop(r)) => Some(*r),
        _ => None,
    }
}

/// Set the picture's crop (changed where it stands, or added at `crop_place`), or remove it.
pub fn set_crop(edits: &mut Vec<EditOp>, rect: Option<[f32; 4]>) {
    match (last(edits, |e| matches!(e, EditOp::Crop(_))), rect) {
        (Some(i), Some(r)) => edits[i] = EditOp::Crop(r),
        (Some(i), None) => {
            edits.remove(i);
        }
        (None, Some(r)) => edits.insert(crop_place(edits), EditOp::Crop(r)),
        (None, None) => {}
    }
}

/// The crop box `r` as it is after `turns` quarter turns clockwise of the picture.
pub fn turn_rect(r: [f32; 4], turns: i32) -> [f32; 4] {
    let mut r = [r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])];
    for _ in 0..turns.rem_euclid(4) {
        // a point (x, y) goes to (1 - y, x)
        r = [1.0 - r[3], r[0], 1.0 - r[1], r[2]];
    }
    r
}

/// Quarter turns made after the crop are moved before it (the crop box turned with them): the
/// picture comes out the same, and the crop tool can then show the picture the way up it is.
pub fn turns_before_crop(edits: &mut Vec<EditOp>) {
    let Some(ci) = last(edits, |e| matches!(e, EditOp::Crop(_))) else { return };
    // only when nothing else that moves the picture comes after the crop
    if edits[ci + 1..].iter().any(|e| is_geometry(e) && !matches!(e, EditOp::Rotate(_))) {
        return;
    }
    let turns: i32 = edits[ci + 1..].iter().map(|e| if let EditOp::Rotate(n) = e { *n } else { 0 }).sum();
    if !edits[ci + 1..].iter().any(|e| matches!(e, EditOp::Rotate(_))) {
        return;
    }
    let mut i = ci + 1;
    while i < edits.len() {
        if matches!(edits[i], EditOp::Rotate(_)) {
            edits.remove(i);
        } else {
            i += 1;
        }
    }
    if let EditOp::Crop(r) = &mut edits[ci] {
        *r = turn_rect(*r, turns);
    }
    if turns.rem_euclid(4) != 0 {
        edits.insert(ci, EditOp::Rotate(turns.rem_euclid(4)));
    }
}

/// What the crop tool shows: the picture as it is where the crop cuts it (everything before the
/// crop, or before where a new crop would go), with the later colour changes, but without the
/// crop itself and without what is drawn on or measured against the cropped picture after it
/// (borders, text, later turns).
pub fn crop_view(edits: &[EditOp]) -> Vec<EditOp> {
    let (at, skip) = match last(edits, |e| matches!(e, EditOp::Crop(_))) {
        Some(i) => (i, 1),
        None => (crop_place(edits), 0),
    };
    let mut v: Vec<EditOp> = edits[..at].to_vec();
    v.extend(edits[at + skip..].iter().filter(|e| !is_geometry(e) && !matches!(e, EditOp::Decor(_))).cloned());
    v
}

/// The part of a crop box the pointer took hold of.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Grip {
    /// The whole box, to move it.
    Inside,
    /// An edge or a corner: which sides follow the pointer (left, top, right, bottom).
    Sides { left: bool, top: bool, right: bool, bottom: bool },
}

/// What a pointer at `p` takes hold of on the box `b` (both as fractions of the picture);
/// `reach` is how near an edge counts, as fractions of the picture's (width, height).
pub fn grip_at(b: [f32; 4], p: (f32, f32), reach: (f32, f32)) -> Option<Grip> {
    let near = |v: f32, edge: f32, r: f32| (v - edge).abs() <= r;
    let in_x = p.0 >= b[0] - reach.0 && p.0 <= b[2] + reach.0;
    let in_y = p.1 >= b[1] - reach.1 && p.1 <= b[3] + reach.1;
    if !in_x || !in_y {
        return None;
    }
    let (mut left, mut right) = (near(p.0, b[0], reach.0), near(p.0, b[2], reach.0));
    let (mut top, mut bottom) = (near(p.1, b[1], reach.1), near(p.1, b[3], reach.1));
    // a small box: the nearer of two opposite edges
    if left && right {
        left = (p.0 - b[0]).abs() <= (p.0 - b[2]).abs();
        right = !left;
    }
    if top && bottom {
        top = (p.1 - b[1]).abs() <= (p.1 - b[3]).abs();
        bottom = !top;
    }
    if left || right || top || bottom { Some(Grip::Sides { left, top, right, bottom }) } else { Some(Grip::Inside) }
}

/// Smallest crop box, as a fraction of the picture's side.
const MIN_BOX: f32 = 0.02;

/// The box `start` after the pointer, holding it by `grip`, moved by `(dx, dy)` (fractions of
/// the picture). The box stays inside the picture. With `shape` (the box's width / height in
/// the same fractions, i.e. the wanted shape divided by the picture's own) the box keeps that
/// shape, also against the picture's edges.
pub fn drag_box(start: [f32; 4], grip: Grip, (dx, dy): (f32, f32), shape: Option<f32>) -> [f32; 4] {
    let [x0, y0, x1, y1] = start;
    let (w, h) = (x1 - x0, y1 - y0);
    match grip {
        Grip::Inside => {
            let x = (x0 + dx).clamp(0.0, (1.0 - w).max(0.0));
            let y = (y0 + dy).clamp(0.0, (1.0 - h).max(0.0));
            [x, y, x + w, y + h]
        }
        Grip::Sides { left, top, right, bottom } => {
            let (mut nx0, mut ny0, mut nx1, mut ny1) = (x0, y0, x1, y1);
            if left {
                nx0 = (x0 + dx).clamp(0.0, x1 - MIN_BOX);
            }
            if right {
                nx1 = (x1 + dx).clamp(x0 + MIN_BOX, 1.0);
            }
            if top {
                ny0 = (y0 + dy).clamp(0.0, y1 - MIN_BOX);
            }
            if bottom {
                ny1 = (y1 + dy).clamp(y0 + MIN_BOX, 1.0);
            }
            let Some(shape) = shape.filter(|s| s.is_finite() && *s > 0.0) else { return [nx0, ny0, nx1, ny1] };
            // the fixed point: the opposite corner, or the middle of the opposite edge
            let ax = if left { x1 } else if right { x0 } else { (x0 + x1) / 2.0 };
            let ay = if top { y1 } else if bottom { y0 } else { (y0 + y1) / 2.0 };
            // the size asked for: the dragged dimension leads (the larger one at a corner)
            let (mut bw, mut bh) = (nx1 - nx0, ny1 - ny0);
            let by_width = if (left || right) && (top || bottom) { bw / shape >= bh } else { left || right };
            if by_width {
                bh = bw / shape;
            } else {
                bw = bh * shape;
            }
            // room from the fixed point to the picture's edges, in the directions the box grows
            let room_x = if left { ax } else if right { 1.0 - ax } else { 2.0 * ax.min(1.0 - ax) };
            let room_y = if top { ay } else if bottom { 1.0 - ay } else { 2.0 * ay.min(1.0 - ay) };
            let k = (room_x / bw).min(room_y / bh).min(1.0);
            let (bw, bh) = (bw * k, bh * k);
            let (bx0, bx1) = if left { (ax - bw, ax) } else if right { (ax, ax + bw) } else { (ax - bw / 2.0, ax + bw / 2.0) };
            let (by0, by1) = if top { (ay - bh, ay) } else if bottom { (ay, ay + bh) } else { (ay - bh / 2.0, ay + bh / 2.0) };
            [bx0, by0, bx1, by1]
        }
    }
}

/// A new box drawn from `a` to the pointer at `b` (fractions of the picture), inside the
/// picture, keeping `shape` (see `drag_box`) when given. None while it is too small.
pub fn drawn_box(a: (f32, f32), b: (f32, f32), shape: Option<f32>) -> Option<[f32; 4]> {
    let (bx, by) = (b.0.clamp(0.0, 1.0), b.1.clamp(0.0, 1.0));
    let (mut w, mut h) = ((bx - a.0).abs(), (by - a.1).abs());
    if let Some(shape) = shape.filter(|s| s.is_finite() && *s > 0.0) {
        // the width leads; both shrink together when the height would leave the picture
        h = w / shape;
        let room = if by >= a.1 { 1.0 - a.1 } else { a.1 };
        if h > room {
            h = room;
            w = h * shape;
        }
    }
    if w < 0.01 || h < 0.01 {
        return None;
    }
    let x0 = if bx >= a.0 { a.0 } else { a.0 - w };
    let y0 = if by >= a.1 { a.1 } else { a.1 - h };
    Some([x0, y0, x0 + w, y0 + h])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::{apply_all, sample_image};

    fn adj(exposure: f32) -> Adjust {
        Adjust { exposure, ..Default::default() }
    }

    #[test]
    fn the_adjustment_is_changed_where_it_stands() {
        let mut e = vec![EditOp::Adjust(adj(0.5)), EditOp::Crop([0.1, 0.1, 0.9, 0.9])];
        // sliders show the picture's values, also after a crop was added
        assert_eq!(adjust_of(&e).exposure, 0.5);
        set_adjust(&mut e, adj(1.0));
        assert_eq!(e, vec![EditOp::Adjust(adj(1.0)), EditOp::Crop([0.1, 0.1, 0.9, 0.9])]);
        set_adjust(&mut e, Adjust::default());
        assert_eq!(e, vec![EditOp::Crop([0.1, 0.1, 0.9, 0.9])]);
        set_adjust(&mut e, adj(0.2));
        assert_eq!(e.len(), 2);
        assert_eq!(adjust_of(&[]), Adjust::default());
    }

    #[test]
    fn one_look_under_the_effects() {
        let border = EditOp::Decor(crate::decor::Decor::Border { width: 0.05, bottom: 0.0, color: [255, 255, 255] });
        let mut e = vec![EditOp::Adjust(adj(0.5)), border.clone()];
        set_look(&mut e, Some((Look::Vivid, 2.0)));
        assert_eq!(e[1], EditOp::Look { look: Look::Vivid, amount: 1.0 });
        assert_eq!(e[2], border);
        set_look(&mut e, Some((Look::Sepia, 0.4)));
        assert_eq!(look_of(&e), Some((Look::Sepia, 0.4)));
        assert_eq!(e.len(), 3);
        assert_eq!(without_look(&e).len(), 2);
        set_look(&mut e, None);
        assert_eq!(look_of(&e), None);
    }

    #[test]
    fn straighten_goes_before_the_crop() {
        let mut e = vec![EditOp::Rotate(1), EditOp::Crop([0.0, 0.0, 0.5, 0.5]), EditOp::Adjust(adj(0.3))];
        set_geometry(&mut e, EditOp::Straighten(2.0));
        assert_eq!(e[1], EditOp::Straighten(2.0));
        assert!(matches!(e[2], EditOp::Crop(_)));
        set_geometry(&mut e, EditOp::Straighten(3.0));
        assert_eq!(geometry_of(&e).0, 3.0);
        assert_eq!(e.len(), 4);
        set_geometry(&mut e, EditOp::Straighten(0.0));
        assert_eq!(e.len(), 3);
        // without a crop: after the turns, before the colours
        let mut e = vec![EditOp::Rotate(1), EditOp::Adjust(adj(0.3))];
        set_geometry(&mut e, EditOp::Perspective { vertical: 0.5, horizontal: 0.0 });
        assert!(matches!(e[1], EditOp::Perspective { .. }));
    }

    #[test]
    fn the_crop_is_one_step_that_can_be_changed() {
        let mut e = vec![EditOp::Adjust(adj(0.3))];
        set_crop(&mut e, Some([0.1, 0.1, 0.6, 0.6]));
        // before the colour steps
        assert_eq!(e[0], EditOp::Crop([0.1, 0.1, 0.6, 0.6]));
        set_crop(&mut e, Some([0.0, 0.0, 0.9, 0.9]));
        assert_eq!(crop_of(&e), Some([0.0, 0.0, 0.9, 0.9]));
        assert_eq!(e.len(), 2);
        // the tool shows the picture uncropped, with its colours
        assert_eq!(crop_view(&e), vec![EditOp::Adjust(adj(0.3))]);
        set_crop(&mut e, None);
        assert_eq!(e, vec![EditOp::Adjust(adj(0.3))]);
    }

    #[test]
    fn the_crop_view_leaves_out_what_sits_on_the_cropped_picture() {
        let border = EditOp::Decor(crate::decor::Decor::Border { width: 0.05, bottom: 0.0, color: [255, 255, 255] });
        let e = vec![EditOp::Rotate(1), EditOp::Crop([0.0, 0.0, 0.5, 0.5]), EditOp::Adjust(adj(0.3)), border];
        assert_eq!(crop_view(&e), vec![EditOp::Rotate(1), EditOp::Adjust(adj(0.3))]);
        // no crop yet: the same view, the box will go after the turn
        let e = vec![EditOp::Rotate(1), EditOp::Adjust(adj(0.3))];
        assert_eq!(crop_view(&e), e);
    }

    #[test]
    fn turning_after_a_crop_is_the_same_as_turning_first() {
        let img = sample_image(3, 80, 40);
        for turns in [1, 2, 3, -1] {
            let r = [0.25, 0.5, 0.75, 1.0];
            let mut e = vec![EditOp::Crop(r), EditOp::Adjust(adj(0.3)), EditOp::Rotate(turns)];
            let before = apply_all(&img, &e);
            turns_before_crop(&mut e);
            assert!(matches!(e[0], EditOp::Rotate(_)) && matches!(e[1], EditOp::Crop(_)), "{e:?}");
            assert_eq!(e.len(), 3);
            let after = apply_all(&img, &e);
            assert_eq!(before.dimensions(), after.dimensions(), "turns {turns}");
            assert!(before.as_raw() == after.as_raw(), "turns {turns}");
        }
        // nothing to do, nothing changed
        let mut e = vec![EditOp::Rotate(1), EditOp::Crop([0.0, 0.0, 0.5, 0.5])];
        let same = e.clone();
        turns_before_crop(&mut e);
        assert_eq!(e, same);
    }

    #[test]
    fn grips() {
        let b = [0.2, 0.2, 0.8, 0.8];
        let reach = (0.02, 0.02);
        assert_eq!(grip_at(b, (0.5, 0.5), reach), Some(Grip::Inside));
        assert_eq!(grip_at(b, (0.2, 0.5), reach), Some(Grip::Sides { left: true, top: false, right: false, bottom: false }));
        assert_eq!(grip_at(b, (0.81, 0.79), reach), Some(Grip::Sides { left: false, top: false, right: true, bottom: true }));
        assert_eq!(grip_at(b, (0.1, 0.5), reach), None);
    }

    #[test]
    fn dragging_a_box_keeps_it_inside_and_in_shape() {
        let b = [0.2, 0.2, 0.6, 0.6];
        // moved against the edge
        let r = drag_box(b, Grip::Inside, (0.9, 0.0), None);
        assert!((r[0] - 0.6).abs() < 1e-6 && (r[2] - 1.0).abs() < 1e-6 && r[1] == 0.2, "{r:?}");
        // free: only the dragged side moves
        let right = Grip::Sides { left: false, top: false, right: true, bottom: false };
        let r = drag_box(b, right, (0.1, 0.5), None);
        assert!((r[2] - 0.7).abs() < 1e-6 && r[1] == 0.2 && r[3] == 0.6);
        // fixed shape, a corner dragged far past the picture's edge: still the shape, inside
        let corner = Grip::Sides { left: false, top: false, right: true, bottom: true };
        for shape in [1.0, 1.5, 0.5] {
            let r = drag_box(b, corner, (2.0, 2.0), Some(shape));
            let (w, h) = (r[2] - r[0], r[3] - r[1]);
            assert!((w / h - shape).abs() < 1e-4, "{r:?}");
            assert!(r[0] >= -1e-6 && r[1] >= -1e-6 && r[2] <= 1.0 + 1e-6 && r[3] <= 1.0 + 1e-6, "{r:?}");
            // the opposite corner did not move
            assert!((r[0] - 0.2).abs() < 1e-6 && (r[1] - 0.2).abs() < 1e-6);
        }
        // an edge with a fixed shape grows around the middle of the opposite edge
        let r = drag_box(b, right, (0.2, 0.0), Some(1.0));
        assert!(((r[2] - r[0]) - (r[3] - r[1])).abs() < 1e-5 && (r[0] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn drawing_a_box_keeps_its_shape_at_the_edge() {
        // free
        assert_eq!(drawn_box((0.2, 0.2), (0.6, 0.5), None), Some([0.2, 0.2, 0.6, 0.5]));
        assert_eq!(drawn_box((0.2, 0.2), (0.205, 0.5), None), None);
        // dragged far to the right from near the bottom: the height runs out, the width follows
        let r = drawn_box((0.1, 0.8), (0.9, 0.95), Some(1.0)).unwrap();
        assert!(((r[2] - r[0]) - (r[3] - r[1])).abs() < 1e-6 && r[3] <= 1.0 + 1e-6, "{r:?}");
        // upwards and to the left
        let r = drawn_box((0.5, 0.5), (0.3, 0.1), Some(2.0)).unwrap();
        assert!((r[2] - 0.5).abs() < 1e-6 && (r[3] - 0.5).abs() < 1e-6 && ((r[2] - r[0]) / (r[3] - r[1]) - 2.0).abs() < 1e-4);
    }
}
