mod rdwm;
mod config;
mod key_mapping;

fn main() {
    let mut wm = rdwm::WindowManager::new();
    if let Err(e) = wm.run() {
        eprintln!("Window manager error: {:?}", e);
    }
}
