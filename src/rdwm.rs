use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::process::Command;
use xcb::{
    x::{self, Cw, EventMask, ModMask, Window},
    Connection, ProtocolError, VoidCookieChecked, Xid,
};

use crate::config::{ACTION_MAPPINGS, DEFAULT_BORDER_WIDTH, NUM_WORKSPACES};
use crate::key_mapping::ActionEvent;
use crate::atoms::Atoms;
use crate::workspace::Workspace;

pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub focused_border_pixel: u32,
    pub normal_border_pixel: u32,
}



pub struct WindowManagerConfig {
    pub key_bindings: HashMap<(u8, ModMask), ActionEvent>,
    pub screen_config: ScreenConfig,
    pub atoms: Atoms,
    pub root_window: Window,
}

pub struct WindowManager {
    conn: Connection,
    workspaces: [Workspace; NUM_WORKSPACES],
    workspace: usize,
    key_bindings: HashMap<(u8, ModMask), ActionEvent>,
    screen_width: u32,
    screen_height: u32,
    focused_border_pixel: u32,
    normal_border_pixel: u32,
    border_width: u32,
    atoms: Atoms,
    root_window: Window,
    wm_check_window: Window,
}

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, _) = Connection::connect(None)?;
        info!("Connected to X.");

        // Initialize configuration before creating WindowManager
        let config = Self::initialize_config(&conn)?;

        // Create WM check window
        let wm_check_window = Self::create_wm_check_window(&conn, config.root_window);

        let wm = WindowManager {
            conn,
            workspaces: Default::default(),
            workspace: 0,
            key_bindings: config.key_bindings,
            screen_width: config.screen_config.width,
            screen_height: config.screen_config.height,
            focused_border_pixel: config.screen_config.focused_border_pixel,
            normal_border_pixel: config.screen_config.normal_border_pixel,
            border_width: DEFAULT_BORDER_WIDTH,
            atoms: config.atoms,
            root_window: config.root_window,
            wm_check_window,
        };

        // Get root window and set up substructure redirect
        wm.set_substructure_redirect()?;
        info!("Successfully set substructure redirect");

        // Set up key grabs
        wm.set_keygrabs();

        // Set up EWMH hints
        wm.publish_ewmh_hints();

        Ok(wm)
    }

    /*

    ▗▄ ▗▖▗▄▄▄▖▄   ▄     ▗▖ ▗▖▗▄▄▄▖▗▖   ▗▄▄▖ ▗▄▄▄▖▗▄▄▖  ▗▄▖
    ▐█ ▐▌▐▛▀▀▘█   █     ▐▌ ▐▌▐▛▀▀▘▐▌   ▐▛▀▜▖▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▐▛▌▐▌▐▌   ▜▖█▗▛     ▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▙
    ▐▌█▐▌▐███ ▐▌█▐▌     ▐███▌▐███ ▐▌   ▐██▛ ▐███ ▐███  ▜█▙
    ▐▌▐▟▌▐▌   ▐█▀█▌     ▐▌ ▐▌▐▌   ▐▌   ▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐▌ █▌▐▙▄▄▖▐█ █▌     ▐▌ ▐▌▐▙▄▄▖▐▙▄▄▖▐▌   ▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▘ ▀▘▝▀▀▀▘▝▀ ▀▘     ▝▘ ▝▘▝▀▀▀▘▝▀▀▀▘▝▘   ▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn initialize_config(
        conn: &Connection,
    ) -> Result<WindowManagerConfig, Box<dyn std::error::Error>> {
        let (keysyms, keysyms_per_keycode) = Self::fetch_keyboard_mapping(conn);
        let key_bindings = Self::populate_key_bindings(conn, &keysyms, keysyms_per_keycode);
        let screen_config = Self::setup_screen(conn);
        let atoms = Atoms::initialize(conn);
        let root_window = Self::get_root_window(conn);

        Ok(WindowManagerConfig {
            key_bindings,
            screen_config,
            atoms,
            root_window,
        })
    }

    fn fetch_keyboard_mapping(conn: &Connection) -> (Vec<u32>, usize) {
        if let Ok(keyboard_mapping) =
            conn.wait_for_reply(conn.send_request(&x::GetKeyboardMapping {
                first_keycode: conn.get_setup().min_keycode(),
                count: conn.get_setup().max_keycode() - conn.get_setup().min_keycode() + 1,
            }))
        {
            let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode() as usize;
            let keysyms = keyboard_mapping.keysyms().to_vec();
            (keysyms, keysyms_per_keycode)
        } else {
            warn!("Failed to get keyboard mapping, using empty keysyms");
            (vec![], 0)
        }
    }

    fn populate_key_bindings(
        conn: &Connection,
        keysyms: &[u32],
        keysyms_per_keycode: usize,
    ) -> HashMap<(u8, ModMask), ActionEvent> {
        let mut key_bindings = HashMap::new();

        for mapping in ACTION_MAPPINGS {
            let modifiers = mapping
                .modifiers
                .iter()
                .copied()
                .reduce(|acc, modkey| acc | modkey)
                .unwrap_or(xcb::x::ModMask::empty());

            for (i, chunk) in keysyms.chunks(keysyms_per_keycode).enumerate() {
                if chunk.contains(&mapping.key.raw()) {
                    let keycode = conn.get_setup().min_keycode() + i as u8;
                    key_bindings.insert((keycode, modifiers), mapping.action);
                    info!(
                        "Mapped key {:?} (keycode: {}) with modifiers {:?} to action: {:?}",
                        mapping.key, keycode, modifiers, mapping.action
                    );
                    break;
                }
            }
        }

        key_bindings
    }

    fn setup_screen(conn: &Connection) -> ScreenConfig {
        let root = conn.get_setup().roots().next().expect("Cannot find root");
        ScreenConfig {
            width: root.width_in_pixels() as u32,
            height: root.height_in_pixels() as u32,
            focused_border_pixel: root.white_pixel(),
            normal_border_pixel: root.black_pixel(),
        }
    }

    fn get_root_window(conn: &Connection) -> Window {
        conn.get_setup()
            .roots()
            .next()
            .expect("Cannot find root")
            .root()
    }

    fn create_wm_check_window(conn: &Connection, root: Window) -> Window {
        // Create a check window for _NET_SUPPORTING_WM_CHECK
        // This window is used by clients to verify the WM is EWMH compliant
        let win = conn.generate_id();
        let values = [
            x::Cw::OverrideRedirect(true),
        ];
        conn.send_request(&x::CreateWindow {
            depth: 0,
            wid: win,
            parent: root,
            x: -1,
            y: -1,
            width: 1,
            height: 1,
            border_width: 0,
            class: x::WindowClass::InputOnly,
            visual: 0,
            value_list: &values,
        });
        win
    }

    fn set_substructure_redirect(&self) -> Result<(), ProtocolError> {
        let values = [Cw::EventMask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::KEY_PRESS,
        )];
        self.conn
            .send_and_check_request(&x::ChangeWindowAttributes {
                window: self.root_window(),
                value_list: &values,
            })
    }

    fn set_keygrabs(&self) {
        for &(keycode, modifiers) in self.key_bindings.keys() {
            match self.conn.send_and_check_request(&x::GrabKey {
                owner_events: false,
                grab_window: self.root_window(),
                modifiers,
                key: keycode,
                pointer_mode: x::GrabMode::Async,
                keyboard_mode: x::GrabMode::Async,
            }) {
                Ok(_) => info!(
                    "Successfully grabbed key: keycode {} with modifiers {:?}",
                    keycode, modifiers
                ),
                Err(e) => warn!("Failed to grab key {}: {:?}", keycode, e),
            }
        }
    }



    /*

    ▗▄▄▄▖▄   ▄▗▄ ▄▖▗▖ ▗▖
    ▐▛▀▀▘█   █▐█ █▌▐▌ ▐▌
    ▐▌   ▜▖█▗▛▐███▌▐▌ ▐▌
    ▐███ ▐▌█▐▌▐▌█▐▌▐███▌
    ▐▌   ▐█▀█▌▐▌▀▐▌▐▌ ▐▌
    ▐▙▄▄▖▐█ █▌▐▌ ▐▌▐▌ ▐▌
    ▝▀▀▀▘▝▀ ▀▘▝▘ ▝▘▝▘ ▝▘

    */

    

    fn publish_ewmh_hints(&self) {
        // Publish _NET_SUPPORTING_WM_CHECK on both root and check window
        // This points the root window to the check window
        Atoms::set_window_property(
            &self.conn,
            self.root_window(),
            self.atoms.net_supporting_wm_check,
            &[self.wm_check_window.resource_id()],
        );

        // The check window points to itself
        Atoms::set_window_property(
            &self.conn,
            self.wm_check_window,
            self.atoms.net_supporting_wm_check,
            &[self.wm_check_window.resource_id()],
        );

        // Publish _NET_SUPPORTING - list of supported atoms
        let supported_atoms = [
            self.atoms.net_supported,
            self.atoms.net_supporting_wm_check,
            self.atoms.net_number_of_desktops,
            self.atoms.net_current_desktop,
        ];

        Atoms::set_atom(
            &self.conn,
            self.root_window(),
            self.atoms.net_supported,
            &supported_atoms.iter().map(|a| a.resource_id()).collect::<Vec<_>>(),
        );

        // Publish desktop information
        Atoms::set_cardinal32(&self.conn, self.root_window(), self.atoms.net_number_of_desktops, &[NUM_WORKSPACES as u32]);
        Atoms::set_cardinal32(&self.conn, self.root_window(), self.atoms.net_current_desktop, &[0 as u32]);

        info!("Published EWMH hints successfully");
    }

    fn update_current_workspace(&self) {
        Atoms::set_cardinal32(&self.conn, self.root_window(), self.atoms.net_current_desktop, &[self.workspace as u32]);
    }

    

    /*

    ▗▖ ▗▖▗▄▄▄▖ ▄▄▄ ▗▖    ▗▄▖
    ▐▌ ▐▌▝▀█▀▘ ▀█▀ ▐▌   ▗▛▀▜
    ▐▌ ▐▌  █    █  ▐▌   ▐▙
    ▐▌ ▐▌  █    █  ▐▌    ▜█▙
    ▐▌ ▐▌  █    █  ▐▌      ▜▌
    ▝█▄█▘  █   ▄█▄ ▐▙▄▄▖▐▄▄▟▘
     ▝▀▘   ▀   ▀▀▀ ▝▀▀▀▘ ▀▀▘


    */

    fn root_window(&self) -> Window {
        self.root_window
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace {
        self.workspaces
            .get_mut(self.workspace)
            .expect("Workspace should never be out of bounds")
    }

    fn current_workspace(&self) -> &Workspace {
        self.workspaces
            .get(self.workspace)
            .expect("Workspace should never be out of bounds")
    }

    /*

    ▄   ▄ ▄▄▄ ▗▄ ▗▖▗▄▄   ▗▄▖ ▄   ▄     ▗▖ ▗▖▗▄▄▄▖▗▖   ▗▄▄▖ ▗▄▄▄▖▗▄▄▖  ▗▄▖
    █   █ ▀█▀ ▐█ ▐▌▐▛▀█  █▀█ █   █     ▐▌ ▐▌▐▛▀▀▘▐▌   ▐▛▀▜▖▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▜▖█▗▛  █  ▐▛▌▐▌▐▌ ▐▌▐▌ ▐▌▜▖█▗▛     ▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▙
    ▐▌█▐▌  █  ▐▌█▐▌▐▌ ▐▌▐▌ ▐▌▐▌█▐▌     ▐███▌▐███ ▐▌   ▐██▛ ▐███ ▐███  ▜█▙
    ▐█▀█▌  █  ▐▌▐▟▌▐▌ ▐▌▐▌ ▐▌▐█▀█▌     ▐▌ ▐▌▐▌   ▐▌   ▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐█ █▌ ▄█▄ ▐▌ █▌▐▙▄█  █▄█ ▐█ █▌     ▐▌ ▐▌▐▙▄▄▖▐▙▄▄▖▐▌   ▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▀ ▀▘ ▀▀▀ ▝▘ ▀▘▝▀▀   ▝▀▘ ▝▀ ▀▘     ▝▘ ▝▘▝▀▀▀▘▝▀▀▀▘▝▘   ▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn configure_window(
        &self,
        window: Window,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> VoidCookieChecked {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(width),
            x::ConfigWindow::Height(height),
            x::ConfigWindow::BorderWidth(self.border_width),
        ];

        self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        })
    }

    fn configure_windows(&self) {
        let window_count = self.current_workspace().num_of_windows() as u32;
        if window_count == 0 {
            debug!("No windows to configure");
            return;
        }

        let border_width = self.border_width as i32;
        let cell = (self.screen_width as i32) / (window_count as i32);
        let inner_w = (cell - 2 * border_width).max(1);
        let inner_h = ((self.screen_height as i32) - 2 * border_width).max(1);

        let config_cookies: Vec<_> = self
            .current_workspace()
            .iter_windows()
            .enumerate()
            .map(|(i, win)| {
                let x = i as i32 * cell;
                let y = 0;
                self.configure_window(*win, x, y, inner_w as u32, inner_h as u32)
            })
            .collect();

        config_cookies.into_iter().for_each(|cookie| {
            let _ = self.conn.check_request(cookie);
        });
    }

    fn set_focus(&mut self, idx: usize) {
        // Reset border on old focused window (if any)
        if let Some(old) = self.current_workspace().get_focused_window().copied() {
            self.set_window_border(old, self.normal_border_pixel, self.border_width);
            debug!("Reset border on old focused window");
        }

        self.current_workspace_mut().set_focus(idx);

        // Set border on window to be focused (if present)
        if let Some(new_focus_window) = self.current_workspace().get_focused_window().copied() {
            self.set_window_border(
                new_focus_window,
                self.focused_border_pixel,
                self.border_width,
            );
            let _ = self.conn.send_and_check_request(&x::SetInputFocus {
                revert_to: x::InputFocus::PointerRoot,
                focus: new_focus_window,
                time: 0,
            });
        }
    }

    fn set_window_border(&self, window: Window, pixel: u32, width: u32) {
        let _ = self
            .conn
            .send_and_check_request(&x::ChangeWindowAttributes {
                window,
                value_list: &[x::Cw::BorderPixel(pixel)],
            });

        let _ = self.conn.send_and_check_request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::BorderWidth(width)],
        });
    }

    /*

      ▄    ▄▄ ▗▄▄▄▖ ▄▄▄  ▗▄▖ ▗▄ ▗▖     ▗▖ ▗▖  ▄  ▗▄ ▗▖▗▄▄  ▗▖   ▗▄▄▄▖▗▄▄▖  ▗▄▖
     ▐█▌  █▀▀▌▝▀█▀▘ ▀█▀  █▀█ ▐█ ▐▌     ▐▌ ▐▌ ▐█▌ ▐█ ▐▌▐▛▀█ ▐▌   ▐▛▀▀▘▐▛▀▜▌▗▛▀▜
     ▐█▌ ▐▛     █    █  ▐▌ ▐▌▐▛▌▐▌     ▐▌ ▐▌ ▐█▌ ▐▛▌▐▌▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▙
     █ █ ▐▌     █    █  ▐▌ ▐▌▐▌█▐▌     ▐███▌ █ █ ▐▌█▐▌▐▌ ▐▌▐▌   ▐███ ▐███  ▜█▙
     ███ ▐▙     █    █  ▐▌ ▐▌▐▌▐▟▌     ▐▌ ▐▌ ███ ▐▌▐▟▌▐▌ ▐▌▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▗█ █▖ █▄▄▌  █   ▄█▄  █▄█ ▐▌ █▌     ▐▌ ▐▌▗█ █▖▐▌ █▌▐▙▄█ ▐▙▄▄▖▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▘ ▝▘  ▀▀   ▀   ▀▀▀  ▝▀▘ ▝▘ ▀▘     ▝▘ ▝▘▝▘ ▝▘▝▘ ▀▘▝▀▀  ▝▀▀▀▘▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn spawn_client(&self, cmd: &str) {
        info!("Spawning command: {}", cmd);
        match Command::new(cmd).spawn() {
            Ok(_) => info!("Successfully spawned: {}", cmd),
            Err(e) => error!("Failed to spawn {}: {:?}", cmd, e),
        }
    }

    fn kill_client(&mut self) {
        if let Some(window_to_kill) = self.current_workspace_mut().removed_focused_window() {
            info!("Killing client window: {:?}", window_to_kill);

            // Send KillClient request
            match self.conn.send_and_check_request(&x::KillClient {
                resource: window_to_kill.resource_id(),
            }) {
                Ok(_) => info!("Successfully killed window: {:?}", window_to_kill),
                Err(e) => error!("Failed to kill window {:?}: {:?}", window_to_kill, e),
            }

            // Reconfigure remaining workspaces
            self.shift_focus(0);
            self.configure_windows();
        }
    }

    fn shift_focus(&mut self, direction: isize) {
        let curr_workspace = self.current_workspace_mut();
        let window_count = curr_workspace.num_of_windows() as isize;

        if window_count == 0 {
            debug!("No windows to focus");
            return;
        }

        let curr = curr_workspace.get_focus().unwrap_or(0) as isize;
        let next_focus: usize = ((curr + direction).rem_euclid(window_count)) as usize;

        debug!("Focus shifted to window index: {}", next_focus);
        self.set_focus(next_focus);
    }

    fn change_workspace(&mut self, new_workspace: usize) {
        if self.workspace == new_workspace || new_workspace >= NUM_WORKSPACES {
            return;
        }
        let old_wspace_cookies: Vec<_> = self
            .current_workspace()
            .iter_windows()
            .map(|win| {
                self.conn
                    .send_request_checked(&x::UnmapWindow { window: *win })
            })
            .collect();

        self.workspace = new_workspace;
        let new_wspace_cookies: Vec<_> = self
            .current_workspace()
            .iter_windows()
            .map(|win| {
                self.conn
                    .send_request_checked(&x::MapWindow { window: *win })
            })
            .collect();

        old_wspace_cookies.into_iter().for_each(|cookie| {
            let _ = self.conn.check_request(cookie);
        });
        new_wspace_cookies.into_iter().for_each(|cookie| {
            let _ = self.conn.check_request(cookie);
        });

        self.update_current_workspace();
    }

    /*

    ▗▄▄▄▖▗▖ ▗▖▗▄▄▄▖▗▄ ▗▖▗▄▄▄▖     ▗▖ ▗▖  ▄  ▗▄ ▗▖▗▄▄  ▗▖   ▗▄▄▄▖▗▄▄▖  ▗▄▖
    ▐▛▀▀▘▝█ █▘▐▛▀▀▘▐█ ▐▌▝▀█▀▘     ▐▌ ▐▌ ▐█▌ ▐█ ▐▌▐▛▀█ ▐▌   ▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▐▌    █ █ ▐▌   ▐▛▌▐▌  █       ▐▌ ▐▌ ▐█▌ ▐▛▌▐▌▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▙
    ▐███  █ █ ▐███ ▐▌█▐▌  █       ▐███▌ █ █ ▐▌█▐▌▐▌ ▐▌▐▌   ▐███ ▐███  ▜█▙
    ▐▌    ▐█▌ ▐▌   ▐▌▐▟▌  █       ▐▌ ▐▌ ███ ▐▌▐▟▌▐▌ ▐▌▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐▙▄▄▖ ▐█▌ ▐▙▄▄▖▐▌ █▌  █       ▐▌ ▐▌▗█ █▖▐▌ █▌▐▙▄█ ▐▙▄▄▖▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▀▀▀▘ ▝▀▘ ▝▀▀▀▘▝▘ ▀▘  ▀       ▝▘ ▝▘▝▘ ▝▘▝▘ ▀▘▝▀▀  ▝▀▀▀▘▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn handle_key_press(&mut self, ev: &x::KeyPressEvent) {
        let keycode = ev.detail();
        let modifiers = ModMask::from_bits_truncate(ev.state().bits());

        if let Some(action) = self.key_bindings.get(&(keycode, modifiers)) {
            match action {
                ActionEvent::Spawn(cmd) => self.spawn_client(cmd),
                ActionEvent::KillClient => self.kill_client(),
                ActionEvent::FocusNext => self.shift_focus(1),
                ActionEvent::FocusPrev => self.shift_focus(-1),
                ActionEvent::Workspace(workspace) => self.change_workspace(*workspace),
            }
        } else {
            println!(
                "No binding found for keycode: {} with modifiers: {:?}",
                keycode, modifiers
            );
        }
    }

    fn handle_map_request(&mut self, window: Window) {
        self.current_workspace_mut().push_window(window);
        self.configure_windows();
        match self.conn.send_and_check_request(&x::MapWindow { window }) {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to map window {:?}: {:?}", window, e);
            }
        }
        let idx = self.current_workspace().num_of_windows().saturating_sub(1);
        self.set_focus(idx);
    }

    fn handle_destroy_event(&mut self, window: Window) {
        let curr_workspace = self.current_workspace_mut();

        if curr_workspace.num_of_windows() == 0 {
            debug!("No window to destroy");
            return;
        }

        curr_workspace.retain(|&win| win.resource_id() != window.resource_id());

        if let Some(focused) = curr_workspace.get_focused_window() {
            if focused.resource_id() == window.resource_id() {
                self.shift_focus(-1);
            }
        }

        self.configure_windows();
    }

    /*

    ▗▄ ▄▖  ▄   ▄▄▄ ▗▄ ▗▖     ▗▖    ▗▄▖  ▗▄▖ ▗▄▄▖
    ▐█ █▌ ▐█▌  ▀█▀ ▐█ ▐▌     ▐▌    █▀█  █▀█ ▐▛▀▜▖
    ▐███▌ ▐█▌   █  ▐▛▌▐▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐▌ ▐▌
    ▐▌█▐▌ █ █   █  ▐▌█▐▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐██▛
    ▐▌▀▐▌ ███   █  ▐▌▐▟▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐▌
    ▐▌ ▐▌▗█ █▖ ▄█▄ ▐▌ █▌     ▐▙▄▄▖ █▄█  █▄█ ▐▌
    ▝▘ ▝▘▝▘ ▝▘ ▀▀▀ ▝▘ ▀▘     ▝▀▀▀▘ ▝▀▘  ▝▀▘ ▝▘

    */

    pub fn run(&mut self) -> xcb::Result<()> {
        loop {
            match self.conn.wait_for_event()? {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    debug!("Received KeyPress event: {:?}", ev);
                    self.handle_key_press(&ev);
                }

                xcb::Event::X(x::Event::MapRequest(ev)) => {
                    debug!("Received MapRequest event for window: {:?}", ev.window());
                    self.handle_map_request(ev.window());
                }

                xcb::Event::X(x::Event::DestroyNotify(ev)) => {
                    debug!("Received DestroyNotify event for window {:?}", ev.window());
                    self.handle_destroy_event(ev.window());
                }

                xcb::Event::X(x::Event::MapNotify(ev)) => {
                    debug!("Window mapped: {:?}", ev.window());
                }

                ev => {
                    debug!("Ignoring event: {:?}", ev);
                }
            }
        }
    }
}
