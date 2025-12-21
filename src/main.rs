mod config;
mod key_mapping;
mod ewmh;
mod atoms;
mod rdwm;
mod workspace;

fn main() {
    env_logger::init();

    match rdwm::WindowManager::new() {
        Ok(mut wm) => {
            if let Err(e) = wm.run() {
                log::error!("Window manager runtime error: {:?}", e);
            }
        }
        Err(e) => {
            log::error!("Failed to initialize window manager: {:?}", e);
        }
    }
}
