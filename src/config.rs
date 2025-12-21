use crate::key_mapping::{ActionEvent, ActionMapping};
use xcb::x::ModMask;
use xkbcommon::xkb;

pub const NUM_WORKSPACES: usize = 10;
pub const DEFAULT_BORDER_WIDTH: u32 = 5;

pub static ACTION_MAPPINGS: &[ActionMapping] = &[
    ActionMapping {
        key: xkb::Keysym::Return,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Spawn("st"),
    },
    ActionMapping {
        key: xkb::Keysym::Return,
        modifiers: &[ModMask::N1, ModMask::SHIFT],
        action: ActionEvent::Spawn("google-chrome-stable"),
    },
    ActionMapping {
        key: xkb::Keysym::d,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Spawn("xclock"),
    },
    ActionMapping {
        key: xkb::Keysym::q,
        modifiers: &[ModMask::N1],
        action: ActionEvent::KillClient,
    },
    ActionMapping {
        key: xkb::Keysym::j,
        modifiers: &[ModMask::N1],
        action: ActionEvent::FocusPrev,
    },
    ActionMapping {
        key: xkb::Keysym::k,
        modifiers: &[ModMask::N1],
        action: ActionEvent::FocusNext,
    },
    ActionMapping {
        key: xkb::Keysym::exclam,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(0),
    },
    ActionMapping {
        key: xkb::Keysym::at,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(1),
    },
];
