use xcb::x::ModMask;
use xkbcommon::xkb::Keysym;
pub struct ActionMapping {
    pub key: Keysym,
    pub modifiers: &'static [ModMask],
    pub action: ActionEvent,
}

#[derive(Debug, Copy, Clone)]
pub enum ActionEvent {
    Spawn(&'static str),
    Kill,
    NextWindow,
    PrevWindow,
    IncreaseWindowWeight(u32),
    DecreaseWindowWeight(u32),
    SwapLeft,
    SwapRight,
    GoToWorkspace(usize),
    SendToWorkspace(usize),
    IncreaseWindowGap(u32),
    DecreaseWindowGap(u32),
    ToggleFullscreen,
    CycleLayout,
}
