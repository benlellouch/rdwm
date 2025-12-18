use crate::key_mapping::{ActionEvent, ActionMapping};
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
    ActionMapping {
        key: xkb::Keysym::j,
        modifiers: &[ModMask::SHIFT],
        action: ActionEvent::FocusPrev,
    },
    ActionMapping {
        key: xkb::Keysym::k,
        modifiers: &[ModMask::SHIFT],
        action: ActionEvent::FocusNext,
    },
];
