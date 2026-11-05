use super::*;

#[test]
fn a_crew_starts_with_one_tab_on_screen() {
    let t = Tabs::default();
    assert_eq!(t.list.len(), 1);
    assert_eq!(t.active, t.list[0].id);
    assert_eq!(t.step(1), None, "nowhere to step with one tab");
}

#[test]
fn a_new_tab_opens_beside_the_one_on_screen_and_a_pushed_one_at_the_end() {
    let mut t = Tabs::default();
    let b = t.add();
    let c = t.push();
    // Back on the first: Cmd+T there opens between it and `b`.
    let d = t.add();
    let order: Vec<u32> = t.list.iter().map(|t| t.id).collect();
    assert_eq!(order, vec![1, d, b, c]);
    assert_eq!(t.id_at(1), Some(1));
    assert_eq!(t.id_at(4), Some(c));
    assert_eq!(t.id_at(0), None);
    assert_eq!(t.id_at(5), None);
}

#[test]
fn stepping_wraps_both_ways() {
    let mut t = Tabs::default();
    let b = t.add();
    let c = t.push();
    assert_eq!(t.step(1), Some(b));
    assert_eq!(t.step(-1), Some(c));
    t.active = c;
    assert_eq!(t.step(1), Some(1));
}

#[test]
fn an_emptied_tab_goes_and_its_left_neighbour_comes_on() {
    let mut t = Tabs::default();
    let b = t.add();
    let c = t.push();
    t.active = c;
    assert_eq!(t.prune(&[1, b]), Some(b));
    assert_eq!(t.list.len(), 2);
    // A background tab emptied goes quietly.
    t.active = 1;
    assert_eq!(t.prune(&[1]), None);
    assert_eq!(t.list.len(), 1);
}

#[test]
fn the_last_tab_stays_even_with_nothing_in_it() {
    let mut t = Tabs::default();
    let b = t.add();
    t.active = b;
    assert_eq!(t.prune(&[]), None);
    assert_eq!(t.list.len(), 1);
    assert_eq!(t.list[0].id, b);
}

#[test]
fn a_close_shifts_every_parked_grid() {
    let mut t = Tabs::default();
    let b = t.add();
    t.get_mut(b).expect("tab").grid.add(3);
    t.on_close(1);
    assert_eq!(t.get_mut(b).expect("tab").grid.recent(), &[2]);
}
