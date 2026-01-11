use indexmap::IndexMap;
use xcb::{Xid, x::Window};

#[derive(Debug)]
pub struct Client {
    window: Window,
    size: u32,
    is_mapped: bool,
}

impl Client {
    pub const fn window(&self) -> Window {
        self.window
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    pub fn increase_window_size(&mut self, increment: u32) {
        self.size = self.size.saturating_add(increment);
    }

    pub fn decrease_window_size(&mut self, decrement: u32) {
        self.size = self.size.saturating_sub(decrement).max(1);
    }

    pub const fn is_mapped(&self) -> bool {
        self.is_mapped
    }

    pub const fn set_mapped(&mut self, mapped: bool) {
        self.is_mapped = mapped;
    }
}

#[derive(Default, Debug)]
pub struct Workspace {
    clients: IndexMap<u32, Client>,
    focus: Option<usize>,
}

impl Workspace {
    pub fn get_focused_window(&self) -> Option<Window> {
        self.focus
            .and_then(|i| self.clients.get_index(i))
            .map(|(_key, client)| client.window())
    }

    pub fn get_focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focus
            .and_then(|i| self.clients.get_index_mut(i))
            .map(|(_key, client)| client)
    }

    pub fn get_client_mut(&mut self, win_resource_id: &u32) -> Option<&mut Client> {
        self.clients.get_mut(win_resource_id)
    }

    pub fn num_of_windows(&self) -> usize {
        self.clients.len()
    }

    pub fn set_focus(&mut self, idx: usize) -> bool {
        if idx >= self.clients.len() {
            return false;
        }
        self.focus = Some(idx);
        true
    }

    pub const fn get_focus(&self) -> Option<usize> {
        self.focus
    }

    pub fn push_window(&mut self, window: Window) {
        self.clients.insert(
            window.resource_id(),
            Client {
                window,
                size: 1,
                is_mapped: true,
            },
        );
        if self.focus.is_none() {
            self.focus = Some(self.clients.len().saturating_sub(1));
        }
    }

    pub fn remove_window_index(&mut self, idx: usize) -> Option<Window> {
        let entry = self.clients.shift_remove_index(idx);
        self.update_focus();
        entry.map(|(_key, client)| client.window)
    }

    pub fn remove_client(&mut self, win_resource_id: &u32) -> Option<Client> {
        let client = self.clients.shift_remove(win_resource_id);
        self.update_focus();
        client
    }

    fn update_focus(&mut self) {
        if self.clients.is_empty() {
            self.focus = None;
            return;
        }

        match self.focus {
            Some(f) if f < self.clients.len() => {}
            _ => self.focus = Some(self.clients.len().saturating_sub(1)),
        }
    }

    pub fn removed_focused_window(&mut self) -> Option<Window> {
        if let Some(idx) = self.focus {
            self.remove_window_index(idx)
        } else {
            None
        }
    }

    pub fn iter_windows(&self) -> impl Iterator<Item = &Window> {
        self.clients.iter().map(|(_key, client)| &client.window)
    }

    pub fn iter_clients(&self) -> impl Iterator<Item = &Client> {
        self.clients.iter().map(|(_key, client)| client)
    }

    pub fn index_of_window(&self, window: Window) -> Option<usize> {
        self.clients.get_index_of(&window.resource_id())
    }

    pub fn swap_windows(&mut self, idx_a: usize, idx_b: usize) {
        if idx_a < self.num_of_windows() && idx_b < self.num_of_windows() {
            self.clients.swap_indices(idx_a, idx_b);
        }
    }
}
