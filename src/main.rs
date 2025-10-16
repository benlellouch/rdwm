mod config;
mod key_mapping;
mod rdwm;

fn main() {
    match rdwm::WindowManager::new() {
        Ok(mut wm) => {
            if let Err(e) = wm.run() {
                eprintln!("Window manager runtime error: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize window manager: {:?}", e);
        }
    }
}
