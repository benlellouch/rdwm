use std::collections::HashMap;

use log::warn;
use xcb::{Xid, x::Window};

use crate::{
    config::NUM_WORKSPACES,
    effect::Effect,
    key_mapping::ActionEvent,
    layout::{LayoutManager, Rect},
    workspace::Workspace,
    x11::WindowType,
};

#[derive(Clone, Copy, Debug)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub focused_border_pixel: u32,
    pub normal_border_pixel: u32,
}

pub struct State {
    layout_manager: LayoutManager,

    workspaces: [Workspace; NUM_WORKSPACES],
    window_to_workspace: HashMap<Window, usize>,
    current_workspace: usize,

    screen: ScreenConfig,
    border_width: u32,
    window_gap: u32,

    dock_windows: Vec<Window>,
    dock_height: u32,
}

impl State {
    pub fn new(screen: ScreenConfig, border_width: u32, window_gap: u32, dock_height: u32) -> Self {
        Self {
            layout_manager: LayoutManager::new(),
            workspaces: Default::default(),
            window_to_workspace: Default::default(),
            current_workspace: 0,
            screen,
            border_width,
            window_gap,
            dock_windows: Vec::new(),
            dock_height,
        }
    }

    pub const fn screen(&self) -> ScreenConfig {
        self.screen
    }

    pub const fn current_workspace_id(&self) -> usize {
        self.current_workspace
    }

    pub fn focused_window(&self) -> Option<Window> {
        self.current_workspace().get_focus_window()
    }

    pub fn usable_screen_height(&self) -> u32 {
        if !self.dock_windows.is_empty() {
            return self.screen.height.saturating_sub(self.dock_height);
        }
        self.screen.height
    }

    pub fn window_workspace(&self, window: Window) -> Option<usize> {
        self.window_to_workspace.get(&window).copied()
    }

