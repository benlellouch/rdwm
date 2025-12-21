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
        key: xkb::Keysym::_1,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(0),
    },
    ActionMapping {
        key: xkb::Keysym::_2,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(1),
    },
        ActionMapping {
        key: xkb::Keysym::_3,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(2),
    },
    ActionMapping {
        key: xkb::Keysym::_4,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(3),
    },
    ActionMapping {
        key: xkb::Keysym::_5,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(4),
    },
    ActionMapping {
        key: xkb::Keysym::_6,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(5),
    },
    ActionMapping {
        key: xkb::Keysym::_7,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(6),
    },
    ActionMapping {
        key: xkb::Keysym::_8,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(7),
    },
    ActionMapping {
        key: xkb::Keysym::_9,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(8),
    },
        ActionMapping {
        key: xkb::Keysym::_0,
        modifiers: &[ModMask::N1],
        action: ActionEvent::Workspace(9),
    },
];
