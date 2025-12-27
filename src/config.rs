use crate::key_mapping::{ActionEvent, ActionMapping};
use std::option_env;
use xcb::x::ModMask;
use xkbcommon::xkb;

pub const NUM_WORKSPACES: usize = 10;
pub const DEFAULT_BORDER_WIDTH: u32 = 3;
pub const DEFAULT_WINDOW_GAP: u32 = 0;

const TESTING: Option<&str> = option_env!("WM_TESTING");
const MOD: ModMask = if TESTING.is_none() {
    ModMask::N4
} else {
    ModMask::N1
};

/// Usage: binding!(key, [modifiers], action)
macro_rules! binding {
    ($key:expr, [$($mod:expr),*], $action:expr) => {
        ActionMapping {
            key: $key,
            modifiers: &[$($mod),*],
            action: $action,
        }
    };
}

#[rustfmt::skip] 
pub static ACTION_MAPPINGS: &[ActionMapping] = &[
    // ==================== SPAWN BINDINGS ====================
    binding!(xkb::Keysym::Return, [MOD], ActionEvent::Spawn("st")),
    binding!(xkb::Keysym::Return, [MOD, ModMask::SHIFT], ActionEvent::Spawn("google-chrome-stable")),
    binding!(xkb::Keysym::space, [MOD], ActionEvent::Spawn("rofi -show drun")),

    // ==================== MULTIMEDIA BINDINGS ====================
    binding!(xkb::Keysym::XF86_ScrollUp, [], ActionEvent::Spawn("amixer set Master -q 5%+")),
    binding!(xkb::Keysym::XF86_ScrollDown, [], ActionEvent::Spawn("amixer set Master -q 5%-")),
    binding!(xkb::Keysym::XF86_AudioRaiseVolume, [], ActionEvent::Spawn("amixer set Master -q 5%+")),
    binding!(xkb::Keysym::XF86_AudioLowerVolume, [], ActionEvent::Spawn("amixer set Master -q 5%-")),

    // ==================== WINDOW MANAGEMENT ====================
    binding!(xkb::Keysym::q, [MOD], ActionEvent::Kill),
    binding!(xkb::Keysym::Left, [MOD], ActionEvent::PrevWindow),
    binding!(xkb::Keysym::Right, [MOD], ActionEvent::NextWindow),
    binding!(xkb::Keysym::Left, [MOD, ModMask::SHIFT], ActionEvent::SwapLeft),
    binding!(xkb::Keysym::Right, [MOD, ModMask::SHIFT], ActionEvent::SwapRight),

    // ==================== WINDOW SIZING ====================
    binding!(xkb::Keysym::equal, [MOD], ActionEvent::IncreaseWindowWeight(1)),
    binding!(xkb::Keysym::minus, [MOD], ActionEvent::DecreaseWindowWeight(1)),
    binding!(xkb::Keysym::equal, [MOD, ModMask::SHIFT], ActionEvent::IncreaseWindowGap(1)),
    binding!(xkb::Keysym::minus, [MOD, ModMask::SHIFT], ActionEvent::DecreaseWindowGap(1)),

    // ==================== WORKSPACE NAVIGATION (MOD + 1-9, 0) ====================
    binding!(xkb::Keysym::_1, [MOD], ActionEvent::GoToWorkspace(0)),
    binding!(xkb::Keysym::_2, [MOD], ActionEvent::GoToWorkspace(1)),
    binding!(xkb::Keysym::_3, [MOD], ActionEvent::GoToWorkspace(2)),
    binding!(xkb::Keysym::_4, [MOD], ActionEvent::GoToWorkspace(3)),
    binding!(xkb::Keysym::_5, [MOD], ActionEvent::GoToWorkspace(4)),
    binding!(xkb::Keysym::_6, [MOD], ActionEvent::GoToWorkspace(5)),
    binding!(xkb::Keysym::_7, [MOD], ActionEvent::GoToWorkspace(6)),
    binding!(xkb::Keysym::_8, [MOD], ActionEvent::GoToWorkspace(7)),
    binding!(xkb::Keysym::_9, [MOD], ActionEvent::GoToWorkspace(8)),
    binding!(xkb::Keysym::_0, [MOD], ActionEvent::GoToWorkspace(9)),

    // ==================== WORKSPACE SEND (MOD + SHIFT + 1-9, 0) ====================
    binding!(xkb::Keysym::_1, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(0)),
    binding!(xkb::Keysym::_2, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(1)),
    binding!(xkb::Keysym::_3, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(2)),
    binding!(xkb::Keysym::_4, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(3)),
    binding!(xkb::Keysym::_5, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(4)),
    binding!(xkb::Keysym::_6, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(5)),
    binding!(xkb::Keysym::_7, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(6)),
    binding!(xkb::Keysym::_8, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(7)),
    binding!(xkb::Keysym::_9, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(8)),
    binding!(xkb::Keysym::_0, [MOD, ModMask::SHIFT], ActionEvent::SendToWorkspace(9)),
];