    pub fn is_window_fullscreen(&self, window: Window) -> bool {
        self.window_workspace(window)
            .and_then(|workspace_id| self.get_workspace(workspace_id))
            .and_then(|workspace| workspace.get_fullscreen_window())
            .map(|fullscreen| window == fullscreen)
            .unwrap_or(false)
    }
    pub fn managed_windows_sorted(&self) -> Vec<Window> {
        let mut entries = self
            .window_to_workspace
            .iter()
            .map(|(w, ws)| (*ws, w.resource_id(), *w))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(ws, id, _w)| (*ws, *id));
        entries.into_iter().map(|(_ws, _id, w)| w).collect()
    }

    pub fn client_list_windows(&self) -> Vec<Window> {
        let mut out = self.managed_windows_sorted();

        let mut docks = self.dock_windows.clone();
        docks.sort_by_key(xcb::Xid::resource_id);
        out.extend(docks);

        out
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace {
        self.workspaces
            .get_mut(self.current_workspace)
            .expect("Workspace should never be out of bounds")
    }

    fn current_workspace(&self) -> &Workspace {
        self.workspaces
            .get(self.current_workspace)
            .expect("Workspace should never be out of bounds")
    }

    fn get_workspace(&self, workspace_id: usize) -> Option<&Workspace> {
        self.workspaces.get(workspace_id)
    }

    fn get_workspace_mut(&mut self, workspace_id: usize) -> Option<&mut Workspace> {
        self.workspaces.get_mut(workspace_id)
    }

    fn tracked_window_type(&self, window: Window) -> WindowType {
        let window_id = window.resource_id();

        if self
            .dock_windows
            .iter()
            .any(|w| w.resource_id() == window_id)
        {
            return WindowType::Dock;
        }

        if self.window_workspace(window).is_some() {
            return WindowType::Managed;
        }

        WindowType::Unmanaged
    }

    fn cycle_layout(&mut self) -> Vec<Effect> {
        self.layout_manager.cycle_layout();
        self.configure_windows(self.current_workspace)
    }

    pub fn configure_windows(&self, workspace_id: usize) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];
        if let Some(current_workspace) = self.get_workspace(workspace_id) {
            if let Some(fullscreen) = current_workspace.get_fullscreen_window()
                && current_workspace.is_window_mapped(&fullscreen)
            {
                effects.push(Effect::Configure {
                    window: fullscreen,
                    x: 0,
                    y: 0,
                    w: self.screen.width,
                    h: self.screen.height,
                    border: 0,
                });
                effects.push(Effect::Raise(fullscreen));
                return effects;
            }

            let clients: Vec<_> = current_workspace
                .iter_clients()
                .filter(|client| client.is_mapped())
                .collect();
            if clients.is_empty() {
                return effects;
            }

            let weights: Vec<u32> = clients.iter().map(|client| client.size()).collect();
            let area = Rect {
                x: 0,
                y: 0,
                w: self.screen.width,
                h: self.usable_screen_height(),
            };
            let layout = self.layout_manager.get_current_layout().generate_layout(
                area,
                &weights,
                self.border_width,
                self.window_gap,
            );

            effects = clients
                .iter()
                .zip(layout)
                .map(|(client, rect)| Effect::Configure {
                    window: client.window(),
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                    border: self.border_width,
                })
                .collect();
        }

        effects
    }

    pub fn configure_dock_windows(&self) -> Vec<Effect> {
        let mut effects = Vec::with_capacity(self.dock_windows.len());
        let dock_y = (self.screen.height as i32) - (self.dock_height as i32);

        for &window in &self.dock_windows {
            effects.push(Effect::ConfigurePositionSize {
                window,
                x: 0,
                y: dock_y,
                w: self.screen.width,
                h: self.dock_height,
            });
        }

        effects
    }

    pub fn set_focus(&mut self, window: Window) -> Vec<Effect> {
        if let Some(fs) = self.current_workspace().get_fullscreen_window()
            && self.current_workspace().is_window_mapped(&fs)
        {
            return vec![];
        }

        let mut effects = Vec::new();

        let fullscreen_window = self.current_workspace().get_fullscreen_window();
        let previous_focus = self.current_workspace().get_focus_window();
        if self.current_workspace_mut().set_focus(window) {
            if let Some(previous_window) = previous_focus {
                effects.push(Effect::SetBorder {
                    window: previous_window,
                    pixel: self.screen.normal_border_pixel,
                    width: if fullscreen_window == Some(previous_window) {
                        0
                    } else {
                        self.border_width
                    },
                });
            }

            effects.push(Effect::SetBorder {
                window,
                pixel: self.screen.focused_border_pixel,
                width: if fullscreen_window == Some(window) {
                    0
                } else {
                    self.border_width
                },
            });
            effects.push(Effect::Focus(window));
            if fullscreen_window == Some(window) {
                effects.push(Effect::Raise(window));
            }
        }
        effects
    }

    pub fn toggle_fullscreen(&mut self) -> Vec<Effect> {
        let Some(focused) = self.current_workspace().get_focus_window() else {
            return vec![];
        };

        let prev_fullscreen = self.current_workspace().get_fullscreen_window();
        let toggle_off = prev_fullscreen == Some(focused);

        let mut effects = Vec::new();

        if toggle_off {
            self.current_workspace_mut().clear_fullscreen();
        } else {
            self.current_workspace_mut().set_fullscreen(focused);
            effects.push(Effect::Raise(focused));
        }

        effects.extend(self.configure_windows(self.current_workspace));
        effects.extend(self.set_focus(focused));
        effects
    }

    pub fn focus_window(&mut self, window: Window, desktop_hint: Option<usize>) -> Vec<Effect> {
        let mut effects = Vec::new();

        let workspace_id = self.window_workspace(window).or(desktop_hint);

        if self.current_workspace().get_fullscreen_window().is_some() {
            return effects;
        } //We don't want our focus to be stolen if we are fullscreen

        let Some(workspace_id) = workspace_id else {
            return effects;
        };

        if workspace_id < NUM_WORKSPACES && workspace_id != self.current_workspace {
            effects.extend(self.go_to_workspace(workspace_id));
        }

        effects.extend(self.set_focus(window));

        effects
    }

    pub fn go_to_workspace(&mut self, new_workspace_id: usize) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];

        if self.current_workspace == new_workspace_id || new_workspace_id >= NUM_WORKSPACES {
            return effects;
        }

        let old_workspace_id = self.current_workspace;
        let old_windows: Vec<Window> = self
            .workspaces
            .get(old_workspace_id)
            .expect("Workspace should never be out of bounds")
            .iter_windows()
            .copied()
            .collect();

        {
            let old_ws = self
                .workspaces
                .get_mut(old_workspace_id)
                .expect("Workspace should never be out of bounds");
            for &win in &old_windows {
                old_ws.set_client_mapped(&win, false);
            }
        }

        for win in old_windows {
            effects.push(Effect::Unmap(win));
        }

        self.current_workspace = new_workspace_id;

        let new_windows: Vec<Window> = self.current_workspace().iter_windows().copied().collect();

        {
            let new_ws = self.current_workspace_mut();
            for win in &new_windows {
                new_ws.set_client_mapped(win, true);
            }
        }

        for win in new_windows {
            effects.push(Effect::Map(win));
        }

        effects.extend(self.configure_windows(self.current_workspace));
        if let Some(focus) = self.current_workspace().get_focus_window() {
            effects.extend(self.set_focus(focus));
        }

        effects
    }

    pub fn send_to_workspace(&mut self, workspace_id: usize) -> Vec<Effect> {
        let mut effects = Vec::new();
        if workspace_id >= NUM_WORKSPACES || workspace_id == self.current_workspace_id() {
            return effects;
        }

        if let Some(window_to_send) = self.current_workspace_mut().removed_focused_window()
            && let Some(new_workspace) = self.workspaces.get_mut(workspace_id)
        {
            new_workspace.push_window(window_to_send);
            new_workspace.set_client_mapped(&window_to_send, false);
            self.window_to_workspace
                .insert(window_to_send, workspace_id);

            effects.push(Effect::Unmap(window_to_send));
            effects.push(Effect::SetBorder {
                window: window_to_send,
                pixel: self.screen.normal_border_pixel,
                width: self.border_width,
            });

            effects.extend(self.configure_windows(self.current_workspace));

            if let Some(focus) = self.current_workspace().get_focus_window() {
                effects.extend(self.set_focus(focus));
            }
        }

        effects
    }

    pub fn increase_window_weight(&mut self, increment: u32) -> Vec<Effect> {
        if let Some(focused_win) = self.current_workspace_mut().get_focused_client_mut() {
            focused_win.increase_window_size(increment);
            return self.configure_windows(self.current_workspace);
        }

        vec![]
    }

    pub fn decrease_window_weight(&mut self, increment: u32) -> Vec<Effect> {
        if let Some(focused_win) = self.current_workspace_mut().get_focused_client_mut() {
            focused_win.decrease_window_size(increment);
            return self.configure_windows(self.current_workspace);
        }
        vec![]
    }

    pub fn increase_window_gap(&mut self, increment: u32) -> Vec<Effect> {
        self.window_gap += increment;
        self.configure_windows(self.current_workspace)
    }

    pub fn decrease_window_gap(&mut self, decrement: u32) -> Vec<Effect> {
        let new_gap = self.window_gap.saturating_sub(decrement);

        if new_gap == self.window_gap {
            return vec![];
        }

        self.window_gap = new_gap;
        self.configure_windows(self.current_workspace)
    }

    pub fn shift_focus(&mut self, direction: isize) -> Vec<Effect> {
        let Some(next_focus) = self.current_workspace().next_mapped_window(direction) else {
            warn!("Failed to retrieve next focus");
            return vec![];
        };

        self.set_focus(next_focus)
    }

    pub fn swap_window(&mut self, direction: isize) -> Vec<Effect> {
        let current_workspace = self.current_workspace_mut();
        if current_workspace.get_fullscreen_window().is_some() {
            return vec![];
        }
        let Some(next_window) = current_workspace.next_mapped_window(direction) else {
            return vec![];
        };

        let Some(focus) = current_workspace.get_focus_window() else {
            return vec![];
        };

        current_workspace.swap_windows(&focus, &next_window);

        let mut effects = vec![];
        effects.extend(self.configure_windows(self.current_workspace));
        effects
    }

    pub fn on_map_request(&mut self, window: Window, window_type: WindowType) -> Vec<Effect> {
        match window_type {
            WindowType::Unmanaged => vec![Effect::Map(window)],
            WindowType::Dock => self.handle_map_request_dock(window),
            WindowType::Managed => self.handle_map_request_managed(window),
        }
    }

    fn handle_map_request_dock(&mut self, window: Window) -> Vec<Effect> {
        let mut effects = Vec::new();

        if !self
            .dock_windows
            .iter()
            .any(|w| w.resource_id() == window.resource_id())
        {
            self.dock_windows.push(window);
        }

        effects.push(Effect::Map(window));
        effects.extend(self.configure_dock_windows());
        effects.extend(self.configure_windows(self.current_workspace));
        effects
    }

    fn handle_map_request_managed(&mut self, window: Window) -> Vec<Effect> {
        let mut effects = Vec::new();

        match self.current_workspace_mut().get_client_mut(&window) {
            Some(client) => {
                client.set_mapped(true);
            }
            None => {
                self.current_workspace_mut().push_window(window);
                self.window_to_workspace
                    .insert(window, self.current_workspace);
            }
        }

        effects.push(Effect::Map(window));

        if let Some(fs) = self.current_workspace().get_fullscreen_window()
            && self.current_workspace().is_window_mapped(&fs)
        {
            effects.extend(self.configure_windows(self.current_workspace));
        } else {
            effects.extend(self.set_focus(window));
            effects.extend(self.configure_windows(self.current_workspace));
        }

        effects
    }

    pub fn on_destroy(&mut self, window: Window) -> Vec<Effect> {
        match self.tracked_window_type(window) {
            WindowType::Dock => self.handle_destroy_event_dock(window),
            WindowType::Managed => self.handle_destroy_event_managed(window),
            WindowType::Unmanaged => vec![],
        }
    }

    fn handle_destroy_event_dock(&mut self, window: Window) -> Vec<Effect> {
        let window_id = window.resource_id();
        self.dock_windows.retain(|w| w.resource_id() != window_id);

        let mut effects = Vec::new();
        if !self.dock_windows.is_empty() {
            effects.extend(self.configure_dock_windows());
        }

        effects.extend(self.configure_windows(self.current_workspace));
        effects
    }

    fn handle_destroy_event_managed(&mut self, window: Window) -> Vec<Effect> {
        if let Some(workspace_id) = self.window_to_workspace.remove(&window)
            && let Some(current_workspace) = self.workspaces.get_mut(workspace_id)
        {
            current_workspace.remove_client(window);
        }

        let mut effects = Vec::new();
        effects.extend(self.configure_windows(self.current_workspace));
        if let Some(focus) = self.current_workspace().get_focus_window() {
            effects.extend(self.set_focus(focus));
        }
        effects
    }

    pub fn on_unmap(&mut self, window: Window) -> Vec<Effect> {
        match self.tracked_window_type(window) {
            WindowType::Dock => vec![],
            WindowType::Managed => self.handle_unmap_event_managed(window),
            WindowType::Unmanaged => vec![],
        }
    }

    fn handle_unmap_event_managed(&mut self, window: Window) -> Vec<Effect> {
        let Some(workspace_id) = self.window_workspace(window) else {
            return vec![];
        };

        let mut changed = false;
        if let Some(workspace) = self.workspaces.get_mut(workspace_id)
            && let Some(client) = workspace.get_client_mut(&window)
            && client.is_mapped()
        {
            workspace.set_client_mapped(&window, false);
            changed = true;
        }

        if workspace_id != self.current_workspace {
            return vec![];
        }

        if !changed {
            return vec![];
        }

        let mut effects = Vec::new();
        effects.extend(self.configure_windows(self.current_workspace));
        effects
    }

    pub fn apply_action(&mut self, action: ActionEvent) -> Vec<Effect> {
        match action {
            ActionEvent::NextWindow => self.shift_focus(1),
            ActionEvent::PrevWindow => self.shift_focus(-1),
            ActionEvent::IncreaseWindowWeight(increment) => self.increase_window_weight(increment),
            ActionEvent::DecreaseWindowWeight(increment) => self.decrease_window_weight(increment),
            ActionEvent::SwapLeft => self.swap_window(-1),
            ActionEvent::SwapRight => self.swap_window(1),
            ActionEvent::GoToWorkspace(workspace_id) => self.go_to_workspace(workspace_id),
            ActionEvent::SendToWorkspace(workspace_id) => self.send_to_workspace(workspace_id),
            ActionEvent::IncreaseWindowGap(increment) => self.increase_window_gap(increment),
            ActionEvent::DecreaseWindowGap(increment) => self.decrease_window_gap(increment),
            ActionEvent::ToggleFullscreen => self.toggle_fullscreen(),
            ActionEvent::CycleLayout => self.cycle_layout(),
            _ => vec![],
        }
    }

    pub fn track_startup_dock(&mut self, window: Window) {
        if !self
            .dock_windows
            .iter()
            .any(|w| w.resource_id() == window.resource_id())
        {
            self.dock_windows.push(window);
        }
    }

    pub fn track_startup_managed(&mut self, window: Window, workspace_id: usize) {
        if let Some(ws) = self.get_workspace_mut(workspace_id) {
            ws.push_window(window);
            self.window_to_workspace.insert(window, workspace_id);
        }
    }

    pub fn startup_finalize(&mut self, current_desktop: Option<usize>) -> Vec<Effect> {
        let mut effects = Vec::new();

        if !self.dock_windows.is_empty() {
            effects.extend(self.configure_dock_windows());
        }

        if let Some(workspace_id) = current_desktop {
            self.current_workspace = (workspace_id + 1) % NUM_WORKSPACES;
            effects.extend(self.go_to_workspace(workspace_id));
            return effects;
        }

        effects
    }
}

