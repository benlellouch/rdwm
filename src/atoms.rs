use xcb::atoms_struct;

atoms_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct Atoms {
        // ===== EWMH root properties =====
        pub number_of_desktops => b"_NET_NUMBER_OF_DESKTOPS" only_if_exists = false,
        pub current_desktop => b"_NET_CURRENT_DESKTOP" only_if_exists = false,
        pub desktop_names => b"_NET_DESKTOP_NAMES" only_if_exists = false,
        pub desktop_viewport => b"_NET_DESKTOP_VIEWPORT" only_if_exists = false,
        pub desktop_geometry => b"_NET_DESKTOP_GEOMETRY" only_if_exists = false,
        pub workarea => b"_NET_WORKAREA" only_if_exists = false,
        pub showing_desktop => b"_NET_SHOWING_DESKTOP" only_if_exists = false,
        pub active_window => b"_NET_ACTIVE_WINDOW" only_if_exists = false,
        pub client_list => b"_NET_CLIENT_LIST" only_if_exists = false,
        pub client_list_stacking => b"_NET_CLIENT_LIST_STACKING" only_if_exists = false,

        pub supported => b"_NET_SUPPORTED" only_if_exists = false,
        pub supporting_wm_check => b"_NET_SUPPORTING_WM_CHECK" only_if_exists = false,

        // ===== EWMH WM identity =====
        pub wm_name => b"_NET_WM_NAME" only_if_exists = false,
        pub wm_pid => b"_NET_WM_PID" only_if_exists = false,
        pub utf8_string => b"UTF8_STRING" only_if_exists = false,

        // ===== EWMH per-window properties =====
        pub wm_window_type => b"_NET_WM_WINDOW_TYPE" only_if_exists = false,
        pub wm_window_type_dock => b"_NET_WM_WINDOW_TYPE_DOCK" only_if_exists = false,
        pub wm_strut_partial => b"_NET_WM_STRUT_PARTIAL" only_if_exists = false,
        pub wm_state => b"_NET_WM_STATE" only_if_exists = false,
        pub wm_state_fullscreen => b"_NET_WM_STATE_FULLSCREEN" only_if_exists = false,
        pub close_window => b"_NET_CLOSE_WINDOW" only_if_exists = false,
        pub wm_protocols => b"WM_PROTOCOLS" only_if_exists = false,
        pub wm_delete_window => b"WM_DELETE_WINDOW" only_if_exists = false,
        pub wm_desktop => b"_NET_WM_DESKTOP" only_if_exists = false,
    }
}
