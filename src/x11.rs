use crate::{atoms::Atoms, effect::Effect};
use log::error;
use xcb::{
    Connection, ProtocolError, VoidCookieChecked, Xid,
    x::{self, EventMask, Window},
};

pub struct X11 {
    conn: Connection,
    root: Window,
    atoms: Atoms,
    wm_check_window: Window,
}

impl X11 {
    pub fn new(conn: Connection, root: Window, atoms: Atoms, wm_check_window: Window) -> Self {
        Self {
            conn,
            root,
            atoms,
            wm_check_window,
        }
    }

    pub const fn root(&self) -> Window {
        self.root
    }

    pub const fn wm_check_window(&self) -> Window {
        self.wm_check_window
    }

    pub const fn atoms(&self) -> &Atoms {
        &self.atoms
    }

    pub fn wait_for_event(&self) -> xcb::Result<xcb::Event> {
        self.conn.wait_for_event()
    }

    pub fn apply_effects_unchecked(&self, effects: &[Effect]) {
        for effect in effects {
            self.send_effect_unchecked(effect);
        }

        if let Err(e) = self.flush() {
            error!("Failed to flush X connection: {e:?}");
        }
    }

    pub fn apply_effects_checked(&self, effects: &[Effect]) {
        let mut pending_checks: Vec<(VoidCookieChecked, String)> = Vec::new();

        for effect in effects {
            let effect_dbg = format!("{effect:?}");
            for cookie in self.send_effect_checked(effect) {
                pending_checks.push((cookie, effect_dbg.clone()));
            }
        }

        if let Err(e) = self.flush() {
            error!("Failed to flush X connection: {e:?}");
        }

        for (cookie, effect_dbg) in pending_checks {
            if let Err(e) = self.check_cookie(cookie) {
                error!("X error applying {effect_dbg}: {e:?}");
            }
        }
    }

    pub fn send_effect_unchecked(&self, effect: &Effect) {
        match effect {
            Effect::Map(window) => self.map_window_unchecked(*window),
            Effect::Unmap(window) => self.unmap_window_unchecked(*window),
            Effect::Focus(window) => self.focus_window_unchecked(*window),
            Effect::Raise(window) => self.raise_window_unchecked(*window),
            Effect::Configure {
                window,
                x,
                y,
                w,
                h,
                border,
            } => self.configure_window_unchecked(*window, *x, *y, *w, *h, *border),
            Effect::ConfigurePositionSize { window, x, y, w, h } => {
                self.configure_window_position_size_unchecked(*window, *x, *y, *w, *h)
            }
            Effect::SetBorder {
                window,
                pixel,
                width,
            } => self.set_border_unchecked(*window, *pixel, *width),
            Effect::SetCardinal32 {
                window,
                atom,
                value,
            } => self.set_cardinal32_unchecked(*window, *atom, *value),
            Effect::SetCardinal32List {
                window,
                atom,
                values,
            } => self.set_cardinal32_list_unchecked(*window, *atom, values),
            Effect::SetAtomList {
                window,
                atom,
                values,
            } => self.set_atom_list_unchecked(*window, *atom, values),
            Effect::SetUtf8String {
                window,
                atom,
                value,
            } => self.set_utf8_string_unchecked(*window, *atom, value),
            Effect::SetWindowProperty {
                window,
                atom,
                values,
            } => self.set_window_property_unchecked(*window, *atom, values),
            Effect::KillClient(window) => self.kill_client_unchecked(*window),
            Effect::SendWmDelete(window) => self.send_wm_delete_unchecked(*window),
            Effect::GrabKey {
                keycode,
                modifiers,
                grab_window,
            } => self.grab_key_unchecked(*keycode, *modifiers, *grab_window),
        }
    }

    pub fn send_effect_checked(&self, effect: &Effect) -> Vec<VoidCookieChecked> {
        match effect {
            Effect::Map(window) => self.map_window_checked(*window),
            Effect::Unmap(window) => self.unmap_window_checked(*window),
            Effect::Focus(window) => self.focus_window_checked(*window),
            Effect::Raise(window) => self.raise_window_checked(*window),
            Effect::Configure {
                window,
                x,
                y,
                w,
                h,
                border,
            } => self.configure_window_checked_effect(*window, *x, *y, *w, *h, *border),
            Effect::ConfigurePositionSize { window, x, y, w, h } => {
                self.configure_window_position_size_checked(*window, *x, *y, *w, *h)
            }
            Effect::SetBorder {
                window,
                pixel,
                width,
            } => self.set_border_checked(*window, *pixel, *width),
            Effect::SetCardinal32 {
                window,
                atom,
                value,
            } => self.set_cardinal32_checked(*window, *atom, *value),
            Effect::SetCardinal32List {
                window,
                atom,
                values,
            } => self.set_cardinal32_list_checked(*window, *atom, values),
            Effect::SetAtomList {
                window,
                atom,
                values,
            } => self.set_atom_list_checked(*window, *atom, values),
            Effect::SetUtf8String {
                window,
                atom,
                value,
            } => self.set_utf8_string_checked(*window, *atom, value),
            Effect::SetWindowProperty {
                window,
                atom,
                values,
            } => self.set_window_property_checked(*window, *atom, values),
            Effect::KillClient(window) => self.kill_client_checked(*window),
            Effect::SendWmDelete(window) => self.send_wm_delete_checked(*window),
            Effect::GrabKey {
                keycode,
                modifiers,
                grab_window,
            } => self.grab_key_checked(*keycode, *modifiers, *grab_window),
        }
    }

