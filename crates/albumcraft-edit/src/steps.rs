//! Editing a picture's list of edit steps the way the editor's panels do. The list is a recipe
//! with a fixed order, not a history: the steps that move or cut the picture first (turn,
//! straighten, perspective, crop), then the colours (adjustment, look), then what is drawn on or
//! around the picture (vignette, rounded corners, border, text, shadow). There is one step of
//! each kind (only texts and red-eye fixes can be several), changed where it stands; a new step
//! goes to its place in that order (`rank`), wherever it is added from.
//!
//! The list itself (`Vec<EditOp>`, applied in order by `imaging::apply_all`) stays what is
//! saved, and lists made before this order existed are left as they are, so those pictures keep
//! their look: when such a list holds several steps of a kind, the last one is "the" one edited
//! here, and a new step goes after the last step that belongs before it.

use crate::decor::Decor;
use crate::develop::Adjust;
use crate::imaging::EditOp;
use crate::looks::Look;

fn last(edits: &[EditOp], is: impl Fn(&EditOp) -> bool) -> Option<usize> {
    edits.iter().rposition(is)
}

/// Where a step stands in the fixed order (lower first; steps of the same rank stay in the order
/// they were added).
pub fn rank(op: &EditOp) -> u8 {
    match op {
        EditOp::Rotate(_) => 0,
        EditOp::Straighten(_) => 1,
        EditOp::Perspective { .. } => 2,
        EditOp::Crop(_) => 3,
        EditOp::Redeye(_) => 4,
        EditOp::Lucky | EditOp::AutoContrast | EditOp::AutoColor | EditOp::Tune { .. } => 5,
        EditOp::Adjust(_) => 6,
        EditOp::Look { .. } => 7,
        EditOp::Effect(_) => 8,
        EditOp::Decor(Decor::Vignette { .. }) => 9,
        EditOp::Decor(Decor::Rounded { .. }) => 10,
        EditOp::Decor(Decor::Border { .. }) => 11,
        EditOp::Decor(Decor::Text(_)) => 12,
        EditOp::Decor(Decor::Shadow { .. }) => 13,
    }
}

/// Where a new step of that rank goes: after the last step that belongs before it or beside it.
fn place(edits: &[EditOp], rank_of_new: u8) -> usize {
    last(edits, |e| rank(e) <= rank_of_new).map_or(0, |i| i + 1)
}

/// The list is in the fixed order, with one step of each kind that has only one.
pub fn in_order(edits: &[EditOp]) -> bool {
    edits.windows(2).all(|w| {
        let (a, b) = (rank(&w[0]), rank(&w[1]));
        a < b || (a == b && matches!(a, 4 | 5 | 8 | 12))
    })
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

/// Set the picture's adjustment: changed where it stands, added at its place when there is
/// none, removed when it is neutral.
pub fn set_adjust(edits: &mut Vec<EditOp>, a: Adjust) {
    let a = a.clamped();
    match last(edits, |e| matches!(e, EditOp::Adjust(_))) {
        Some(i) if a.is_neutral() => {
            edits.remove(i);
        }
        Some(i) => edits[i] = EditOp::Adjust(a),
        None if a.is_neutral() => {}
        None => edits.insert(place(edits, 6), EditOp::Adjust(a)),
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
        // under the effects drawn on the picture (borders, text), which stay on top
        (None, Some((look, amount))) => edits.insert(place(edits, 7), EditOp::Look { look, amount: amount.clamp(0.0, 1.0) }),
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
        None => edits.insert(place(edits, rank(&op)), op),
    }
}

/// Where a new crop goes: after the steps that move the picture, before the others.
fn crop_place(edits: &[EditOp]) -> usize {
    place(edits, 3)
}

/// Which effect of the Effects tab: the one vignette, rounded corners, border or shadow, or one
/// of the texts (counted from the first).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Vignette,
    Rounded,
    Border,
    Shadow,
    Text(usize),
}

fn is_text(op: &EditOp) -> bool {
    matches!(op, EditOp::Decor(Decor::Text(_)))
}

/// The slot of the step at `i`, when it is an effect.
pub fn slot_at(edits: &[EditOp], i: usize) -> Option<Slot> {
    match edits.get(i)? {
        EditOp::Decor(Decor::Vignette { .. }) => Some(Slot::Vignette),
        EditOp::Decor(Decor::Rounded { .. }) => Some(Slot::Rounded),
        EditOp::Decor(Decor::Border { .. }) => Some(Slot::Border),
        EditOp::Decor(Decor::Shadow { .. }) => Some(Slot::Shadow),
        EditOp::Decor(Decor::Text(_)) => Some(Slot::Text(edits[..i].iter().filter(|e| is_text(e)).count())),
        _ => None,
    }
}

