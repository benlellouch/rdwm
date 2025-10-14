use std::process::Command;
use xcb::{
    x::{self, Cw, EventMask , Window, ModMask}, Connection, ProtocolError
};
use crate::config::ACTION_MAPPINGS;

pub struct WindowManager{
    conn: Connection,
    windows: Vec<Window>,
}

impl WindowManager {
    pub fn new() -> Self {
        let (conn, _) = Connection::connect(None).unwrap();
        WindowManager {
            conn,
            windows: vec![],
        }
    }

    // we might not need to reparent and just add border to current might just be easier tbh 
    fn frame(&self, window_to_frame: Window) -> Window{
        let border_width: u8 = 3;
        let border_color: u32 = 0xff0000;
        let bg_color: u32 = 0x0000ff;

        // match self.conn.wait_for_reply(self.conn.send_request(&x::GetWindowAttributes { window: window_to_frame})) {
        //    Ok(attributes)=> {
        //     let frame: Window = self.conn.generate_id();
        //     self.conn.send_request_checked(&x::CreateWindow {
        //         depth: x::COPY_FROM_PARENT as u8,
        //         wid: frame,
        //         parent: self.conn.get_setup().roots().next().unwrap(),
        //         x:attributes.,
        //         y: 0,


        //     })
        //    }
        //    Err(e) => {
        //     println!("Failed to frame window: {:?}", window_to_frame);
        //     window_to_frame
        //    } 
        // }
        window_to_frame    
    }

    fn configure_window(&self, window: Window, x: i32, y: i32, width: u32, height: u32) -> Result<(), ProtocolError> {
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

    fn handle_key_press(&self, ev: &x::KeyPressEvent) {
        if ev.detail() == 0x18 {
            println!("Attempting to spawn new process");
            // Try a simpler application first
            Command::new("xterm").spawn().expect("Failed to Spawn Window");
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
            match self.configure_window(
                *win,
                x,
                0,
                window_width,
                window_height
            ) {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to configure window {:?}: {:?}", win, e);
                }
            }
        }

        match self.conn.send_and_check_request(&x::MapWindow {
            window,
        }) {
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
        self.conn.send_and_check_request(&x::ChangeWindowAttributes {
            window: root,
            value_list: &values,
        })
    }

    // fn set_keybindings(&self, root: Window) {
    //     for mapping in ACTION_MAPPINGS {
    //         let keycode = xcb::x::Keysym::fr(&self.conn, mapping.key);
    //         let modifiers = mapping.modifiers.iter().fold(xcb::x::ModMask::empty(), |acc, &modkey| {
    //             acc | match modkey {
    //                 "Shift" => xcb::x::ModMask::SHIFT,
    //                 "Lock" => xcb::x::ModMask::LOCK,
    //                 "Control" => xcb::x::ModMask::CONTROL,
    //                 "Mod1" => xcb::x::ModMask::N1,
    //                 "Mod2" => xcb::x::ModMask::N2,
    //                 "Mod3" => xcb::x::ModMask::N3,
    //                 "Mod4" => xcb::x::ModMask::N4,
    //                 "Mod5" => xcb::x::ModMask::N5,
    //             }
    //         });

    //         if let Some(kc) = keycode {
    //             self.conn.send_and_check_request(&x::GrabKey {
    //                 owner_events: false,
    //                 grab_window: root,
    //                 modifiers,
    //                 key: kc,
    //                 pointer_mode: xcb::x::GrabMode::Async,
    //                 keyboard_mode: xcb::x::GrabMode::Async,
    //             }).expect("Failed to grab key");
    //         } else {
    //             println!("Warning: No keycode found for keysym '{}'", mapping.key);
    //         }
    //     }
    // }


    pub fn run(&mut self) -> xcb::Result<()> {

        let root = self.conn.get_setup().roots().next().unwrap();
        print!("Root window: {:?}", root.root());
        
        // Get screen dimensions once
        let screen_width = root.width_in_pixels() as u32;
        let screen_height = root.height_in_pixels() as u32;

        match self.set_substructure_redirect(root.root()) {
            Ok(_) => println!("Succesfully set substructure redirect"),
            Err(e) => {
                println!("Cannot set attributes: {:?}", e);
            }
        }

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
                    println!("Received ConfigureRequest event for window: {:?}", ev.window());
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
