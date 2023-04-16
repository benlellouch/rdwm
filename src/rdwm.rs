use std::process::Command;
use xcb::{
    x::{self, Cw, EventMask, Screen},
    Connection, Event,
};

pub struct WindowManager {
    conn: Connection,
    // screen: &Screen
}

impl WindowManager {
    pub fn new() -> Self {
        let (conn, screen_num) = Connection::connect(None).unwrap();
        println!("Connected to X.");
        // WindowManager { conn, screen}
        WindowManager { conn }
    }

    pub fn run(&self) -> xcb::Result<()> {
        let setup = self.conn.get_setup();
        let screen = setup.roots().next().unwrap();

        let values = [Cw::EventMask(
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY | EventMask::KEY_PRESS,
        )];

        let cookie = self.conn.send_request_checked(&x::ChangeWindowAttributes {
            window: screen.root(),
            value_list: &values,
        });

        match (self.conn.check_request(cookie)) {
            Ok(_) => println!("Succesfully set substructure redirect"), 
            Err(e) => println!("Cannot set attributes: {:?}", e),
        }

        // let values = [Cw::EventMask(EventMask::KEY_PRESS)];

        // let cookie = self.conn.send_request_checked(&x::ChangeWindowAttributes {
        //     window: screen.root(),
        //     value_list: &values,
        // });

        // match (self.conn.check_request(cookie)) {
        //     Ok(_) => println!("Successfully set keypress"),
        //     Err(e) => println!("Cannot set attributes: {:?}", e),
        // }
        loop {
            match self.conn.wait_for_event()? {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    println!("Received event: {:?}", ev);
                    if ev.detail() == 0x18 {
                        Command::new("alacritty").spawn();
                    }
                }

                xcb::Event::X(x::Event::MapRequest(ev)) => {
                    println!("Received event: {:?}", ev);
                    let cookie = self.conn.send_request_checked(&x::MapWindow {
                        window: ev.window(),
                    });
                    self.conn.check_request(cookie);
                }
                ev => {
                    println!("Receive event: {:?}", ev);
                }
            }
        }
    }
}
