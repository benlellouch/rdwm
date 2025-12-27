use std::slice::Iter;
use xcb::x::Window;

#[derive(Debug)]
pub struct TiledWindow {
    window: Window,
    size: u32,
}

impl TiledWindow {
    pub fn window(&self) -> Window {
        self.window
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn increase_window_size(&mut self, increment: u32) {
        self.size += increment
    }

    pub fn decrease_window_size(&mut self, increment: u32) {
        if self.size > 1 {
            self.size -= increment
        }
    }
}

#[derive(Default, Debug)]
pub struct Workspace {
    windows: Vec<TiledWindow>,
    focus: Option<usize>,
}

impl Workspace {
    pub fn get_focused_window(&self) -> Option<&Window> {
        self.focus
            .and_then(|i| self.windows.get(i))
            .map(|tw| &tw.window)
    }

    pub fn get_focused_tiled_window_mut(&mut self) -> Option<&mut TiledWindow> {
        self.focus.and_then(|i| self.windows.get_mut(i))
    }

    pub fn num_of_windows(&self) -> usize {
        self.windows.len()
    }

    pub fn set_focus(&mut self, idx: usize) -> bool {
        if idx >= self.windows.len() {
            return false;
        }
        self.focus = Some(idx);
        true
    }

    pub fn get_focus(&self) -> Option<usize> {
        self.focus
    }

    pub fn push_window(&mut self, window: Window) {
        // new windows get a default size (weight) of 1
        self.windows.push(TiledWindow { window, size: 1 });
        if self.focus.is_none() {
            self.focus = Some(self.windows.len().saturating_sub(1));
        }
    }

    pub fn remove_window(&mut self, idx: usize) -> Option<Window> {
        if idx < self.num_of_windows() {
            let tw = self.windows.remove(idx);
            let window = tw.window;
            self.update_focus();
            return Some(window);
        }
        None
    }

    fn update_focus(&mut self) {
        if self.windows.is_empty() {
            self.focus = None;
            return;
        }

        match self.focus {
            Some(f) if f < self.windows.len() => {}
            _ => self.focus = Some(self.windows.len().saturating_sub(1)),
        }
    }

    pub fn removed_focused_window(&mut self) -> Option<Window> {
        if let Some(idx) = self.focus {
            self.remove_window(idx)
        } else {
            None
        }
    }

    pub fn iter_windows(&self) -> impl Iterator<Item = &Window> {
        self.windows.iter().map(|tw| &tw.window)
    }

    pub fn iter_tiled_windows(&self) -> Iter<'_, TiledWindow> {
        self.windows.iter()
    }

    pub fn swap_windows(&mut self, idx_a: usize, idx_b: usize) {
        if idx_a < self.num_of_windows() && idx_b < self.num_of_windows() {
            self.windows.swap(idx_a, idx_b);
        }
    }

    pub fn retain<F: FnMut(&Window) -> bool>(&mut self, f: F) {
        let mut f = f;
        self.windows.retain(|tw| f(&tw.window));
        self.update_focus();
    }
}
