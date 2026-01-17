mod atoms;
mod config;
mod effect;
mod ewmh_manager;
mod key_mapping;
mod keyboard;
mod layout;
mod rdwm;
mod state;
mod workspace;
mod x11;

fn main() {
    env_logger::init();

    match rdwm::WindowManager::new() {
        Ok(mut wm) => {
            if let Err(e) = wm.run() {
                log::error!("Window manager runtime error: {e:?}");
            }
        }
        Err(e) => {
            log::error!("Failed to initialize window manager: {e:?}");
        }
    }
}