/// Where the effect of that slot stands in the list (the last one, in a list with several).
fn slot_index(edits: &[EditOp], slot: Slot) -> Option<usize> {
    match slot {
        Slot::Text(n) => edits.iter().enumerate().filter(|(_, e)| is_text(e)).nth(n).map(|(i, _)| i),
        _ => (0..edits.len()).rev().find(|&i| slot_at(edits, i) == Some(slot)),
    }
}

/// The effect in that slot.
pub fn decor_of(edits: &[EditOp], slot: Slot) -> Option<Decor> {
    match slot_index(edits, slot).and_then(|i| edits.get(i)) {
        Some(EditOp::Decor(d)) => Some(d.clone()),
        _ => None,
    }
}

/// Add an effect: a vignette, rounded corners, border or shadow takes the place of the one the
/// picture has; a text is one more text. The slot it is in.
pub fn add_decor(edits: &mut Vec<EditOp>, d: Decor) -> Slot {
    let op = EditOp::Decor(d);
    let at = match slot_at(std::slice::from_ref(&op), 0) {
        Some(slot @ (Slot::Vignette | Slot::Rounded | Slot::Border | Slot::Shadow)) => slot_index(edits, slot),
        _ => None,
    };
    let i = match at {
        Some(i) => {
            edits[i] = op;
            i
        }
        None => {
            let i = place(edits, rank(&op));
            edits.insert(i, op);
            i
        }
    };
    slot_at(edits, i).unwrap_or(Slot::Text(0))
}

/// Change the effect in that slot (to one of the same kind), or remove it with None.
pub fn set_decor(edits: &mut Vec<EditOp>, slot: Slot, d: Option<Decor>) {
    let Some(i) = slot_index(edits, slot) else { return };
    match d {
        None => {
            edits.remove(i);
        }
        Some(d) => {
            if let EditOp::Decor(cur) = &mut edits[i]
                && cur.same_kind(&d)
            {
                *cur = d;
            }
        }
    }
}

/// The perspective sliders' values after `turns` quarter turns clockwise of the picture.
fn turn_perspective((mut v, mut h): (f32, f32), turns: i32) -> (f32, f32) {
    for _ in 0..turns.rem_euclid(4) {
        // the top becomes the right side, the right side the bottom
        (v, h) = (-h, v);
    }
    (v, h)
}

/// Turn the picture by `dir` quarter turns clockwise. The turn joins the picture's one turn
/// step at the start of the list: the straighten, perspective and crop made since are turned
/// with it, so the picture comes out the same, only turned. What is drawn on the picture after
/// (border, text, shadow) is not turned: a caption stays readable under the turned picture.
pub fn turn(edits: &mut Vec<EditOp>, dir: i32) {
    let dir = dir.rem_euclid(4);
    if dir == 0 {
        return;
    }
    let mut i = edits.iter().position(|e| matches!(e, EditOp::Decor(_))).unwrap_or(edits.len());
    while i > 0 {
        match &mut edits[i - 1] {
            EditOp::Rotate(n) => {
                // -1…2, so that a quarter turn left is still called that
                let sum = (*n + dir).rem_euclid(4);
                if sum == 0 {
                    edits.remove(i - 1);
                } else {
                    *n = if sum == 3 { -1 } else { sum };
                }
                return;
            }
            EditOp::Crop(r) | EditOp::Redeye(r) => *r = turn_rect(*r, dir),
            EditOp::Perspective { vertical, horizontal } => (*vertical, *horizontal) = turn_perspective((*vertical, *horizontal), dir),
            // the same whichever way up the picture is
            EditOp::Straighten(_) | EditOp::Adjust(_) | EditOp::Look { .. } => {}
            // older colour steps, some of which depend on the way up: the turn stays after them
            _ => break,
        }
        i -= 1;
    }
    edits.insert(i, EditOp::Rotate(if dir == 3 { -1 } else { dir }));
}

