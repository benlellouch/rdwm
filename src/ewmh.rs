pub enum EwmhHint {
    NetNumberOfDesktops,
    NetCurrentDesktop,
    NetSupported,
    NetSupportingWmCheck,
}

impl EwmhHint {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NetNumberOfDesktops => "_NET_NUMBER_OF_DESKTOPS",
            Self::NetCurrentDesktop => "_NET_CURRENT_DESKTOP",
            Self::NetSupported => "_NET_SUPPORTED",
            Self::NetSupportingWmCheck => "_NET_SUPPORTING_WM_CHECK",
        }
    }
}
