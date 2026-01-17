use crate::layout::vertical_layout::VerticalLayout;

mod atoms;
mod config;
mod effect;
mod ewmh_manager;
mod key_mapping;
mod keyboard;
mod layout;
mod rdwm;
mod wm_state;
mod workspace;
mod x11;

fn main() {
    env_logger::init();

    match rdwm::WindowManager::new(VerticalLayout {}) {
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
