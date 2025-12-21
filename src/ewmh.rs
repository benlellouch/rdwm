pub enum EwmhHint {
    NetNumberOfDesktops,
    NetCurrentDesktop,
    NetSupported,
    NetSupportingWmCheck,
}

impl EwmhHint {
    pub fn as_str(&self) -> &'static str {
        match self {
            EwmhHint::NetNumberOfDesktops => "_NET_NUMBER_OF_DESKTOPS",
            EwmhHint::NetCurrentDesktop => "_NET_CURRENT_DESKTOP",
            EwmhHint::NetSupported => "_NET_SUPPORTED",
            EwmhHint::NetSupportingWmCheck => "_NET_SUPPORTING_WM_CHECK",
        }
    }
}