    fn map_window_unchecked(&self, window: Window) {
        self.conn.send_request(&x::MapWindow { window });
    }

    fn unmap_window_unchecked(&self, window: Window) {
        self.conn.send_request(&x::UnmapWindow { window });
    }

    fn focus_window_unchecked(&self, window: Window) {
        self.conn.send_request(&x::SetInputFocus {
            revert_to: x::InputFocus::PointerRoot,
            focus: window,
            time: x::CURRENT_TIME,
        });
    }

    fn raise_window_unchecked(&self, window: Window) {
        let config_values = [x::ConfigWindow::StackMode(x::StackMode::Above)];
        self.conn.send_request(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        });
    }

    fn configure_window_unchecked(
        &self,
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        border: u32,
    ) {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(w),
            x::ConfigWindow::Height(h),
            x::ConfigWindow::BorderWidth(border),
        ];
        self.conn.send_request(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        });
    }

    fn configure_window_position_size_unchecked(
        &self,
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(w),
            x::ConfigWindow::Height(h),
        ];
        self.conn.send_request(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        });
    }

    fn set_border_unchecked(&self, window: Window, pixel: u32, width: u32) {
        self.conn.send_request(&x::ChangeWindowAttributes {
            window,
            value_list: &[x::Cw::BorderPixel(pixel)],
        });
        self.conn.send_request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::BorderWidth(width)],
        });
    }

    fn set_cardinal32_unchecked(&self, window: Window, atom: x::Atom, value: u32) {
        self.conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_CARDINAL,
            data: &[value],
        });
    }

    fn set_cardinal32_list_unchecked(&self, window: Window, atom: x::Atom, values: &[u32]) {
        self.conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_CARDINAL,
            data: values,
        });
    }

    fn set_atom_list_unchecked(&self, window: Window, atom: x::Atom, values: &[u32]) {
        self.conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_ATOM,
            data: values,
        });
    }

    fn set_window_property_unchecked(&self, window: Window, atom: x::Atom, values: &[u32]) {
        self.conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_WINDOW,
            data: values,
        });
    }

    fn set_utf8_string_unchecked(&self, window: Window, atom: x::Atom, value: &str) {
        self.conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: self.atoms.utf8_string,
            data: value.as_bytes(),
        });
    }

    fn kill_client_unchecked(&self, window: Window) {
        self.conn.send_request(&x::KillClient {
            resource: window.resource_id(),
        });
    }

    fn send_wm_delete_unchecked(&self, window: Window) {
        let ev = self.wm_delete_client_message(window);
        self.conn.send_request(&x::SendEvent {
            propagate: false,
            destination: x::SendEventDest::Window(window),
            event_mask: x::EventMask::NO_EVENT,
            event: &ev,
        });
    }

    fn grab_key_unchecked(&self, keycode: u8, modifiers: x::ModMask, grab_window: Window) {
        self.conn.send_request(&x::GrabKey {
            owner_events: false,
            grab_window,
            modifiers,
            key: keycode,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        });
    }

    fn map_window_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::MapWindow { window })]
    }

    fn unmap_window_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::UnmapWindow { window })]
    }

    fn focus_window_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::SetInputFocus {
            revert_to: x::InputFocus::PointerRoot,
            focus: window,
            time: x::CURRENT_TIME,
        })]
    }

    fn raise_window_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        let config_values = [x::ConfigWindow::StackMode(x::StackMode::Above)];
        vec![self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        })]
    }

    fn configure_window_checked_effect(
        &self,
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        border: u32,
    ) -> Vec<VoidCookieChecked> {
        vec![self.configure_window_checked(window, x, y, w, h, border)]
    }

    fn configure_window_position_size_checked(
        &self,
        window: Window,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> Vec<VoidCookieChecked> {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(w),
            x::ConfigWindow::Height(h),
        ];
        vec![self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        })]
    }

    fn set_border_checked(&self, window: Window, pixel: u32, width: u32) -> Vec<VoidCookieChecked> {
        let a = self.conn.send_request_checked(&x::ChangeWindowAttributes {
            window,
            value_list: &[x::Cw::BorderPixel(pixel)],
        });
        let b = self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::BorderWidth(width)],
        });
        vec![a, b]
    }

    fn set_cardinal32_checked(
        &self,
        window: Window,
        atom: x::Atom,
        value: u32,
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_CARDINAL,
            data: &[value],
        })]
    }

    fn set_cardinal32_list_checked(
        &self,
        window: Window,
        atom: x::Atom,
        values: &[u32],
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_CARDINAL,
            data: values,
        })]
    }

    fn set_atom_list_checked(
        &self,
        window: Window,
        atom: x::Atom,
        values: &[u32],
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_ATOM,
            data: values,
        })]
    }

    fn set_window_property_checked(
        &self,
        window: Window,
        atom: x::Atom,
        values: &[u32],
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: x::ATOM_WINDOW,
            data: values,
        })]
    }

    fn set_utf8_string_checked(
        &self,
        window: Window,
        atom: x::Atom,
        value: &str,
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atom,
            r#type: self.atoms.utf8_string,
            data: value.as_bytes(),
        })]
    }

    fn kill_client_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::KillClient {
            resource: window.resource_id(),
        })]
    }

    fn send_wm_delete_checked(&self, window: Window) -> Vec<VoidCookieChecked> {
        let ev = self.wm_delete_client_message(window);
        vec![self.conn.send_request_checked(&x::SendEvent {
            propagate: false,
            destination: x::SendEventDest::Window(window),
            event_mask: x::EventMask::NO_EVENT,
            event: &ev,
        })]
    }

    fn grab_key_checked(
        &self,
        keycode: u8,
        modifiers: x::ModMask,
        grab_window: Window,
    ) -> Vec<VoidCookieChecked> {
        vec![self.conn.send_request_checked(&x::GrabKey {
            owner_events: false,
            grab_window,
            modifiers,
            key: keycode,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        })]
    }

    fn wm_delete_client_message(&self, window: Window) -> x::ClientMessageEvent {
        x::ClientMessageEvent::new(
            window,
            self.atoms.wm_protocols,
            x::ClientMessageData::Data32([
                self.atoms.wm_delete_window.resource_id(),
                x::CURRENT_TIME,
                0,
                0,
                0,
            ]),
        )
    }

    pub fn flush(&self) -> xcb::Result<()> {
        self.conn.flush().map_err(Into::into)
    }

    pub fn check_cookie(&self, cookie: VoidCookieChecked) -> xcb::Result<()> {
        self.conn.check_request(cookie).map_err(Into::into)
    }

    pub fn set_root_event_mask(&self) -> Result<(), ProtocolError> {
        let values = [x::Cw::EventMask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::KEY_PRESS,
        )];
        self.conn
            .send_and_check_request(&x::ChangeWindowAttributes {
                window: self.root,
                value_list: &values,
            })
    }

    pub fn get_root_window_children(&self) -> Result<Vec<Window>, xcb::Error> {
        let cookie = self.conn.send_request(&x::QueryTree { window: self.root });
        let reply = self.conn.wait_for_reply(cookie)?;
        Ok(reply.children().to_vec())
    }

    pub fn is_dock_window(&self, window: Window) -> bool {
        let cookie = self.conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: self.atoms.wm_window_type,
            r#type: x::ATOM_ATOM,
            long_offset: 0,
            long_length: 32,
        });

        if let Ok(reply) = self.conn.wait_for_reply(cookie) {
            let atoms_vec: &[x::Atom] = reply.value();
            atoms_vec
                .iter()
                .any(|a| a.resource_id() == self.atoms.wm_window_type_dock.resource_id())
        } else {
            false
        }
    }

    pub fn supports_wm_delete(&self, window: Window) -> Result<bool, xcb::Error> {
        let cookie = self.conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: self.atoms.wm_protocols,
            r#type: x::ATOM_ATOM,
            long_offset: 0,
            long_length: 1024,
        });

        let reply = self.conn.wait_for_reply(cookie)?;
        let atoms_list: &[x::Atom] = reply.value();
        Ok(atoms_list.contains(&self.atoms.wm_delete_window))
    }

    pub fn get_cardinal32(&self, window: x::Window, prop: x::Atom) -> Option<u32> {
        let cookie = self.conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: prop,
            r#type: x::ATOM_CARDINAL,
            long_offset: 0,
            long_length: 1,
        });

        if let Ok(reply) = self.conn.wait_for_reply(cookie) {
            let value: &[u32] = reply.value();
            if !value.is_empty() {
                return value.first().cloned();
            }
        }
        error!("Failed to get Cardinal32 property for atom {prop:?} on {window:?}");
        None
    }

    fn configure_window_checked(
        &self,
        window: Window,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        border: u32,
    ) -> VoidCookieChecked {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(width),
            x::ConfigWindow::Height(height),
            x::ConfigWindow::BorderWidth(border),
        ];

        self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        })
    }
}