#[cfg(test)]
mod state_tests {

    use std::any::{Any, TypeId};

    use xcb::XidNew;

    use super::*;

    fn make_state_with_windows(windows: &[(usize, u32, bool)], dock_height: u32) -> State {
        let screen = ScreenConfig {
            width: 800,
            height: 600,
            focused_border_pixel: 0,
            normal_border_pixel: 1,
        };

        let mut state = State::new(screen, 1, 0, dock_height);

        for (workspace_id, window_id, mapped) in windows {
            let window = Window::new(*window_id);
            state.track_startup_managed(window, *workspace_id);
            if !*mapped {
                let workspace = state.get_workspace_mut(*workspace_id).unwrap();
                workspace.set_client_mapped(&window, false);
            }
        }

        state
    }

    fn find_configure_height(effects: &[Effect], window: Window) -> Option<u32> {
        effects.iter().find_map(|effect| match effect {
            Effect::Configure { window: w, h, .. } if *w == window => Some(*h),
            _ => None,
        })
    }

    fn make_state(num_of_clients_per_workspace: u32) -> State {
        let screen = ScreenConfig {
            width: 800,
            height: 600,
            focused_border_pixel: 0,
            normal_border_pixel: 1,
        };
        let mut state = State::new(screen, 1, 0, 25);
        for i in 0..(num_of_clients_per_workspace * NUM_WORKSPACES as u32) {
            let workspace_id: usize = (i as usize) / NUM_WORKSPACES;
            let window = Window::new(i);
            state.track_startup_managed(window, workspace_id);
            if workspace_id > 0 {
                let workspace = state.get_workspace_mut(workspace_id).unwrap();
                workspace.set_client_mapped(&window, false);
            }
        }

        state
    }

