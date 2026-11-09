use super::*;

#[test]
fn a_saved_setting_keeps_only_a_scene_it_knows() {
    assert_eq!(clean("orbit".into()), "orbit");
    assert_eq!(clean(" orbit ".into()), "orbit");
    assert_eq!(clean("galaxy".into()), "");
    assert_eq!(clean(String::new()), "");
}

#[test]
fn the_ladder_offers_off_then_every_scene() {
    let rows = ladder();
    assert_eq!(rows[0].0, "off");
    assert_eq!(rows.len(), 1 + Stage::ALL.len());
    assert!(rows.iter().any(|(n, _)| n == "orbit"));
}