/// Put a step into the list the way the editor would: a turn joins the turn, an adjustment,
/// look, straighten, perspective, crop, vignette, rounded corners, border or shadow takes the
/// place of the one the picture has, a text or red-eye fix is added at its place in the order.
pub fn put(edits: &mut Vec<EditOp>, op: EditOp) {
    match op {
        EditOp::Rotate(n) => turn(edits, n),
        EditOp::Straighten(_) | EditOp::Perspective { .. } => set_geometry(edits, op),
        EditOp::Crop(r) => set_crop(edits, Some(r)),
        EditOp::Adjust(a) => set_adjust(edits, a),
        EditOp::Look { look, amount } => set_look(edits, Some((look, amount))),
        EditOp::Decor(d) => {
            // the same text twice (edits pasted again) is one text
            if !(matches!(d, Decor::Text(_)) && edits.iter().any(|e| matches!(e, EditOp::Decor(x) if *x == d))) {
                add_decor(edits, d);
            }
        }
        _ => edits.insert(place(edits, rank(&op)), op),
    }
}

/// Put every one of `ops` into the list (pasted edits).
pub fn merge(edits: &mut Vec<EditOp>, ops: &[EditOp]) {
    for op in ops {
        put(edits, op.clone());
    }
}

/// Take the step at `i` out of the list. A turn is taken back as a turn the other way, so that
/// the crop and perspective made on the turned picture still fit.
pub fn remove_step(edits: &mut Vec<EditOp>, i: usize) {
    match edits.get(i) {
        Some(EditOp::Rotate(n)) => {
            let n = *n;
            turn(edits, -n);
        }
        Some(_) => {
            edits.remove(i);
        }
        None => {}
    }
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

    fn border() -> Decor {
        Decor::Border { width: 0.05, bottom: 0.0, color: [255, 255, 255] }
    }

    fn text(t: &str) -> Decor {
        Decor::Text(crate::decor::TextOverlay { text: t.into(), ..Default::default() })
    }

    #[test]
    fn steps_go_to_their_place_whatever_the_order_they_are_made_in() {
        let mut e = vec![];
        put(&mut e, EditOp::Decor(border()));
        put(&mut e, EditOp::Decor(Decor::Vignette { amount: 0.5, size: 0.5 }));
        put(&mut e, EditOp::Look { look: Look::Vivid, amount: 0.5 });
        put(&mut e, EditOp::Adjust(adj(0.3)));
        put(&mut e, EditOp::Crop([0.1, 0.1, 0.9, 0.9]));
        put(&mut e, EditOp::Decor(text("a")));
        put(&mut e, EditOp::Straighten(2.0));
        put(&mut e, EditOp::Decor(Decor::Shadow { size: 0.04, opacity: 0.5, background: [255, 255, 255] }));
        put(&mut e, EditOp::Rotate(1));
        put(&mut e, EditOp::Perspective { vertical: 0.2, horizontal: 0.0 });
        put(&mut e, EditOp::Decor(Decor::Rounded { radius: 0.05, background: [255, 255, 255] }));
        let ranks: Vec<u8> = e.iter().map(rank).collect();
        assert_eq!(ranks, vec![0, 1, 2, 3, 6, 7, 9, 10, 11, 12, 13], "{e:?}");
        assert!(in_order(&e));
        // the adjustment is under the border, not on it
        assert!(e.iter().position(|x| matches!(x, EditOp::Adjust(_))) < e.iter().position(|x| matches!(x, EditOp::Decor(_))));
    }

    #[test]
    fn one_effect_of_each_kind_but_several_texts() {
        let mut e = vec![];
        assert_eq!(add_decor(&mut e, border()), Slot::Border);
        // a second border (here the instant frame) takes the place of the first
        let instant = Decor::Border { width: 0.05, bottom: 0.15, color: [250, 248, 242] };
        assert_eq!(add_decor(&mut e, instant.clone()), Slot::Border);
        assert_eq!(e, vec![EditOp::Decor(instant.clone())]);
        assert_eq!(add_decor(&mut e, text("a")), Slot::Text(0));
        assert_eq!(add_decor(&mut e, text("b")), Slot::Text(1));
        assert_eq!(e.len(), 3);
        assert_eq!(decor_of(&e, Slot::Text(1)), Some(text("b")));
        assert_eq!(slot_at(&e, 2), Some(Slot::Text(1)));
        // changed in its slot; another kind is not taken
        set_decor(&mut e, Slot::Text(0), Some(text("c")));
        set_decor(&mut e, Slot::Border, Some(text("x")));
        assert_eq!(e, vec![EditOp::Decor(instant), EditOp::Decor(text("c")), EditOp::Decor(text("b"))]);
        set_decor(&mut e, Slot::Text(0), None);
        assert_eq!(decor_of(&e, Slot::Text(0)), Some(text("b")));
        set_decor(&mut e, Slot::Border, None);
        set_decor(&mut e, Slot::Shadow, None);
        assert_eq!(e, vec![EditOp::Decor(text("b"))]);
    }

    #[test]
    fn pasted_edits_take_the_place_of_the_pictures_own() {
        let mut e = vec![EditOp::Adjust(adj(0.3)), EditOp::Decor(border()), EditOp::Decor(text("a"))];
        let pasted = [EditOp::Adjust(adj(0.8)), EditOp::Look { look: Look::Sepia, amount: 1.0 }, EditOp::Decor(border()), EditOp::Decor(text("a"))];
        merge(&mut e, &pasted);
        assert_eq!(e, vec![EditOp::Adjust(adj(0.8)), EditOp::Look { look: Look::Sepia, amount: 1.0 }, EditOp::Decor(border()), EditOp::Decor(text("a"))]);
        // pasted again: nothing more
        let same = e.clone();
        merge(&mut e, &pasted);
        assert_eq!(e, same);
    }

    #[test]
    fn lists_made_before_keep_their_steps() {
        // two borders and an adjustment after them, as older versions could make
        let old = vec![EditOp::Decor(border()), EditOp::Decor(border()), EditOp::Adjust(adj(0.3))];
        assert!(!in_order(&old));
        let mut e = old.clone();
        set_adjust(&mut e, adj(0.5));
        assert_eq!(e.len(), 3);
        assert_eq!(e[2], EditOp::Adjust(adj(0.5)));
        assert_eq!(e[..2], old[..2]);
    }

    /// Mean difference per channel value between two pictures of the same size.
    fn difference(a: &image::RgbaImage, b: &image::RgbaImage) -> f32 {
        assert_eq!(a.dimensions(), b.dimensions());
        let sum: u64 = a.as_raw().iter().zip(b.as_raw()).map(|(x, y)| u64::from(x.abs_diff(*y))).sum();
        sum as f32 / a.as_raw().len() as f32
    }

    #[test]
    fn a_turn_keeps_the_picture_and_turns_it() {
        let img = sample_image(3, 120, 80);
        let geometry = vec![
            EditOp::Straighten(3.0),
            EditOp::Perspective { vertical: 0.5, horizontal: -0.3 },
            EditOp::Crop([0.25, 0.25, 0.75, 1.0]),
            EditOp::Adjust(adj(0.3)),
        ];
        for dir in [1, 2, 3, -1] {
            // turned after everything else: what the result has to look like
            let mut after = geometry.clone();
            after.push(EditOp::Rotate(dir));
            let want = apply_all(&img, &after);
            let mut e = geometry.clone();
            turn(&mut e, dir);
            assert!(matches!(e[0], EditOp::Rotate(_)) && in_order(&e), "{e:?}");
            let got = apply_all(&img, &e);
            // not pixel for pixel (the straighten samples between pixels), but the same picture
            assert!(difference(&want, &got) < 2.0, "turn {dir}: {}", difference(&want, &got));
            // and a wrong turn of the perspective would not be
            let mut wrong = e.clone();
            wrong[2] = EditOp::Perspective { vertical: 0.5, horizontal: -0.3 };
            if dir.rem_euclid(4) != 2 {
                assert!(difference(&want, &apply_all(&img, &wrong)) > difference(&want, &got), "turn {dir}");
            }
        }
    }

    #[test]
    fn turns_join_and_leave_the_effects_as_they_are() {
        let mut e = vec![EditOp::Crop([0.0, 0.0, 0.5, 1.0]), EditOp::Decor(text("a"))];
        turn(&mut e, 1);
        assert_eq!(e, vec![EditOp::Rotate(1), EditOp::Crop(turn_rect([0.0, 0.0, 0.5, 1.0], 1)), EditOp::Decor(text("a"))]);
        turn(&mut e, 1);
        assert_eq!(e[0], EditOp::Rotate(2));
        turn(&mut e, 1);
        assert_eq!(e[0], EditOp::Rotate(-1));
        turn(&mut e, 1);
        assert_eq!(e, vec![EditOp::Crop([0.0, 0.0, 0.5, 1.0]), EditOp::Decor(text("a"))]);
        // removing the turn step takes the crop back with it
        turn(&mut e, -1);
        remove_step(&mut e, 0);
        assert_eq!(e, vec![EditOp::Crop([0.0, 0.0, 0.5, 1.0]), EditOp::Decor(text("a"))]);
        remove_step(&mut e, 0);
        assert_eq!(e, vec![EditOp::Decor(text("a"))]);
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
