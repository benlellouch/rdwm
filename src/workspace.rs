use indexmap::IndexMap;
use xcb::x::Window;

#[derive(Debug)]
pub struct Client {
    window: Window,
    size: u32,
    is_mapped: bool,
}

impl Client {
    pub fn new(window: Window) -> Self {
        Client {
            window,
            size: 1,
            is_mapped: true,
        }
    }
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
    clients: IndexMap<Window, Client>,
    focus: Option<usize>,
    fullscreen: Option<Window>,
}

impl Workspace {
    pub const fn fullscreen_window(&self) -> Option<Window> {
        self.fullscreen
    }

    pub fn set_fullscreen(&mut self, window: Window) {
        if self.clients.contains_key(&window) {
            self.fullscreen = Some(window);
            self.update_focus();
        }
    }

    pub fn clear_fullscreen(&mut self) {
        self.fullscreen = None;
        self.update_focus();
    }

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

    pub fn get_client_mut(&mut self, window: Window) -> Option<&mut Client> {
        self.clients.get_mut(&window)
    }

    pub fn set_client_mapped(&mut self, window: Window, mapped: bool) {
        if let Some(client) = self.clients.get_mut(&window) {
            client.set_mapped(mapped);
        }
        self.update_focus();
    }

    pub fn is_window_mapped(&self, window: Window) -> bool {
        self.clients.get(&window).is_some_and(|c| c.is_mapped())
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
        self.clients.insert(window, Client::new(window));
        if self.focus.is_none() {
            self.focus = Some(self.clients.len().saturating_sub(1));
        }

        self.update_focus();
    }

    pub fn remove_window_index(&mut self, idx: usize) -> Option<Window> {
        let entry = self.clients.shift_remove_index(idx);
        self.update_focus();
        entry.map(|(_key, client)| client.window)
    }

    pub fn remove_client(&mut self, window: Window) -> Option<Client> {
        let client = self.clients.shift_remove(&window);
        self.update_focus();
        client
    }

    fn update_focus(&mut self) {
        if let Some(fs) = self.fullscreen {
            if let Some(idx) = self.index_of_window(fs) {
                self.focus = Some(idx);
                return;
            }
            // Fullscreen window disappeared.
            self.fullscreen = None;
        }

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
        self.clients.get_index_of(&window)
    }

    pub fn swap_windows(&mut self, idx_a: usize, idx_b: usize) {
        if idx_a < self.num_of_windows() && idx_b < self.num_of_windows() {
            self.clients.swap_indices(idx_a, idx_b);
        }
    }
}

#[cfg(test)]
mod client_tests {
    use xcb::XidNew;

    use super::*;

    #[test]
    fn test_weight_at_min_bound() {
        let window = Window::new(0);
        let mut client = Client::new(window);

        client.decrease_window_size(2);
        assert_eq!(client.size(), 1);
    }

    #[test]
    fn test_decrease_weight() {
        let window = Window::new(0);
        let mut client = Client {
            window,
            size: 5,
            is_mapped: true,
        };

        client.decrease_window_size(2);
        assert_eq!(client.size(), 3);
    }

    #[test]
    fn test_increase_weight() {
        let window = Window::new(0);
        let mut client = Client::new(window);

        client.increase_window_size(1);
        assert_eq!(client.size(), 2);
    }
}

#[cfg(test)]
mod workspace_tests {
    use xcb::XidNew;

    use super::*;

    #[test]
    fn test_fullscreen_empty_workspace() {
        let mut workspace = Workspace::default();
        workspace.set_fullscreen(Window::new(0));
        assert!(workspace.fullscreen_window().is_none());
        assert!(workspace.get_focused_window().is_none());
    }

    fn make_workspace(num_of_clients: u32) -> Workspace {
        let mut workspace = Workspace::default();
        for i in 0..num_of_clients {
            let window = Window::new(i);
            workspace.push_window(window);
        }
        workspace
    }

    #[test]
    fn test_set_fullscreen_window() {
        let mut workspace = make_workspace(5);
        let window = Window::new(2);
        workspace.set_fullscreen(window);

        assert_eq!(workspace.fullscreen_window(), Some(window));
        assert_eq!(workspace.get_focused_window(), Some(window));
    }

    #[test]
    fn test_clear_fullscreen() {
        let mut workspace = make_workspace(5);
        let window = Window::new(2);
        workspace.set_fullscreen(window);
        workspace.clear_fullscreen();

        assert!(workspace.fullscreen_window().is_none());
        assert_eq!(workspace.get_focused_window(), Some(window));
    }

    #[test]
    fn test_remove_fullscreen() {
        let mut workspace = make_workspace(5);
        let fullscreen_window = Window::new(2);
        let expected_next_focus = Window::new(3);

        workspace.set_fullscreen(fullscreen_window);
        let client = workspace.remove_client(fullscreen_window);

        assert!(client.is_some());
        assert!(workspace.fullscreen_window().is_none());
        assert_eq!(workspace.get_focused_window(), Some(expected_next_focus));
    }

    #[test]
    fn test_remove_last_client() {
        let mut workspace = make_workspace(1);
        let window_to_remove = Window::new(0);
        let client = workspace.remove_client(window_to_remove);

        assert!(client.is_some());
        assert!(workspace.get_focused_window().is_none());
    }

    #[test]
    fn test_remove_non_managed_client() {
        let mut workspace = make_workspace(5);
        let window_to_remove = Window::new(6);
        let client = workspace.remove_client(window_to_remove);
        assert!(client.is_none());
    }
}
