use std::collections::HashMap;
use std::process::Command;
use xcb::{
    x::{self, Cw, EventMask, ModMask, Window},
    Connection, ProtocolError, VoidCookieChecked, Xid,
};

use crate::config::{ACTION_MAPPINGS, DEFAULT_BORDER_WIDTH, NUM_WORKSPACES};
use crate::key_mapping::ActionEvent;
use crate::workspace::Workspace;

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
}

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, _) = Connection::connect(None)?;
        println!("Connected to X.");

        // Get keyboard mapping from X server
        let (keysyms, keysyms_per_keycode) = if let Ok(keyboard_mapping) =
            conn.wait_for_reply(conn.send_request(&x::GetKeyboardMapping {
                first_keycode: conn.get_setup().min_keycode(),
                count: conn.get_setup().max_keycode() - conn.get_setup().min_keycode() + 1,
            })) {
            let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode() as usize;
            let keysyms = keyboard_mapping.keysyms().to_vec();
            (keysyms, keysyms_per_keycode)
        } else {
            println!("Failed to get keyboard mapping, using empty keysyms");
            (vec![], 0)
        };

        let mut wm = WindowManager {
            conn,
            workspaces: Default::default(),
            workspace: 0,
            key_bindings: HashMap::new(),
            screen_width: 0,
            screen_height: 0,
            focused_border_pixel: 0,
            normal_border_pixel: 0,
            border_width: DEFAULT_BORDER_WIDTH, // choose desired border thickness
        };

        // Create key bindings HashMap
        for mapping in ACTION_MAPPINGS {
            let modifiers = mapping
                .modifiers
                .iter()
                .copied()
                .reduce(|acc, modkey| acc | modkey)
                .unwrap_or(xcb::x::ModMask::empty());

            // Find keycode for this keysym
            for (i, chunk) in keysyms.chunks(keysyms_per_keycode).enumerate() {
                if chunk.contains(&mapping.key.raw()) {
                    let keycode = wm.conn.get_setup().min_keycode() + i as u8;
                    wm.key_bindings.insert((keycode, modifiers), mapping.action);
                    println!(
                        "Mapped key {:?} (keycode: {}) with modifiers {:?} to action: {:?}",
                        mapping.key, keycode, modifiers, mapping.action
                    );
                    break;
                }
            }
        }

        let root_screen = wm.conn.get_setup().roots().next().unwrap();
        wm.screen_width = root_screen.width_in_pixels() as u32;
        wm.screen_height = root_screen.height_in_pixels() as u32;

        wm.focused_border_pixel = root_screen.white_pixel();
        wm.normal_border_pixel = root_screen.black_pixel();

        // Get root window and set up substructure redirect
        let root = root_screen.root();
        wm.set_substructure_redirect(root)?;
        println!("Successfully set substructure redirect");

        // Set up key grabs
        wm.set_keygrabs(root);

        Ok(wm)
    }

    fn set_substructure_redirect(&self, root: Window) -> Result<(), ProtocolError> {
        let values = [Cw::EventMask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::KEY_PRESS,
        )];
        self.conn
            .send_and_check_request(&x::ChangeWindowAttributes {
                window: root,
                value_list: &values,
            })
    }

    fn set_keygrabs(&self, root: Window) {
        for &(keycode, modifiers) in self.key_bindings.keys() {
            match self.conn.send_and_check_request(&x::GrabKey {
                owner_events: false,
                grab_window: root,
                modifiers,
                key: keycode,
                pointer_mode: x::GrabMode::Async,
                keyboard_mode: x::GrabMode::Async,
            }) {
                Ok(_) => println!(
                    "Successfully grabbed key: keycode {} with modifiers {:?}",
                    keycode, modifiers
                ),
                Err(e) => println!("Failed to grab key {}: {:?}", keycode, e),
            }
        }
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
            println!("No windows to configure");
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
        // Reset border on old focused window
        self.set_window_border(
            *self.current_workspace().get_focused_window(),
            self.normal_border_pixel,
            self.border_width,
        );

        println!("Reset border on old focused window");

        self.current_workspace_mut().set_focus(idx);
        // Set border on window to be focused
        let new_focus_window = *self.current_workspace().get_focused_window();
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
        _    ____ _____ ___ ___  _   _   _   _    _    _   _ ____  _     _____ ____  ____
       / \  / ___|_   _|_ _/ _ \| \ | | | | | |  / \  | \ | |  _ \| |   | ____|  _ \/ ___|
      / _ \| |     | |  | | | | |  \| | | |_| | / _ \ |  \| | | | | |   |  _| | |_) \___ \
     / ___ \ |___  | |  | | |_| | |\  | |  _  |/ ___ \| |\  | |_| | |___| |___|  _ < ___) |
    /_/   \_\____| |_| |___\___/|_| \_| |_| |_/_/   \_\_| \_|____/|_____|_____|_| \_\____/

    */

    fn spawn_client(&self, cmd: &str) {
        println!("Spawning command: {}", cmd);
        match Command::new(cmd).spawn() {
            Ok(_) => println!("Successfully spawned: {}", cmd),
            Err(e) => println!("Failed to spawn {}: {:?}", cmd, e),
        }
    }

    fn kill_client(&mut self) {
        if let Some(window_to_kill) = self.current_workspace_mut().removed_focused_window() {
            println!("Killing client window: {:?}", window_to_kill);

            // Send KillClient request
            match self.conn.send_and_check_request(&x::KillClient {
                resource: window_to_kill.resource_id(),
            }) {
                Ok(_) => println!("Successfully killed window: {:?}", window_to_kill),
                Err(e) => println!("Failed to kill window {:?}: {:?}", window_to_kill, e),
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
            println!("No windows to focus");
            return;
        }

        let next_focus: usize = ((curr_workspace.get_focus() as isize + direction + window_count)
            % window_count) as usize;

        println!("Focus shifted to window index: {}", next_focus);
        self.set_focus(next_focus);
    }

    fn change_workspace(&mut self, new_workspace: usize) {
        if self.workspace == new_workspace {
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
    }

    /*
      _______     _______ _   _ _____   _   _    _    _   _ ____  _     _____ ____  ____
    | ____\ \   / / ____| \ | |_   _| | | | |  / \  | \ | |  _ \| |   | ____|  _ \/ ___|
    |  _|  \ \ / /|  _| |  \| | | |   | |_| | / _ \ |  \| | | | | |   |  _| | |_) \___ \
    | |___  \ V / | |___| |\  | | |   |  _  |/ ___ \| |\  | |_| | |___| |___|  _ < ___) |
    |_____|  \_/  |_____|_| \_| |_|   |_| |_/_/   \_\_| \_|____/|_____|_____|_| \_\____/

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
                _ => {
                    println!("Action {:?} not implemented yet", action);
                }
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
                println!("Failed to map window {:?}: {:?}", window, e);
            }
        }
        self.set_focus(self.current_workspace().num_of_windows() - 1);
    }

    fn handle_destroy_event(&mut self, window: Window) {
        let curr_workspace = self.current_workspace_mut();

        if curr_workspace.num_of_windows() == 0 {
            println!("No window to destroy");
            return;
        }

        curr_workspace.retain(|&win| win.resource_id() != window.resource_id());

        if curr_workspace.get_focused_window().resource_id() == window.resource_id() {
            self.shift_focus(-1);
        }

        self.configure_windows();
    }

    pub fn run(&mut self) -> xcb::Result<()> {
        loop {
            match self.conn.wait_for_event()? {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    println!("Received KeyPress event: {:?}", ev);
                    self.handle_key_press(&ev);
                }

                xcb::Event::X(x::Event::MapRequest(ev)) => {
                    println!("Received MapRequest event for window: {:?}", ev.window());
                    self.handle_map_request(ev.window());
                }

                xcb::Event::X(x::Event::ConfigureRequest(ev)) => {
                    println!(
                        "Received ConfigureRequest event for window: {:?}",
                        ev.window()
                    );
                    println!("  Parent: {:?}", ev.parent());
                    println!("  Requested position: ({}, {})", ev.x(), ev.y());
                    println!("  Requested size: {}x{}", ev.width(), ev.height());

                    // Check if this is a new window
                }

                xcb::Event::X(x::Event::DestroyNotify(ev)) => {
                    println!("Received DestroyNotify event for window {:?}", ev.window());
                    self.handle_destroy_event(ev.window());
                }

                xcb::Event::X(x::Event::MapNotify(ev)) => {
                    println!("Window mapped: {:?}", ev.window());
                }

                ev => {
                    println!("Ignoring event: {:?}", ev);
                }
            }
        }
    }
}