    #[test]
    fn test_set_focus() {
        let mut state = make_state(10);
        let window_to_focus = Window::new(6);
        let effects = state.set_focus(window_to_focus);

        assert_eq!(state.focused_window().unwrap(), window_to_focus);
        assert!(effects.contains(&Effect::SetBorder {
            window: Window::new(0),
            pixel: state.screen.normal_border_pixel,
            width: state.border_width
        }));
        assert!(effects.contains(&Effect::SetBorder {
            window: window_to_focus,
            pixel: state.screen.focused_border_pixel,
            width: state.border_width
        }));
        assert!(effects.contains(&Effect::Focus(window_to_focus)));
    }

    #[test]
    fn test_toggle_fullscreen() {
        let mut state = make_state(10);
        let window_to_fullsreen = Window::new(6);
        let _ = state.set_focus(window_to_fullsreen);
        let mut fullscreen_effects = state.toggle_fullscreen();

        // Test that we succesfully toggled window to fullscreen
        assert_eq!(state.focused_window().unwrap(), window_to_fullsreen);
        assert_eq!(
            state.current_workspace().get_fullscreen_window().unwrap(),
            window_to_fullsreen
        );
        assert!(state.is_window_fullscreen(window_to_fullsreen));
        assert!(fullscreen_effects.contains(&Effect::Raise(window_to_fullsreen)));
        assert!(fullscreen_effects.contains(&Effect::Configure {
            window: window_to_fullsreen,
            x: 0,
            y: 0,
            w: 800,
            h: 600,
            border: 0
        }));

        fullscreen_effects = state.toggle_fullscreen();

        assert_eq!(state.focused_window().unwrap(), window_to_fullsreen);
        assert_eq!(state.current_workspace().get_fullscreen_window(), None);
        assert!(!state.is_window_fullscreen(window_to_fullsreen));
        assert!(fullscreen_effects.contains(&Effect::Focus(window_to_fullsreen)))
    }

