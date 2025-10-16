use crate::key_mapping::ActionMapping;
use xcb::x::ModMask;
use xkbcommon::xkb;

pub static ACTION_MAPPINGS: &[ActionMapping] = &[
    ActionMapping {
        key: xkb::Keysym::Return,
        modifiers: &[ModMask::SHIFT],
        action: "st",
    },
    ActionMapping {
        key: xkb::Keysym::d,
        modifiers: &[ModMask::SHIFT],
        action: "xclock",
    },
];
