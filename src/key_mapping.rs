use xcb::x::ModMask;
use xkbcommon::xkb::Keysym;
pub struct ActionMapping {
    pub key: Keysym,
    pub modifiers: &'static [ModMask],
    pub action: &'static str,
}
