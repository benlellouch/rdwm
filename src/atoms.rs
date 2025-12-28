use log::error;
use xcb::x;
use xcb::Connection;

use crate::ewmh::EwmhHint;

pub struct Atoms {
    pub net_number_of_desktops: x::Atom,
    pub net_current_desktop: x::Atom,
    pub net_supported: x::Atom,
    pub net_supporting_wm_check: x::Atom,
    pub net_wm_window_type: x::Atom,
    pub net_wm_window_type_dock: x::Atom,
}

impl Atoms {
    pub fn initialize(conn: &Connection) -> Self {
        let net_number_of_desktops =
            Self::intern_atom(conn, EwmhHint::NetNumberOfDesktops.as_str());
        let net_current_desktop = Self::intern_atom(conn, EwmhHint::NetCurrentDesktop.as_str());
        let net_supported = Self::intern_atom(conn, EwmhHint::NetSupported.as_str());
        let net_supporting_wm_check =
            Self::intern_atom(conn, EwmhHint::NetSupportingWmCheck.as_str());
        let net_wm_window_type = Self::intern_atom(conn, "_NET_WM_WINDOW_TYPE");
        let net_wm_window_type_dock = Self::intern_atom(conn, "_NET_WM_WINDOW_TYPE_DOCK");

        Self {
            net_number_of_desktops,
            net_current_desktop,
            net_supported,
            net_supporting_wm_check,
            net_wm_window_type,
            net_wm_window_type_dock,
        }
    }

    pub fn intern_atom(conn: &Connection, name: &str) -> x::Atom {
        let cookie = conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: name.as_bytes(),
        });
        conn.wait_for_reply(cookie)
            .expect("If Interning Atom fails we don't want to start the WM")
            .atom()
    }

    pub fn set_window_property(
        conn: &Connection,
        window: x::Window,
        prop: x::Atom,
        values: &[u32],
    ) {
        conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: prop,
            r#type: x::ATOM_WINDOW,
            data: values,
        });
    }

    pub fn set_atom(conn: &Connection, root: x::Window, prop: x::Atom, values: &[u32]) {
        if let Err(e) = conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: root,
            property: prop,
            r#type: x::ATOM_ATOM,
            data: values,
        }) {
            error!("Failed to set Atom: {e:?}");
        }
    }

    pub fn set_cardinal32(conn: &Connection, root: x::Window, prop: x::Atom, values: &[u32]) {
        if let Err(e) = conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: root,
            property: prop,
            r#type: x::ATOM_CARDINAL,
            data: values,
        }) {
            error!("Failed to set Cardinal: {e:?}");
        }
    }
}