    #[test]
    fn test_toggle_fullscreen_and_switch_focus() {
        let mut state = make_state(10);
        let window_to_fullsreen = Window::new(6);
        let window_to_focus = Window::new(2);
        let _ = state.set_focus(window_to_fullsreen);
        let _fullscreen_effects = state.toggle_fullscreen();
        let focus_effects = state.set_focus(window_to_focus);
        // We assert that our focus has not been stolen
        assert!(focus_effects.is_empty());
    }

    #[test]
    fn test_toggle_fullscreen_and_kill_window() {
        let mut state = make_state(10);
        let window_to_fullsreen = Window::new(6);
        let expected_focus = Window::new(7);
        let _ = state.set_focus(window_to_fullsreen);
        let _fullscreen_effects = state.toggle_fullscreen();
        let destroy_effects = state.on_destroy(window_to_fullsreen);

        assert!(!state.is_window_fullscreen(window_to_fullsreen));
        assert_eq!(state.focused_window().unwrap(), expected_focus);
        assert!(destroy_effects.contains(&Effect::Focus(expected_focus)));
        assert_eq!(
            destroy_effects
                .iter()
                .filter(|effect| matches!(
                    effect,
                    Effect::Configure {
                        window: _,
                        x: _,
                        y: _,
                        w: _,
                        h: _,
                        border: _
                    }
                ))
                .collect::<Vec<&Effect>>()
                .len(),
            9
        )
    }

