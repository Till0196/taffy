#![cfg(feature = "float_layout")]
use taffy::geometry::Point;
use taffy::prelude::*;
use taffy::style::{Clear, Float};
use taffy_test_helpers::new_test_tree;

/// Regression test for <https://wpt.live/css/CSS2/floats-clear/floats-146.xht>
///
/// A right float placed after two left floats must not be placed higher than the
/// outer top of the earlier floats (CSS2 float rule 5). This scenario previously
/// panicked in `FloatContext::subdivide_segment` when the float's end coincided
/// with an existing segment boundary due to floating point imprecision.
#[test]
fn float_no_higher_than_earlier_floats() {
    let mut taffy = new_test_tree();

    let block = |width: f32, height: f32, float: Float| Style {
        display: Display::Block,
        float,
        size: Size { width: length(width), height: length(height) },
        ..Default::default()
    };

    // Spacer block advancing the flow position to a fractional y offset (43.2)
    let spacer = taffy.new_leaf(block(400.0, 43.2, Float::None)).unwrap();
    let float_a = taffy.new_leaf(block(258.0, 42.0, Float::Left)).unwrap();
    let float_b = taffy.new_leaf(block(298.0, 42.0, Float::Left)).unwrap();
    let float_c = taffy.new_leaf(block(98.0, 42.0, Float::Right)).unwrap();

    let root = taffy
        .new_with_children(
            Style {
                display: Display::Block,
                size: Size { width: length(400.0), height: auto() },
                ..Default::default()
            },
            &[spacer, float_a, float_b, float_c],
        )
        .unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let layout_a = taffy.layout(float_a).unwrap();
    let layout_b = taffy.layout(float_b).unwrap();
    let layout_c = taffy.layout(float_c).unwrap();

    // A is placed below the spacer
    assert_eq!(layout_a.location, Point { x: 0.0, y: 43.0 });
    // B does not fit next to A, so is placed below it
    assert_eq!(layout_b.location, Point { x: 0.0, y: 85.0 });
    // C must not be placed higher than B (it fits next to B on the right)
    assert_eq!(layout_c.location, Point { x: 302.0, y: 85.0 });
}

/// Regression test for <https://wpt.live/css/CSS2/floats-clear/margin-collapse-033.xht>
///
/// Floats that occupy no horizontal space (e.g. a zero-width float) still affect
/// `clear`: a cleared block must be placed past the bottom of all floats on the
/// relevant side, regardless of their width.
#[test]
fn clear_past_zero_width_float() {
    let mut taffy = new_test_tree();

    // float: left with auto width resolving to 0
    let floated = taffy
        .new_leaf(Style {
            display: Display::Block,
            float: Float::Left,
            size: Size { width: auto(), height: length(1.0) },
            ..Default::default()
        })
        .unwrap();
    let cleared = taffy.new_leaf(Style { display: Display::Block, clear: Clear::Left, ..Default::default() }).unwrap();
    let sibling = taffy
        .new_leaf(Style {
            display: Display::Block,
            margin: Rect { top: length(99.0), ..Rect::zero() },
            ..Default::default()
        })
        .unwrap();

    let container = taffy
        .new_with_children(
            Style {
                display: Display::Block,
                size: Size { width: length(100.0), height: auto() },
                ..Default::default()
            },
            &[floated, cleared, sibling],
        )
        .unwrap();
    let root = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[container]).unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    // The cleared (self-collapsing) block is placed below the float (y=1). Its clearance
    // prevents the sibling's 99px margin from collapsing with preceding margins, so the
    // container's height is 1 + 99 = 100.
    assert_eq!(taffy.layout(container).unwrap().size.height, 100.0);
}

fn float_block(width: f32, height: f32, float: Float) -> Style {
    Style {
        display: Display::Block,
        float,
        size: Size { width: length(width), height: length(height) },
        ..Default::default()
    }
}

fn root_style(width: f32) -> Style {
    Style { display: Display::Block, size: Size { width: length(width), height: auto() }, ..Default::default() }
}

