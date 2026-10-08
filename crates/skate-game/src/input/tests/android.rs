use super::android::Shared;
use super::*;
use skate_core::input::xbox;

fn pad(shared: &Shared, id: i32) {
    shared.connected(id, "Backbone One", 0x358a, 1, &[]);
}

fn st(buttons: u16) -> XboxState {
    android_state(buttons, [0; 4], [0; 2])
}

#[test]
fn android_state_clamps_to_field_ranges() {
    let s = android_state(0x1000, [40000, -40000, i32::MAX, i32::MIN], [-5, 999]);
    assert_eq!((s.left, s.right, s.triggers), ([i16::MAX, i16::MIN], [i16::MAX, i16::MIN], [0, 255]));
}

#[test]
fn android_state_passes_buttons_and_up_positive_y_through() {
    let all = BUTTON_NAMES.iter().fold(0u16, |m, &(_, bit)| m | bit);
    assert_eq!(android_state(all, [0; 4], [0; 2]).buttons, all);
    let s = android_state(0, [100, 12000, -100, -12000], [0; 2]);
    assert_eq!((s.left, s.right), ([100, 12000], [-100, -12000]));
}

#[test]
fn full_triggers_convert_to_exactly_one() {
    let out = xbox::convert(&android_state(0, [0; 4], [255, 255]), 0);
    assert_eq!((out[10], out[11]), (1.0, 1.0));
    let out = xbox::convert(&android_state(0, [0; 4], [0, 0]), 0);
    assert_eq!((out[10], out[11]), (0.0, 0.0));
}

#[test]
fn slots_are_assigned_in_connect_order_and_reused() {
    let shared = Shared::default();
    assert_eq!(shared.poll(0).err(), Some(DeviceError::Disconnected));
    for id in [40, 41, 42] {
        pad(&shared, id);
    }
    pad(&shared, 41); // idempotent
    shared.disconnected(41);
    assert!(shared.poll(1).is_err() && shared.poll(2).is_ok());
    pad(&shared, 99); // first free slot
    let p = shared.poll(1).unwrap();
    assert_eq!(p.subtype, 1);
    assert_eq!(p.kind.unwrap().driver, "android#99");
    pad(&shared, 100);
    pad(&shared, 101); // slots 0..=3 full: ignored
    assert!(shared.poll(3).is_ok());
    shared.disconnected(7); // unknown: no-op
}

#[test]
fn packet_number_advances_only_on_change_and_unknown_ids_are_ignored() {
    let shared = Shared::default();
    shared.state(5, st(1)); // not connected
    pad(&shared, 5);
    assert_eq!(shared.poll(0).unwrap().number, 0);
    shared.state(5, st(0x1000));
    shared.state(5, st(0x1000));
    let p = shared.poll(0).unwrap();
    assert_eq!((p.number, p.state.buttons), (1, 0x1000));
    shared.state(5, st(0));
    assert_eq!(shared.poll(0).unwrap().number, 2);
    shared.disconnected(5);
    shared.state(5, st(1));
    assert!(shared.poll(0).is_err());
}
