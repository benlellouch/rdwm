use crate::key_mapping::{ActionMapping, ActionEvent};
use xcb::x::ModMask;
use xkbcommon::xkb;

pub static ACTION_MAPPINGS: &[ActionMapping] = &[
    ActionMapping {
        key: xkb::Keysym::Return,
        modifiers: &[ModMask::SHIFT],
        action: ActionEvent::Spawn("st"),
    },
    ActionMapping {
        key: xkb::Keysym::d,
        modifiers: &[ModMask::SHIFT],
        action: ActionEvent::Spawn("xclock"),
    },
    ActionMapping {
        key: xkb::Keysym::q,
        modifiers: &[ModMask::SHIFT],
        action: ActionEvent::KillClient,
    },
];
