use std::collections::HashMap;
use std::process::Command;
use xcb::{
    Connection, ProtocolError, Xid, x::{self, Cw, EventMask, ModMask, Window}
};

use crate::config::ACTION_MAPPINGS;
use crate::key_mapping::ActionEvent;

pub struct WindowManager {
    conn: Connection,
    windows: Vec<Window>,
    key_bindings: HashMap<(u8, ModMask), ActionEvent>,
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
            windows: vec![],
            key_bindings: HashMap::new(),
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
                    wm.key_bindings
                        .insert((keycode, modifiers), mapping.action);
                    println!(
                        "Mapped key {:?} (keycode: {}) with modifiers {:?} to action: {:?}",
                        mapping.key, keycode, modifiers, mapping.action
                    );
                    break;
                }
            }
        }

        // Get root window and set up substructure redirect
        let root = wm.conn.get_setup().roots().next().unwrap().root();
        wm.set_substructure_redirect(root)?;
        println!("Successfully set substructure redirect");

        // Set up key grabs
        wm.set_keygrabs(root);

        Ok(wm)
    }

    fn configure_window(
        &self,
        window: Window,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), ProtocolError> {
        let config_values = [
            x::ConfigWindow::X(x),
            x::ConfigWindow::Y(y),
            x::ConfigWindow::Width(width),
            x::ConfigWindow::Height(height),
        ];

        self.conn.send_and_check_request(&x::ConfigureWindow {
            window,
            value_list: &config_values,
        })
    }

    fn handle_key_press(&mut self, ev: &x::KeyPressEvent) {
        let keycode = ev.detail();
        let modifiers = ModMask::from_bits_truncate(ev.state().bits());

        if let Some(action) = self.key_bindings.get(&(keycode, modifiers)) {
            match action {
                ActionEvent::Spawn(cmd) => {
                    println!("Spawning command: {}", cmd);

                    match Command::new(cmd).spawn() {
                        Ok(_) => println!("Successfully spawned: {}", cmd),
                        Err(e) => println!("Failed to spawn {}: {:?}", cmd, e),
                    }
                }
                ActionEvent::KillClient => {
                    let window_to_kill: Option<Window> = self.windows.pop();
                    match window_to_kill {
                        Some(win) => {
                            println!("Killing client window: {:?}", win);
                            match self.conn.send_and_check_request(&x::KillClient { resource: win.resource_id() }) {
                                Ok(_) => println!("Successfully killed window: {:?}", win),
                                Err(e) => println!("Failed to kill window {:?}: {:?}", win, e),
                            }
                        }
                        None => {
                            println!("No windows to kill");
                        }
                    }
                }
            }
        } else {
            println!(
                "No binding found for keycode: {} with modifiers: {:?}",
                keycode, modifiers
            );
        }
    }
        

    fn handle_map_request(&mut self, window: Window, screen_width: u32, screen_height: u32) {
        // push new window to list
        self.windows.push(window);

        // Calculate horizontal tiling layout
        let window_width = screen_width / self.windows.len() as u32;
        let window_height = screen_height;

        for (i, win) in self.windows.iter().enumerate() {
            let x = i as i32 * window_width as i32;
            match self.configure_window(*win, x, 0, window_width, window_height) {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to configure window {:?}: {:?}", win, e);
                }
            }
        }

        match self.conn.send_and_check_request(&x::MapWindow { window }) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to map window {:?}: {:?}", window, e);
            }
        }
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

    pub fn run(&mut self) -> xcb::Result<()> {
        let root = self.conn.get_setup().roots().next().unwrap();
        println!("Root window: {:?}", root.root());

        // Get screen dimensions once
        let screen_width = root.width_in_pixels() as u32;
        let screen_height = root.height_in_pixels() as u32;

        loop {
            match self.conn.wait_for_event()? {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    println!("Received KeyPress event: {:?}", ev);
                    self.handle_key_press(&ev);
                }

                xcb::Event::X(x::Event::MapRequest(ev)) => {
                    println!("Received MapRequest event for window: {:?}", ev.window());
                    self.handle_map_request(ev.window(), screen_width, screen_height);
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
                    if !self.windows.contains(&ev.window()) {
                        println!("  -> New manageable window detected, treating as MapRequest");
                        self.handle_map_request(ev.window(), screen_width, screen_height);
                    }
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
