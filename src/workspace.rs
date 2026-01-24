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
    pub fn window(&self) -> Window {
        self.window
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn increase_window_size(&mut self, increment: u32) {
        self.size = self.size.saturating_add(increment);
    }

    pub fn decrease_window_size(&mut self, decrement: u32) {
        self.size = self.size.saturating_sub(decrement).max(1);
    }

    pub fn is_mapped(&self) -> bool {
        self.is_mapped
    }

    pub fn set_mapped(&mut self, mapped: bool) {
        self.is_mapped = mapped;
    }
}

#[derive(Default, Debug)]
pub struct Workspace {
    clients: IndexMap<Window, Client>,
    focus: Option<Window>,
    fullscreen: Option<Window>,
}

impl Workspace {
    fn number_of_clients(&self) -> usize {
        self.clients.len()
    }
    pub fn get_fullscreen_window(&self) -> Option<Window> {
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
        self.update_focus()
    }

    pub fn get_focus_window(&self) -> Option<Window> {
        self.focus
    }

    pub fn get_focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focus.and_then(|win| self.clients.get_mut(&win))
    }

    pub fn get_client_mut(&mut self, window: &Window) -> Option<&mut Client> {
        self.clients.get_mut(window)
    }

    pub fn set_client_mapped(&mut self, window: &Window, mapped: bool) {
        if let Some(client) = self.clients.get_mut(window) {
            client.set_mapped(mapped);
        }
        self.update_focus();
    }

    pub fn is_window_mapped(&self, window: &Window) -> bool {
        self.clients.get(window).is_some_and(|c| c.is_mapped())
    }

    pub fn set_focus(&mut self, window: Window) -> bool {
        if self.clients.contains_key(&window) && self.is_window_mapped(&window) {
            self.focus = Some(window);
            return true;
        }
        false
    }

    pub fn push_window(&mut self, window: Window) {
        self.clients.insert(window, Client::new(window));
        if self.focus.is_none() {
            self.set_focus(window);
        }
        self.update_focus();
    }

    pub fn remove_client(&mut self, window: Window) -> Option<Client> {
        let idx_to_remove = self.index_of_window(&window);
        let client = self.clients.shift_remove(&window);
        if let Some(index) = idx_to_remove {
            let new_index = if index < self.number_of_clients() {
                index
            } else {
                self.number_of_clients().saturating_sub(1)
            };
            if let Some(window) = self.get_window_at_index(new_index) {
                self.update_focus_if_invalid(window);
            } else {
                self.update_focus();
            }
        }
        client
    }

    fn update_focus(&mut self) {
        if let Some(fs) = self.fullscreen
            && !self.set_focus(fs)
        {
            self.fullscreen = None;
        }

        if self.clients.is_empty() {
            self.focus = None;
            return;
        }

        if !self.is_focus_valid() {
            let new_focus = self
                .iter_clients()
                .find(|client| client.is_mapped())
                .map(|client| client.window());
            self.focus = new_focus;
        }
    }

    fn update_focus_if_invalid(&mut self, candidate_window: Window) {
        if !self.is_focus_valid() {
            self.set_focus(candidate_window);
        }

        self.update_focus();
    }

    fn is_focus_valid(&self) -> bool {
        self.focus
            .map(|win| self.clients.contains_key(&win))
            .unwrap_or(true)
    }

    pub fn removed_focused_window(&mut self) -> Option<Window> {
        if let Some(window) = self.focus {
            self.remove_client(window).map(|client| client.window())
        } else {
            None
        }
    }

    pub fn iter_windows(&self) -> impl Iterator<Item = &Window> {
        self.clients.keys()
    }

    pub fn iter_clients(&self) -> impl Iterator<Item = &Client> {
        self.clients.values()
    }

    pub fn index_of_window(&self, window: &Window) -> Option<usize> {
        self.clients.get_index_of(window)
    }

    fn get_window_at_index(&self, index: usize) -> Option<Window> {
        self.clients.get_index(index).map(|(window, _)| *window)
    }

    fn next_index(index: isize, direction: isize, length: isize) -> usize {
        (index + direction).rem_euclid(length) as usize
    }

    pub fn next_mapped_window(&self, direction: isize) -> Option<Window> {
        if let Some(window) = self.focus
            && let Some(index) = self.index_of_window(&window)
        {
            let mut next_index =
                Self::next_index(index as isize, direction, self.clients.len() as isize);
            while next_index != index {
                if let Some((next_window, next_client)) = self.clients.get_index(next_index)
                    && next_client.is_mapped()
                {
                    return Some(*next_window);
                }
                next_index =
                    Self::next_index(next_index as isize, direction, self.clients.len() as isize);
            }
        }
        None
    }

    pub fn swap_windows(&mut self, window_a: &Window, window_b: &Window) {
        if let Some(idx_a) = self.index_of_window(window_a)
            && let Some(idx_b) = self.index_of_window(window_b)
        {
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

    fn make_workspace(num_of_clients: u32) -> Workspace {
        let mut workspace = Workspace::default();
        for i in 0..num_of_clients {
            let window = Window::new(i);
            workspace.push_window(window);
        }
        workspace
    }

    #[test]
    fn test_fullscreen_empty_workspace() {
        let mut workspace = Workspace::default();
        workspace.set_fullscreen(Window::new(0));
        assert!(workspace.get_fullscreen_window().is_none());
        assert!(workspace.get_focus_window().is_none());
    }

    #[test]
    fn test_set_fullscreen_window() {
        let mut workspace = make_workspace(5);
        let window = Window::new(2);
        workspace.set_fullscreen(window);

        assert_eq!(workspace.get_fullscreen_window(), Some(window));
        assert_eq!(workspace.get_focus_window(), Some(window));
    }

    #[test]
    fn test_clear_fullscreen() {
        let mut workspace = make_workspace(5);
        let window = Window::new(2);
        workspace.set_fullscreen(window);
        workspace.clear_fullscreen();

        assert!(workspace.get_fullscreen_window().is_none());
        assert_eq!(workspace.get_focus_window(), Some(window));
    }

    #[test]
    fn test_remove_fullscreen() {
        let mut workspace = make_workspace(5);
        let fullscreen_window = Window::new(2);
        let expected_next_focus = Window::new(3);

        workspace.set_fullscreen(fullscreen_window);
        let client = workspace.remove_client(fullscreen_window);

        assert!(client.is_some());
        assert!(workspace.get_fullscreen_window().is_none());
        assert_eq!(workspace.get_focus_window(), Some(expected_next_focus));
    }

    #[test]
    fn test_remove_only_client() {
        let mut workspace = make_workspace(1);
        let window_to_remove = Window::new(0);
        let client = workspace.remove_client(window_to_remove);

        assert!(client.is_some());
        assert!(workspace.get_focus_window().is_none());
    }

    #[test]
    fn test_remove_non_managed_client() {
        let mut workspace = make_workspace(5);
        let window_to_remove = Window::new(6);
        let client = workspace.remove_client(window_to_remove);
        assert!(client.is_none());
    }

    #[test]
    fn test_remove_first_client() {
        let mut workspace = make_workspace(5);
        assert_eq!(
            workspace.get_focus_window(),
            workspace.get_window_at_index(0)
        );
        workspace.removed_focused_window();
        assert_eq!(
            workspace.get_focus_window(),
            workspace.get_window_at_index(0)
        )
    }

    #[test]
    fn test_remove_last_client() {
        let mut workspace = make_workspace(5);
        workspace.set_focus(Window::new(4));
        workspace.removed_focused_window();
        assert_eq!(workspace.get_focus_window(), Some(Window::new(3)));
    }

    #[test]
    fn test_push_window_sets_focus_when_none() {
        let mut workspace = Workspace::default();
        let window = Window::new(10);

        workspace.push_window(window);

        assert_eq!(workspace.get_focus_window(), Some(window));
    }

    #[test]
    fn test_set_focus_rejects_invalid_or_unmapped() {
        let mut workspace = Workspace::default();
        let window_a = Window::new(1);
        let window_b = Window::new(2);

        workspace.push_window(window_a);
        workspace.push_window(window_b);

        workspace.set_client_mapped(&window_b, false);

        assert!(!workspace.set_focus(Window::new(99)));
        assert!(!workspace.set_focus(window_b));
        assert_eq!(workspace.get_focus_window(), Some(window_a));
    }

    #[test]
    fn test_next_window_wraps() {
        let workspace = make_workspace(3);

        assert_eq!(workspace.get_focus_window(), Some(Window::new(0)));
        assert_eq!(workspace.next_mapped_window(1), Some(Window::new(1)));
        assert_eq!(workspace.next_mapped_window(-1), Some(Window::new(2)));
    }

    #[test]
    fn test_swap_windows_changes_order() {
        let mut workspace = make_workspace(3);
        let window_a = Window::new(0);
        let window_b = Window::new(2);

        workspace.swap_windows(&window_a, &window_b);

        let windows: Vec<Window> = workspace.iter_windows().copied().collect();
        assert_eq!(windows, vec![window_b, Window::new(1), window_a]);
    }
}
