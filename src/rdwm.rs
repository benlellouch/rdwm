use log::{debug, error, info, warn};
use std::process::Command;
use std::{collections::HashMap, process::Stdio};
use xcb::{
    x::{self, Cw, EventMask, ModMask, Window},
    Connection, ProtocolError, VoidCookieChecked, Xid,
};

use crate::atoms::Atoms;
use crate::config::{DEFAULT_BORDER_WIDTH, DEFAULT_WINDOW_GAP, NUM_WORKSPACES};
use crate::key_mapping::ActionEvent;
use crate::keyboard::{fetch_keyboard_mapping, populate_key_bindings, set_keygrabs};
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
    screen_height_usable: u32,
    focused_border_pixel: u32,
    normal_border_pixel: u32,
    border_width: u32,
    window_gap: u32,
    atoms: Atoms,
    root_window: Window,
    wm_check_window: Window,
    dock_windows: Vec<Window>,
    dock_height: u32,
}

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, _) = Connection::connect(None)?;
        info!("Connected to X.");

        // Initialize configuration before creating WindowManager
        let config = Self::initialize_config(&conn)?;

        // Create WM check window
        let wm_check_window = Self::create_wm_check_window(&conn, config.root_window);

        let dock_height = 30u32;
        let screen_height_usable = config.screen_config.height.saturating_sub(dock_height);

        let wm = WindowManager {
            conn,
            workspaces: Default::default(),
            workspace: 0,
            key_bindings: config.key_bindings,
            screen_width: config.screen_config.width,
            screen_height: config.screen_config.height,
            screen_height_usable,
            focused_border_pixel: config.screen_config.focused_border_pixel,
            normal_border_pixel: config.screen_config.normal_border_pixel,
            border_width: DEFAULT_BORDER_WIDTH,
            window_gap: DEFAULT_WINDOW_GAP,
            atoms: config.atoms,
            root_window: config.root_window,
            wm_check_window,
            dock_windows: Vec::new(),
            dock_height,
        };

        // Get root window and set up substructure redirect
        wm.set_substructure_redirect()?;
        info!("Successfully set substructure redirect");

        // Set up key grabs
        set_keygrabs(&wm.conn, &wm.key_bindings, wm.root_window());

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
        let (keysyms, keysyms_per_keycode) = fetch_keyboard_mapping(conn);
        let key_bindings = populate_key_bindings(conn, &keysyms, keysyms_per_keycode);
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
        let values = [x::Cw::OverrideRedirect(true)];
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

    fn is_dock_window(&self, window: Window) -> bool {
        // Query _NET_WM_WINDOW_TYPE property
        let cookie = self.conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: self.atoms.net_wm_window_type,
            r#type: x::ATOM_ATOM,
            long_offset: 0,
            long_length: 32,
        });

        if let Ok(reply) = self.conn.wait_for_reply(cookie) {
            let atoms_vec: &[x::Atom] = reply.value();
            // Check if the window type includes _NET_WM_WINDOW_TYPE_DOCK
            for atom in atoms_vec {
                if atom.resource_id() == self.atoms.net_wm_window_type_dock.resource_id() {
                    debug!("Window {:?} identified as dock window", window);
                    return true;
                }
            }
        }
        false
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
            self.atoms.net_wm_window_type,
            self.atoms.net_wm_window_type_dock,
        ];

        Atoms::set_atom(
            &self.conn,
            self.root_window(),
            self.atoms.net_supported,
            &supported_atoms
                .iter()
                .map(|a| a.resource_id())
                .collect::<Vec<_>>(),
        );

        // Publish desktop information
        Atoms::set_cardinal32(
            &self.conn,
            self.root_window(),
            self.atoms.net_number_of_desktops,
            &[NUM_WORKSPACES as u32],
        );
        Atoms::set_cardinal32(
            &self.conn,
            self.root_window(),
            self.atoms.net_current_desktop,
            &[0 as u32],
        );

        info!("Published EWMH hints successfully");
    }

    fn update_current_workspace(&self) {
        Atoms::set_cardinal32(
            &self.conn,
            self.root_window(),
            self.atoms.net_current_desktop,
            &[self.workspace as u32],
        );
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

    fn get_workspace(&self, workspace_id: usize) -> Option<&Workspace> {
        self.workspaces.get(workspace_id)
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

    fn configure_windows(&self, workspace_id: usize) {
        if let Some(workspace) = self.get_workspace(workspace_id) {
            let tiled_windows: Vec<_> = workspace.iter_tiled_windows().collect();
            if tiled_windows.is_empty() {
                debug!("No windows to configure");
                return;
            }

            let total_size: u32 = tiled_windows.iter().map(|tw| tw.size()).sum();
            let border_width = self.border_width + self.window_gap;
            let inner_h = (self.screen_height_usable - 2 * border_width).max(1);
            let screen_partitions = self.screen_width / total_size;

            let mut cumulative = 0u32;
            let config_cookies: Vec<_> = tiled_windows
                .iter()
                .map(|twin| {
                    let cell = (self.screen_width * twin.size()) / total_size;
                    let inner_w = (cell - 2 * border_width).max(1);
                    let x = (cumulative * screen_partitions + self.window_gap) as i32;
                    cumulative += twin.size();
                    self.configure_window(
                        twin.window(),
                        x,
                        self.window_gap as i32,
                        inner_w,
                        inner_h,
                    )
                })
                .collect();

            config_cookies.into_iter().for_each(|cookie| {
                if let Err(e) = self.conn.check_request(cookie) {
                    warn!("Failed to configure window: {:?}", e);
                }
            });
        }
    }

    fn configure_dock_windows(&self) {
        let dock_y = (self.screen_height as i32) - (self.dock_height as i32);

        for window in &self.dock_windows {
            let config_values = [
                x::ConfigWindow::X(0),
                x::ConfigWindow::Y(dock_y),
                x::ConfigWindow::Width(self.screen_width),
                x::ConfigWindow::Height(self.dock_height),
            ];

            let _ = self.conn.send_and_check_request(&x::ConfigureWindow {
                window: *window,
                value_list: &config_values,
            });
        }
    }

    fn set_focus(&mut self, idx: usize) {
        // Reset border on old focused window (if any)
        if let Some(old_window) = self.current_workspace().get_focused_window().copied() {
            self.unfocus_window(old_window);
            debug!("Reset border on old focused window");
        }

        self.current_workspace_mut().set_focus(idx);

        // Set border on window to be focused (if present)
        if let Some(new_focus_window) = self.current_workspace().get_focused_window().copied() {
            self.focus_window(new_focus_window);
            let _ = self.conn.send_and_check_request(&x::SetInputFocus {
                revert_to: x::InputFocus::PointerRoot,
                focus: new_focus_window,
                time: 0,
            });
        }
    }

    fn focus_window(&self, window: Window) {
        self.set_window_border(window, self.focused_border_pixel, self.border_width);
    }

    fn unfocus_window(&self, window: Window) {
        self.set_window_border(window, self.normal_border_pixel, self.border_width);
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
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            error!("Empty command provided");
            return;
        }

        let mut command = Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }

        match command.spawn() {
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
        }
    }

    fn next_window_index(&mut self, direction: isize) -> Option<usize> {
        let curr_workspace = self.current_workspace_mut();
        let window_count: isize = curr_workspace.num_of_windows() as isize;

        if window_count == 0 {
            debug!("No windows to focus");
            return None;
        }

        let curr = curr_workspace.get_focus().unwrap_or(0) as isize;
        Some(((curr + direction).rem_euclid(window_count)) as usize)
    }

    fn shift_focus(&mut self, direction: isize) {
        if let Some(next_focus) = self.next_window_index(direction) {
            debug!("Focus shifted to window index: {}", next_focus);
            self.set_focus(next_focus);
        }
    }

    fn swap_window(&mut self, direction: isize) {
        if let Some(next_window) = self.next_window_index(direction) {
            let curr_workspace = self.current_workspace_mut();
            match curr_workspace.get_focus() {
                Some(focus) => {
                    curr_workspace.swap_windows(focus, next_window);
                    self.set_focus(next_window);
                    self.configure_windows(self.workspace);
                }
                None => error!(
                    "Failed to get focus for current workspace {}",
                    self.workspace
                ),
            }
        }
    }

    fn increase_window_weight(&mut self, increment: u32) {
        if let Some(focused_win) = self.current_workspace_mut().get_focused_tiled_window_mut() {
            focused_win.increase_window_size(increment);
            self.configure_windows(self.workspace);
        }
    }

    fn decrease_window_weight(&mut self, increment: u32) {
        if let Some(focused_win) = self.current_workspace_mut().get_focused_tiled_window_mut() {
            focused_win.decrease_window_size(increment);
            self.configure_windows(self.workspace);
        }
    }

    fn increase_window_gap(&mut self, increment: u32) {
        self.window_gap += increment;
        self.configure_windows(self.workspace);
    }

    fn decrease_window_gap(&mut self, increment: u32) {
        if self.window_gap > 0 {
            self.window_gap -= increment;
            self.configure_windows(self.workspace);
        }
    }

    fn go_to_workspace(&mut self, new_workspace_id: usize) {
        if self.workspace == new_workspace_id || new_workspace_id >= NUM_WORKSPACES {
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

        self.workspace = new_workspace_id;
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
        if let Some(focus) = self.current_workspace().get_focus() {
            self.set_focus(focus);
        }
    }

    fn send_to_workspace(&mut self, workspace_id: usize) {
        match self.current_workspace_mut().removed_focused_window() {
            Some(window_to_send) => {
                if let Some(new_workspace) = self.workspaces.get_mut(workspace_id) {
                    new_workspace.push_window(window_to_send);
                    let _ = self.conn.send_and_check_request(&x::UnmapWindow {
                        window: window_to_send,
                    });
                    self.unfocus_window(window_to_send);
                    self.configure_windows(self.workspace);
                    self.configure_windows(workspace_id);
                    self.shift_focus(0);
                }
            }
            None => error!(
                "Failed to remove focused window from workspace {}",
                self.workspace
            ),
        }
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
                ActionEvent::Kill => self.kill_client(),
                ActionEvent::NextWindow => self.shift_focus(1),
                ActionEvent::PrevWindow => self.shift_focus(-1),
                ActionEvent::GoToWorkspace(workspace_id) => self.go_to_workspace(*workspace_id),
                ActionEvent::SendToWorkspace(workspace_id) => self.send_to_workspace(*workspace_id),
                ActionEvent::SwapRight => self.swap_window(1),
                ActionEvent::SwapLeft => self.swap_window(-1),
                ActionEvent::IncreaseWindowWeight(increment) => {
                    self.increase_window_weight(*increment)
                }
                ActionEvent::DecreaseWindowWeight(increment) => {
                    self.decrease_window_weight(*increment)
                }
                ActionEvent::IncreaseWindowGap(increment) => self.increase_window_gap(*increment),
                ActionEvent::DecreaseWindowGap(increment) => self.decrease_window_gap(*increment),
            }
        } else {
            error!(
                "No binding found for keycode: {} with modifiers: {:?}",
                keycode, modifiers
            );
        }
    }

    fn handle_map_request(&mut self, window: Window) {
        // Check if this is a dock window
        if self.is_dock_window(window) {
            debug!("Mapping dock window: {:?}", window);
            self.dock_windows.push(window);
            match self.conn.send_and_check_request(&x::MapWindow { window }) {
                Ok(_) => {
                    info!("Successfully mapped dock window: {:?}", window);
                    self.configure_dock_windows();
                }
                Err(e) => {
                    error!("Failed to map dock window {:?}: {:?}", window, e);
                }
            }
        } else {
            // Regular window - add to current workspace
            self.current_workspace_mut().push_window(window);
            self.configure_windows(self.workspace);
            match self.conn.send_and_check_request(&x::MapWindow { window }) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to map window {:?}: {:?}", window, e);
                }
            }
            let idx = self.current_workspace().num_of_windows().saturating_sub(1);
            self.set_focus(idx);
        }
    }

    fn handle_destroy_event(&mut self, window: Window) {
        // Check if it's a dock window
        let window_id = window.resource_id();
        let was_dock = self
            .dock_windows
            .iter()
            .any(|w| w.resource_id() == window_id);

        if was_dock {
            debug!("Dock window destroyed: {:?}", window);
            self.dock_windows.retain(|w| w.resource_id() != window_id);
            return;
        }

        // Handle regular window destruction
        let curr_workspace = self.current_workspace_mut();

        if curr_workspace.num_of_windows() == 0 {
            debug!("No window to destroy");
            return;
        }
        curr_workspace.retain(|&win| win.resource_id() != window_id);
        self.shift_focus(0);
        self.configure_windows(self.workspace);
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

    fn spawn_autostart() {
        match Command::new("sh")
            .arg("-c")
            .arg("exec ~/.config/rdwm/autostart.sh")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => debug!("Ran autostart succesfully!"),
            Err(e) => debug!("Failed to run autostart: {:?}", e),
        };
    }

    pub fn run(&mut self) -> xcb::Result<()> {
        Self::spawn_autostart();
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

                xcb::Event::X(x::Event::UnmapNotify(ev)) => {
                    debug!("Received UnmapNotify event for window {:?}", ev.window());
                    // self.handle_destroy_event(ev.window());
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