/// Regression test for <https://wpt.live/css/CSS2/floats/floats-rule3-outside-left-001.xht>
///
/// CSS2 float rule 7: a float that is wider than its containing block and has no other
/// float beside it may overflow the containing block's edge rather than being pushed down.
#[test]
fn oversized_float_overflows_containing_block() {
    let mut taffy = new_test_tree();

    let float_a = taffy.new_leaf(float_block(150.0, 50.0, Float::Left)).unwrap();
    let root = taffy.new_with_children(root_style(100.0), &[float_a]).unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    assert_eq!(taffy.layout(float_a).unwrap().location, Point { x: 0.0, y: 0.0 });
}

/// CSS2 float rules 3 & 7: a float with another float beside it must not overlap that float
/// (rule 3) nor extend past the containing block's opposite edge (rule 7) - it must move down.
#[test]
fn float_beside_existing_float_moves_down_instead_of_overflowing() {
    let mut taffy = new_test_tree();

    // Rule 3: an opposite-direction float that doesn't fit must move down, not overlap
    let left = taffy.new_leaf(float_block(60.0, 50.0, Float::Left)).unwrap();
    let right = taffy.new_leaf(float_block(60.0, 50.0, Float::Right)).unwrap();
    // Rule 7: a same-direction float that doesn't fit must move down, not overflow the
    // containing block's trailing edge
    let left2 = taffy.new_leaf(float_block(60.0, 50.0, Float::Left)).unwrap();

    let root = taffy.new_with_children(root_style(100.0), &[left, right, left2]).unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    assert_eq!(taffy.layout(left).unwrap().location, Point { x: 0.0, y: 0.0 });
    assert_eq!(taffy.layout(right).unwrap().location, Point { x: 40.0, y: 50.0 });
    assert_eq!(taffy.layout(left2).unwrap().location, Point { x: 0.0, y: 100.0 });
}

/// A float inside a block that is pulled up by a negative `margin-top` must be placed at the
/// top of its containing block (CSS2 float rule 4), even when that top is above the top of the
/// block formatting context. The float's position was previously clamped to `y = 0` in the
/// formatting context, dropping it by the size of the negative margin.
#[test]
fn float_in_block_with_negative_margin_top() {
    let mut taffy = new_test_tree();

    let spacer = taffy.new_leaf(float_block(400.0, 500.0, Float::None)).unwrap();
    let floated_left = taffy.new_leaf(float_block(100.0, 50.0, Float::Left)).unwrap();
    let floated_right = taffy.new_leaf(float_block(100.0, 50.0, Float::Right)).unwrap();
    let pulled_up = taffy
        .new_with_children(
            Style {
                display: Display::Block,
                margin: Rect { top: length(-455.0), ..Rect::zero() },
                size: Size { width: length(400.0), height: auto() },
                ..Default::default()
            },
            &[floated_left, floated_right],
        )
        .unwrap();
    let root = taffy.new_with_children(root_style(400.0), &[spacer, pulled_up]).unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    // The pulled-up block starts 455px above the bottom of the spacer
    assert_eq!(taffy.layout(pulled_up).unwrap().location, Point { x: 0.0, y: 45.0 });
    // The floats sit at the top of the pulled-up block (relative to it)
    assert_eq!(taffy.layout(floated_left).unwrap().location, Point { x: 0.0, y: 0.0 });
    assert_eq!(taffy.layout(floated_right).unwrap().location, Point { x: 300.0, y: 0.0 });
}

/// Same as above, but the block formatting context itself starts with the float: the first
/// float in the context must not be clamped to `y = 0` either.
#[test]
fn first_float_in_context_above_context_top() {
    let mut taffy = new_test_tree();

    let floated = taffy.new_leaf(float_block(100.0, 50.0, Float::Left)).unwrap();
    let pulled_up = taffy
        .new_with_children(
            Style {
                display: Display::Block,
                margin: Rect { top: length(-30.0), ..Rect::zero() },
                size: Size { width: length(400.0), height: auto() },
                ..Default::default()
            },
            &[floated],
        )
        .unwrap();
    let root = taffy
        .new_with_children(
            Style { padding: Rect { top: length(1.0), ..Rect::zero() }, ..root_style(400.0) },
            &[pulled_up],
        )
        .unwrap();

    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();

    assert_eq!(taffy.layout(pulled_up).unwrap().location, Point { x: 0.0, y: -29.0 });
    assert_eq!(taffy.layout(floated).unwrap().location, Point { x: 0.0, y: 0.0 });
}
