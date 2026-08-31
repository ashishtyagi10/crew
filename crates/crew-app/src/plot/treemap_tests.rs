use super::layout;

fn area(t: &super::Tile) -> f32 {
    t.w * t.h
}

#[test]
fn a_tiles_area_is_its_share_of_the_rect() {
    let tiles = layout((0.0, 0.0, 10.0, 10.0), &[50.0, 25.0, 25.0]);
    assert_eq!(tiles.len(), 3);
    let total: f32 = tiles.iter().map(area).sum();
    assert!((total - 100.0).abs() < 0.5, "the rect is filled: {total}");
    let half = tiles.iter().find(|t| t.index == 0).unwrap();
    assert!(
        (area(half) - 50.0).abs() < 1.0,
        "half the value is half the area: {}",
        area(half)
    );
}

#[test]
fn tiles_do_not_overlap() {
    let tiles = layout(
        (0.0, 0.0, 12.0, 8.0),
        &[40.0, 22.0, 13.0, 9.0, 8.0, 5.0, 3.0],
    );
    for (i, a) in tiles.iter().enumerate() {
        for b in tiles.iter().skip(i + 1) {
            let sep = a.x + a.w <= b.x + 1e-3
                || b.x + b.w <= a.x + 1e-3
                || a.y + a.h <= b.y + 1e-3
                || b.y + b.h <= a.y + 1e-3;
            assert!(sep, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn every_tile_stays_inside_the_rect() {
    let tiles = layout((2.0, 3.0, 10.0, 6.0), &[9.0, 8.0, 7.0, 1.0, 1.0]);
    for t in &tiles {
        assert!(t.x >= 2.0 - 1e-3 && t.x + t.w <= 12.0 + 1e-2, "{t:?}");
        assert!(t.y >= 3.0 - 1e-3 && t.y + t.h <= 9.0 + 1e-2, "{t:?}");
    }
}

#[test]
fn tiles_are_squarish_rather_than_slivers() {
    // The whole point of squarifying: a naive slice-and-dice layout gives
    // the small items aspect ratios in the tens.
    let vals: Vec<f64> = (1..=12).map(|i| i as f64 * 3.0).collect();
    let tiles = layout((0.0, 0.0, 16.0, 10.0), &vals);
    let worst = tiles
        .iter()
        .map(|t| (t.w / t.h).max(t.h / t.w))
        .fold(0.0f32, f32::max);
    assert!(worst < 5.0, "worst aspect ratio {worst}");
}

#[test]
fn a_value_of_zero_gets_no_tile() {
    let tiles = layout((0.0, 0.0, 8.0, 4.0), &[10.0, 0.0, 5.0]);
    assert_eq!(tiles.len(), 2);
    assert!(tiles.iter().all(|t| t.index != 1));
}

#[test]
fn nothing_to_lay_out_lays_nothing_out() {
    assert!(layout((0.0, 0.0, 8.0, 4.0), &[]).is_empty());
    assert!(layout((0.0, 0.0, 8.0, 4.0), &[0.0, 0.0]).is_empty());
    assert!(layout((0.0, 0.0, 0.0, 4.0), &[1.0]).is_empty());
}

#[test]
fn the_biggest_tile_is_the_one_you_point_at_first() {
    // Descending input puts the largest tile at the origin corner.
    let tiles = layout((0.0, 0.0, 10.0, 10.0), &[60.0, 30.0, 10.0]);
    let first = tiles.iter().find(|t| t.index == 0).unwrap();
    assert!(first.x < 1e-3 && first.y < 1e-3, "{first:?}");
}

#[test]
fn hit_testing_finds_the_tile_under_a_point() {
    let tiles = layout((0.0, 0.0, 10.0, 10.0), &[50.0, 30.0, 20.0]);
    for t in &tiles {
        let (cx, cy) = (t.x + t.w / 2.0, t.y + t.h / 2.0);
        let hit = tiles.iter().filter(|o| o.contains(cx, cy)).count();
        assert_eq!(hit, 1, "one tile owns its own centre");
    }
}
