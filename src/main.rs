mod rdwm;

fn main() {
    let wm = rdwm::WindowManager::new();
    wm.run();
}