    #[test]
    fn test_toggle_fullscreen_and_send_to_workspace() {
        let mut state = make_state(10);
        let window_to_fullsreen = Window::new(6);
        let expected_focus = Window::new(7);
        let _ = state.set_focus(window_to_fullsreen);
        let _fullscreen_effects = state.toggle_fullscreen();
        let workspace_effects = state.send_to_workspace(1);

        assert!(!state.is_window_fullscreen(window_to_fullsreen));
        assert_eq!(state.window_workspace(window_to_fullsreen).unwrap(), 1);
        assert!(
            state
                .get_workspace(0)
                .unwrap()
                .index_of_window(&window_to_fullsreen)
                .is_none()
        );
        assert!(workspace_effects.contains(&Effect::Unmap(window_to_fullsreen)));
        assert!(workspace_effects.contains(&Effect::Focus(expected_focus)));
        assert_eq!(
            workspace_effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .collect::<Vec<&Effect>>()
                .len(),
            9
        )
    }

    #[test]
    fn test_toggle_fullscreen_and_go_to_workspace() {
        let mut state = make_state(10);
        let window_to_fullsreen = Window::new(6);
        let _ = state.set_focus(window_to_fullsreen);
        let _fullscreen_effects = state.toggle_fullscreen();
        let workspace_effects = state.go_to_workspace(1);

        assert!(!state.is_window_fullscreen(window_to_fullsreen));
        assert_eq!(state.current_workspace_id(), 1);
        assert_eq!(
            workspace_effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .collect::<Vec<&Effect>>()
                .len(),
            10
        );
        assert_eq!(
            workspace_effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Unmap(_)))
                .collect::<Vec<&Effect>>()
                .len(),
            10
        );
        assert_eq!(
            workspace_effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Map(_)))
                .collect::<Vec<&Effect>>()
                .len(),
            10
        )
    }

    #[test]
    fn test_fullscreen_then_map_request_does_not_steal_focus() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let fullscreen_window = Window::new(1);
        let _ = state.set_focus(fullscreen_window);
        let _ = state.toggle_fullscreen();

        let new_window = Window::new(2);
        let effects = state.on_map_request(new_window, WindowType::Managed);

        assert_eq!(state.focused_window(), Some(fullscreen_window));
        assert!(state.is_window_fullscreen(fullscreen_window));
        assert!(effects.contains(&Effect::Map(new_window)));
        assert!(!effects.contains(&Effect::Focus(new_window)));
        assert!(state.current_workspace().is_window_mapped(&new_window));
    }

    #[test]
    fn test_unmap_current_workspace_window_reconfigures() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, true)], 25);
        let focus_window = Window::new(1);
        let other_window = Window::new(2);

        let _ = state.set_focus(focus_window);
        let effects = state.on_unmap(other_window);

        assert_eq!(state.focused_window(), Some(focus_window));
        assert!(!state.current_workspace().is_window_mapped(&other_window));
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .collect::<Vec<&Effect>>()
                .len(),
            1
        );
    }

    #[test]
    fn test_dock_reduces_configured_height() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let window = Window::new(1);

        let effects_no_dock = state.configure_windows(0);
        let height_no_dock = find_configure_height(&effects_no_dock, window).unwrap();

        state.track_startup_dock(Window::new(99));
        let effects_with_dock = state.configure_windows(0);
        let height_with_dock = find_configure_height(&effects_with_dock, window).unwrap();

        assert_eq!(height_no_dock, 598);
        assert_eq!(height_with_dock, 573);
        assert!(height_with_dock < height_no_dock);
    }

    #[test]
    fn test_managed_windows_sorted_by_workspace_then_id() {
        let state = make_state_with_windows(&[(1, 3, false), (0, 2, true), (0, 1, true)], 25);
        // Ensure all are tracked
        assert_eq!(state.window_workspace(Window::new(1)), Some(0));
        assert_eq!(state.window_workspace(Window::new(2)), Some(0));
        assert_eq!(state.window_workspace(Window::new(3)), Some(1));

        let sorted = state.managed_windows_sorted();
        assert_eq!(sorted, vec![Window::new(1), Window::new(2), Window::new(3)]);
    }

    #[test]
    fn test_client_list_includes_docks_after_managed() {
        let mut state = make_state_with_windows(&[(0, 5, true), (0, 2, true)], 25);
        state.track_startup_dock(Window::new(20));
        state.track_startup_dock(Window::new(10));

        let list = state.client_list_windows();
        assert_eq!(
            list,
            vec![
                Window::new(2),
                Window::new(5),
                Window::new(10),
                Window::new(20)
            ]
        );
    }

    #[test]
    fn test_focus_window_uses_desktop_hint_when_untracked() {
        let mut state = make_state_with_windows(&[(0, 1, true), (1, 11, true)], 25);
        let effects = state.focus_window(Window::new(11), Some(1));

        assert_eq!(state.current_workspace_id(), 1);
        assert_eq!(state.focused_window(), Some(Window::new(11)));
        assert!(effects.iter().any(|e| matches!(e, Effect::Map(_))));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::Configure { .. }))
        );
    }

    #[test]
    fn test_go_to_workspace_invalid_or_same_is_noop() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let effects_same = state.go_to_workspace(0);
        let effects_invalid = state.go_to_workspace(NUM_WORKSPACES + 1);

        assert!(effects_same.is_empty());
        assert!(effects_invalid.is_empty());
        assert_eq!(state.current_workspace_id(), 0);
    }

    #[test]
    fn test_send_to_workspace_invalid_or_same_is_noop() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let effects_same = state.send_to_workspace(0);
        let effects_invalid = state.send_to_workspace(NUM_WORKSPACES + 1);

        assert!(effects_same.is_empty());
        assert!(effects_invalid.is_empty());
        assert_eq!(state.window_workspace(Window::new(1)), Some(0));
    }

    #[test]
    fn test_increase_decrease_window_gap_reconfigures() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, true)], 25);

        let effects_increase = state.increase_window_gap(1);
        assert_eq!(
            effects_increase
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .count(),
            2
        );

        let effects_decrease = state.decrease_window_gap(1);
        assert_eq!(
            effects_decrease
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .count(),
            2
        );

        let effects_noop = state.decrease_window_gap(1000);
        assert!(effects_noop.is_empty());
    }

    #[test]
    fn test_increase_decrease_window_weight_reconfigures() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, true)], 25);
        let _ = state.set_focus(Window::new(1));

        let effects_inc = state.increase_window_weight(2);
        assert_eq!(
            effects_inc
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .count(),
            2
        );

        let effects_dec = state.decrease_window_weight(1);
        assert_eq!(
            effects_dec
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn test_map_request_unmanaged_is_simple_map() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let effects = state.on_map_request(Window::new(99), WindowType::Unmanaged);

        assert_eq!(effects, vec![Effect::Map(Window::new(99))]);
        assert!(state.window_workspace(Window::new(99)).is_none());
    }

    #[test]
    fn test_dock_map_and_destroy_updates_layout() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let dock = Window::new(50);

        let map_effects = state.on_map_request(dock, WindowType::Dock);
        assert!(map_effects.contains(&Effect::Map(dock)));
        assert!(!state.dock_windows.is_empty());

        let destroy_effects = state.on_destroy(dock);
        assert!(
            !destroy_effects
                .iter()
                .any(|e| matches!(e, Effect::ConfigurePositionSize { .. }))
        );
        assert!(state.dock_windows.is_empty());
    }

    #[test]
    fn test_on_unmap_ignored_for_dock_and_unmanaged() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let dock = Window::new(77);
        state.track_startup_dock(dock);

        let effects_dock = state.on_unmap(dock);
        let effects_unmanaged = state.on_unmap(Window::new(88));

        assert!(effects_dock.is_empty());
        assert!(effects_unmanaged.is_empty());
    }

    #[test]
    fn test_startup_finalize_switches_workspace_when_hint_provided() {
        let mut state = make_state_with_windows(&[(0, 1, true), (1, 11, false)], 25);
        let effects = state.startup_finalize(Some(1));

        assert_eq!(state.current_workspace_id(), 1);
        assert!(effects.iter().any(|e| matches!(e, Effect::Map(_))));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::Configure { .. }))
        );
    }

    #[test]
    fn test_shift_focus_wraps_and_skips_unmapped() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, false), (0, 3, true)], 25);

        let _ = state.set_focus(Window::new(1));
        let effects_forward = state.shift_focus(1);

        assert_eq!(state.focused_window(), Some(Window::new(3)));
        assert!(effects_forward.contains(&Effect::Focus(Window::new(3))));

        let effects_backward = state.shift_focus(-1);
        assert_eq!(state.focused_window(), Some(Window::new(1)));
        assert!(effects_backward.contains(&Effect::Focus(Window::new(1))));
    }

    #[test]
    fn test_shift_focus_noop_when_only_one_mapped() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, false)], 25);
        let _ = state.set_focus(Window::new(1));

        let effects = state.shift_focus(1);

        assert!(effects.is_empty());
        assert_eq!(state.focused_window(), Some(Window::new(1)));
    }

    #[test]
    fn test_swap_window_swaps_with_next_mapped() {
        let mut state = make_state_with_windows(&[(0, 1, true), (0, 2, false), (0, 3, true)], 25);
        let _ = state.set_focus(Window::new(1));

        let effects = state.swap_window(1);

        let order: Vec<Window> = state.current_workspace().iter_windows().copied().collect();
        assert_eq!(order, vec![Window::new(3), Window::new(2), Window::new(1)]);
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, Effect::Configure { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn test_swap_window_noop_when_no_other_mapped() {
        let mut state = make_state_with_windows(&[(0, 1, true)], 25);
        let _ = state.set_focus(Window::new(1));

        let effects = state.swap_window(1);

        assert!(effects.is_empty());
        let order: Vec<Window> = state.current_workspace().iter_windows().copied().collect();
        assert_eq!(order, vec![Window::new(1)]);
    }
}
