use super::uniform;

#[test]
fn the_uniform_carries_the_grid_and_the_clock() {
    let b = crew_theme::holo::Backdrop {
        line: (255, 0, 0),
        spark: (0, 255, 0),
        spark2: (0, 0, 255),
        strength: 0.5,
    };
    let u = uniform(&b, (800.0, 600.0), (8.0, 17.0), 0.25, false);
    assert_eq!(&u[..4], &[800.0, 600.0, 8.0, 17.0]);
    assert_eq!(
        &u[4..8],
        &[1.0, 0.0, 0.0, 0.5],
        "the line, its alpha the strength"
    );
    assert_eq!(&u[8..12], &[0.0, 1.0, 0.0, 1.0]);
    assert_eq!(&u[12..16], &[0.0, 0.0, 1.0, 1.0]);
    assert_eq!(u[16], 0.25);
}
