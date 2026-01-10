use log::error;
use xcb::x;
use xcb::Connection;

pub struct Atoms {
    pub number_of_desktops: x::Atom,
    pub current_desktop: x::Atom,
    pub supported: x::Atom,
    pub supporting_wm_check: x::Atom,
    pub wm_window_type: x::Atom,
    pub wm_window_type_dock: x::Atom,
    pub wm_protocols: x::Atom,
    pub wm_delete_window: x::Atom,
    pub wm_desktop: x::Atom,
}

impl Atoms {
    pub fn initialize(conn: &Connection) -> Self {
        let number_of_desktops = Self::intern_atom(conn, "_NET_NUMBER_OF_DESKTOPS");
        let current_desktop = Self::intern_atom(conn, "_NET_CURRENT_DESKTOP");
        let supported = Self::intern_atom(conn, "_NET_SUPPORTED");
        let supporting_wm_check = Self::intern_atom(conn, "_NET_SUPPORTING_WM_CHECK");
        let wm_window_type = Self::intern_atom(conn, "_NET_WM_WINDOW_TYPE");
        let wm_window_type_dock = Self::intern_atom(conn, "_NET_WM_WINDOW_TYPE_DOCK");
        let wm_protocols = Self::intern_atom(conn, "WM_PROTOCOLS");
        let wm_delete_window = Self::intern_atom(conn, "WM_DELETE_WINDOW");
        let wm_desktop = Self::intern_atom(conn, "_NET_WM_DESKTOP");

        Self {
            number_of_desktops,
            current_desktop,
            supported,
            supporting_wm_check,
            wm_window_type,
            wm_window_type_dock,
            wm_protocols,
            wm_delete_window,
            wm_desktop,
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

    pub fn get_cardinal32(conn: &Connection, window: x::Window, prop: x::Atom) -> Option<u32> {
        let cookie = conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: prop,
            r#type: x::ATOM_CARDINAL,
            long_offset: 0,
            long_length: 1,
        });

        if let Ok(reply) = conn.wait_for_reply(cookie) {
            let value: &[u32] = reply.value();
            if !value.is_empty() {
                return value.first().cloned();
            }
        }
        error!("Failed to get Cardinal32 property for atom {prop:?} on {window:?}");
        None
    }
}
