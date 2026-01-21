use xcb::x::{self, ModMask, Window};

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Map(Window),
    Unmap(Window),
    Configure {
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        border: u32,
    },
    ConfigurePositionSize {
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },
    Focus(Window),
    Raise(Window),
    SetBorder {
        window: Window,
        pixel: u32,
        width: u32,
    },
    SetCardinal32 {
        window: Window,
        atom: x::Atom,
        value: u32,
    },
    SetCardinal32List {
        window: Window,
        atom: x::Atom,
        values: Vec<u32>,
    },
    SetAtomList {
        window: Window,
        atom: x::Atom,
        values: Vec<u32>,
    },
    SetUtf8String {
        window: Window,
        atom: x::Atom,
        value: String,
    },
    SetWindowProperty {
        window: Window,
        atom: x::Atom,
        values: Vec<u32>,
    },
    KillClient(Window),
    SendWmDelete(Window),
    GrabKey {
        keycode: u8,
        modifiers: ModMask,
        grab_window: Window,
    },
}
