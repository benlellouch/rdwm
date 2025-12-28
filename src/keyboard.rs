use log::{info, warn};
use std::collections::HashMap;
use xcb::x::{self, ModMask};
use xcb::Connection;

use crate::config::ACTION_MAPPINGS;
use crate::key_mapping::ActionEvent;

pub fn fetch_keyboard_mapping(conn: &Connection) -> (Vec<u32>, usize) {
    if let Ok(keyboard_mapping) = conn.wait_for_reply(conn.send_request(&x::GetKeyboardMapping {
        first_keycode: conn.get_setup().min_keycode(),
        count: conn.get_setup().max_keycode() - conn.get_setup().min_keycode() + 1,
    })) {
        let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode() as usize;
        let keysyms = keyboard_mapping.keysyms().to_vec();
        (keysyms, keysyms_per_keycode)
    } else {
        warn!("Failed to get keyboard mapping, using empty keysyms");
        (vec![], 0)
    }
}

pub fn populate_key_bindings(
    conn: &Connection,
    keysyms: &[u32],
    keysyms_per_keycode: usize,
) -> HashMap<(u8, ModMask), ActionEvent> {
    let mut key_bindings = HashMap::new();

    for mapping in ACTION_MAPPINGS {
        let modifiers = mapping
            .modifiers
            .iter()
            .copied()
            .reduce(|acc, modkey| acc | modkey)
            .unwrap_or(xcb::x::ModMask::empty());

        for (i, chunk) in keysyms.chunks(keysyms_per_keycode).enumerate() {
            if chunk.contains(&mapping.key.raw()) {
                let keycode = conn.get_setup().min_keycode() + i as u8;
                key_bindings.insert((keycode, modifiers), mapping.action);
                info!(
                    "Mapped key {:?} (keycode: {}) with modifiers {:?} to action: {:?}",
                    mapping.key, keycode, modifiers, mapping.action
                );
                break;
            }
        }
    }

    key_bindings
}

pub fn set_keygrabs(
    conn: &Connection,
    key_bindings: &HashMap<(u8, ModMask), ActionEvent>,
    root: x::Window,
) {
    for &(keycode, modifiers) in key_bindings.keys() {
        match conn.send_and_check_request(&x::GrabKey {
            owner_events: false,
            grab_window: root,
            modifiers,
            key: keycode,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        }) {
            Ok(()) => info!(
                "Successfully grabbed key: keycode {keycode} with modifiers {modifiers:?}"
            ),
            Err(e) => warn!("Failed to grab key {keycode}: {e:?}"),
        }
    }
}
