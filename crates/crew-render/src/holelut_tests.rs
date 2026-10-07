use super::*;

/// The edge of the shadow is where light starts getting away: `3√3` masses.
const B_CRIT: f32 = 5.196_152;

fn fell(b: f32) -> bool {
    column(b).contains(&FELL)
}

#[test]
fn rays_inside_the_critical_impact_parameter_fall_in_and_outside_it_escape() {
    assert!(fell(B_CRIT - 0.05), "just inside the shadow falls in");
    assert!(!fell(B_CRIT + 0.05), "just outside it gets away");
    assert!(fell(1.0) && !fell(12.0));
}

/// Far from the hole a ray barely bends: it comes closest a quarter turn
/// in, as a straight line would, at the radius Schwarzschild gives for its
/// impact parameter (`b² = r³ / (r − 2)`, about `b − 1`).
#[test]
fn a_distant_ray_runs_almost_straight() {
    let c = column(20.0);
    let (i, peak) = c
        .iter()
        .enumerate()
        .fold((0, 0.0f32), |m, (i, &u)| if u > m.1 { (i, u) } else { m });
    let psi = PSI_MAX * i as f32 / (H - 1) as f32;
    let r = 1.0 / peak;
    let b = (r * r * r / (r - 2.0)).sqrt();
    assert!(
        (b - 20.0).abs() < 0.05,
        "closest approach {r} implies b {b}"
    );
    let quarter = std::f32::consts::FRAC_PI_2;
    assert!((psi - quarter).abs() < 0.15, "closest at ψ {psi}");
}

/// A ray grazing the photon sphere turns well past a half-turn before it
/// leaves: that is what lifts the far side of the disk over the shadow.
#[test]
fn a_grazing_ray_wraps_round_the_hole() {
    let c = column(B_CRIT + 0.3);
    let last_near = c.iter().rposition(|&u| u > 0.05).unwrap();
    let psi = PSI_MAX * last_near as f32 / (H - 1) as f32;
    assert!(psi > 1.4 * std::f32::consts::PI, "left at ψ {psi}");
}

#[test]
fn halves_round_trip_the_values_the_table_holds() {
    for x in [0.0f32, 0.05, 0.1666, 0.333, 0.5, 1.0] {
        let h = half(x);
        let back = f32::from_bits(
            ((((h >> 10) & 0x1f) as u32 + 127 - 15) << 23) | (((h & 0x3ff) as u32) << 13),
        );
        let back = if h == 0 { 0.0 } else { back };
        assert!((back - x).abs() <= x * 1e-3 + 1e-6, "{x} -> {back}");
    }
    assert_eq!(table().len(), (W * H) as usize);
}

/// The shader reads the table through its own copy of the extent and size:
/// a change to one side alone would read every ray at the wrong place.
#[test]
fn the_shader_reads_the_table_this_file_writes() {
    let wgsl = include_str!("paperhole.wgsl");
    for line in [
        format!("const LUT_B_MAX: f32 = {B_MAX:.1};"),
        format!("const LUT_PSI_MAX: f32 = {PSI_MAX:.6};"),
        format!("const LUT_W: f32 = {W}.0;"),
        format!("const LUT_H: f32 = {H}.0;"),
    ] {
        assert!(wgsl.contains(&line), "paperhole.wgsl lacks `{line}`");
    }
}
