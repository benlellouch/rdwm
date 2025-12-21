use std::slice::Iter;
use xcb::x::Window;

#[derive(Default, Debug)]
pub struct Workspace {
    windows: Vec<Window>,
    focus: usize,
}

impl Workspace {
    pub fn get_focused_window(&self) -> &Window {
        self.windows
            .get(self.focus)
            .expect("Focus Should never be out of bounds")
    }

    pub fn num_of_windows(&self) -> usize {
        self.windows.len()
    }

    pub fn set_focus(&mut self, idx: usize) {
        if idx >= self.windows.len() {
            return;
        }
        self.focus = idx;
    }

    pub fn get_focus(&self) -> usize {
        self.focus
    }

    pub fn push_window(&mut self, window: Window) {
        self.windows.push(window);
    }

    pub fn remove_window(&mut self, idx: usize) -> Option<Window> {
        if idx < self.num_of_windows() {
            let window = self.windows.remove(idx);
            self.update_focus();
            return Some(window);
        }
        None
    }

    fn update_focus(&mut self) {
        if self.focus >= self.num_of_windows() {
            if self.num_of_windows() == 0 {
                self.focus = 0;
            } else {
                self.focus = (self.focus - 1) % self.num_of_windows();
            }
        }
    }

    pub fn removed_focused_window(&mut self) -> Option<Window> {
        self.remove_window(self.focus)
    }

    pub fn iter_windows(&self) -> Iter<'_, Window> {
        self.windows.iter()
    }

    pub fn retain<F: FnMut(&Window) -> bool>(&mut self, f: F) {
        self.windows.retain(f)
    }
}
