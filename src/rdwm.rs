use log::{debug, error, info};
use std::process::Command;
use std::{collections::HashMap, process::Stdio};
use xcb::{
    Connection, Xid,
    x::{self, ModMask, Window},
};

use crate::atoms::Atoms;
use crate::config::{
    DEFAULT_BORDER_WIDTH, DEFAULT_DOCK_HEIGHT, DEFAULT_WINDOW_GAP, NUM_WORKSPACES,
};
use crate::effect::Effect;
use crate::ewmh_manager::EwmhManager;
use crate::key_mapping::ActionEvent;
use crate::keyboard::{fetch_keyboard_mapping, populate_key_bindings};
use crate::layout::{Layout, Rect};
use crate::workspace::Workspace;
use crate::x11::X11;

pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub focused_border_pixel: u32,
    pub normal_border_pixel: u32,
}

pub struct WindowManager<T: Layout> {
    x11: X11,
    workspaces: [Workspace; NUM_WORKSPACES],
    window_to_workspace: HashMap<Window, usize>,
    current_workspace: usize,
    key_bindings: HashMap<(u8, ModMask), ActionEvent>,
    screen: ScreenConfig,
    border_width: u32,
    window_gap: u32,
    layout: T,
    dock_windows: Vec<Window>,
    dock_height: u32,
}

impl<T: Layout> WindowManager<T> {
    pub fn new(layout: T) -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, _) = Connection::connect(None)?;
        info!("Connected to X.");

        let key_bindings = Self::setup_key_bindings(&conn);
        let (screen, root_window) = Self::setup_root(&conn);
        let atoms = Atoms::intern_all(&conn).expect("Failed to intern Atoms");

        // Create WM check window
        let wm_check_window = Self::create_wm_check_window(&conn, root_window);

        let x11 = X11::new(conn, root_window, atoms, wm_check_window);

        let wm = Self {
            x11,
            workspaces: Default::default(),
            window_to_workspace: Default::default(),
            current_workspace: 0,
            key_bindings,
            screen,
            border_width: DEFAULT_BORDER_WIDTH,
            window_gap: DEFAULT_WINDOW_GAP,
            layout,
            dock_windows: Vec::new(),
            dock_height: DEFAULT_DOCK_HEIGHT,
        };

        // Get root window and set up substructure redirect
        wm.x11.set_root_event_mask()?;
        info!("Successfully set substructure redirect");

        // Set up key grabs
        let keygrab_effects = wm.keygrab_effects();
        wm.x11.apply_effects_checked(&keygrab_effects);

        // Set up EWMH hints
        let ewmh_effects = EwmhManager::new(&wm.x11).publish_hints();
        wm.x11.apply_effects_unchecked(&ewmh_effects);

        // Publish geometry/workarea now that we know screen size.
        let mut ewmh_runtime_effects = vec![
            wm.ewmh()
                .desktop_geometry_effect(wm.screen.width, wm.screen.height),
            wm.ewmh()
                .workarea_effect(0, 0, wm.screen.width, wm.usable_screen_height()),
        ];
        ewmh_runtime_effects.extend(wm.ewmh_state_effects());
        wm.x11.apply_effects_unchecked(&ewmh_runtime_effects);

        Ok(wm)
    }

    /*

    ▗▄ ▗▖▗▄▄▄▖▄   ▄     ▗▖ ▗▖▗▄▄▄▖▗▖   ▗▄▄▖ ▗▄▄▄▖▗▄▄▖  ▗▄▖
    ▐█ ▐▌▐▛▀▀▘█   █     ▐▌ ▐▌▐▛▀▀▘▐▌   ▐▛▀▜▖▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▐▛▌▐▌▐▌   ▜▖█▗▛     ▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▙
    ▐▌█▐▌▐███ ▐▌█▐▌     ▐███▌▐███ ▐▌   ▐██▛ ▐███ ▐███  ▜█▙
    ▐▌▐▟▌▐▌   ▐█▀█▌     ▐▌ ▐▌▐▌   ▐▌   ▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐▌ █▌▐▙▄▄▖▐█ █▌     ▐▌ ▐▌▐▙▄▄▖▐▙▄▄▖▐▌   ▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▘ ▀▘▝▀▀▀▘▝▀ ▀▘     ▝▘ ▝▘▝▀▀▀▘▝▀▀▀▘▝▘   ▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn setup_key_bindings(conn: &Connection) -> HashMap<(u8, ModMask), ActionEvent> {
        let (keysyms, keysyms_per_keycode) = fetch_keyboard_mapping(conn);
        populate_key_bindings(conn, &keysyms, keysyms_per_keycode)
    }

    fn keygrab_effects(&self) -> Vec<Effect> {
        let mut effects = Vec::with_capacity(self.key_bindings.len());
        for &(keycode, modifiers) in self.key_bindings.keys() {
            effects.push(Effect::GrabKey {
                keycode,
                modifiers,
                grab_window: self.x11.root(),
            });
        }
        effects
    }

    fn setup_root(conn: &Connection) -> (ScreenConfig, Window) {
        let root = conn.get_setup().roots().next().expect("Cannot find root");
        let screen = ScreenConfig {
            width: u32::from(root.width_in_pixels()),
            height: u32::from(root.height_in_pixels()),
            focused_border_pixel: root.white_pixel(),
            normal_border_pixel: root.black_pixel(),
        };
        (screen, root.root())
    }

    fn create_wm_check_window(conn: &Connection, root: Window) -> Window {
        // Create a check window for _NET_SUPPORTING_WM_CHECK
        // This window is used by clients to verify the WM is EWMH compliant
        let win = conn.generate_id();
        let values = [x::Cw::OverrideRedirect(true)];
        conn.send_request(&x::CreateWindow {
            depth: 0,
            wid: win,
            parent: root,
            x: -1,
            y: -1,
            width: 1,
            height: 1,
            border_width: 0,
            class: x::WindowClass::InputOnly,
            visual: 0,
            value_list: &values,
        });
        win
    }

    /*

    ▗▄▄▄▖▄   ▄▗▄ ▄▖▗▖ ▗▖
    ▐▛▀▀▘█   █▐█ █▌▐▌ ▐▌
    ▐▌   ▜▖█▗▛▐███▌▐▌ ▐▌
    ▐███ ▐▌█▐▌▐▌█▐▌▐███▌
    ▐▌   ▐█▀█▌▐▌▀▐▌▐▌ ▐▌
    ▐▙▄▄▖▐█ █▌▐▌ ▐▌▐▌ ▐▌
    ▝▀▀▀▘▝▀ ▀▘▝▘ ▝▘▝▘ ▝▘

    */

    fn ewmh(&self) -> EwmhManager<'_> {
        EwmhManager::new(&self.x11)
    }

    /*

    ▗▖ ▗▖▗▄▄▄▖ ▄▄▄ ▗▖    ▗▄▖
    ▐▌ ▐▌▝▀█▀▘ ▀█▀ ▐▌   ▗▛▀▜
    ▐▌ ▐▌  █    █  ▐▌   ▐▙
    ▐▌ ▐▌  █    █  ▐▌    ▜█▙
    ▐▌ ▐▌  █    █  ▐▌      ▜▌
    ▝█▄█▘  █   ▄█▄ ▐▙▄▄▖▐▄▄▟▘
     ▝▀▘   ▀   ▀▀▀ ▝▀▀▀▘ ▀▀▘


    */

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

    fn get_root_window_children(&self) -> Result<Vec<Window>, xcb::Error> {
        self.x11.get_root_window_children()
    }

    fn usable_screen_height(&self) -> u32 {
        if !self.dock_windows.is_empty() {
            return self.screen.height.saturating_sub(self.dock_height);
        }
        self.screen.height
    }

    /*

    ▄   ▄ ▄▄▄ ▗▄ ▗▖▗▄▄   ▗▄▖ ▄   ▄     ▗▖ ▗▖▗▄▄▄▖▗▖   ▗▄▄▖ ▗▄▄▄▖▗▄▄▖  ▗▄▖
    █   █ ▀█▀ ▐█ ▐▌▐▛▀█  █▀█ █   █     ▐▌ ▐▌▐▛▀▀▘▐▌   ▐▛▀▜▖▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▜▖█▗▛  █  ▐▛▌▐▌▐▌ ▐▌▐▌ ▐▌▜▖█▗▛     ▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▙
    ▐▌█▐▌  █  ▐▌█▐▌▐▌ ▐▌▐▌ ▐▌▐▌█▐▌     ▐███▌▐███ ▐▌   ▐██▛ ▐███ ▐███  ▜█▙
    ▐█▀█▌  █  ▐▌▐▟▌▐▌ ▐▌▐▌ ▐▌▐█▀█▌     ▐▌ ▐▌▐▌   ▐▌   ▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐█ █▌ ▄█▄ ▐▌ █▌▐▙▄█  █▄█ ▐█ █▌     ▐▌ ▐▌▐▙▄▄▖▐▙▄▄▖▐▌   ▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▀ ▀▘ ▀▀▀ ▝▘ ▀▘▝▀▀   ▝▀▘ ▝▀ ▀▘     ▝▘ ▝▘▝▀▀▀▘▝▀▀▀▘▝▘   ▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn configure_windows(&self, workspace_id: usize) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];
        if let Some(current_workspace) = self.get_workspace(workspace_id) {
            if let Some(fullscreen) = current_workspace.fullscreen_window()
                && current_workspace.is_window_mapped(fullscreen)
            {
                debug!(
                    "Fullscreen {fullscreen:?} present on workspace {workspace_id}. Overriding Layout"
                );
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
                debug!("No windows to configure");
                return effects;
            }

            let weights: Vec<u32> = clients.iter().map(|client| client.size()).collect();
            let area = Rect {
                x: 0,
                y: 0,
                w: self.screen.width,
                h: self.usable_screen_height(),
            };
            let layout =
                self.layout
                    .generate_layout(area, &weights, self.border_width, self.window_gap);

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

    fn configure_dock_windows(&self) -> Vec<Effect> {
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

    fn set_focus(&mut self, idx: usize) -> Vec<Effect> {
        if let Some(fs) = self.current_workspace().fullscreen_window()
            && self.current_workspace().is_window_mapped(fs)
            && let Some(fs_idx) = self.current_workspace().index_of_window(fs)
            && idx != fs_idx
        {
            return vec![];
        }

        let mut effects = Vec::new();

        let fullscreen_window = self.current_workspace().fullscreen_window();

        if let Some(old_window) = self.current_workspace().get_focused_window() {
            effects.push(Effect::SetBorder {
                window: old_window,
                pixel: self.screen.normal_border_pixel,
                width: if fullscreen_window == Some(old_window) {
                    0
                } else {
                    self.border_width
                },
            });
        }

        self.current_workspace_mut().set_focus(idx);

        if let Some(new_focus_window) = self.current_workspace().get_focused_window() {
            effects.push(Effect::SetBorder {
                window: new_focus_window,
                pixel: self.screen.focused_border_pixel,
                width: if fullscreen_window == Some(new_focus_window) {
                    0
                } else {
                    self.border_width
                },
            });
            effects.push(Effect::Focus(new_focus_window));
            if fullscreen_window == Some(new_focus_window) {
                effects.push(Effect::Raise(new_focus_window));
            }
        }

        effects.push(
            self.ewmh()
                .active_window_effect(self.current_workspace().get_focused_window()),
        );

        effects
    }

    fn toggle_fullscreen(&mut self) -> Vec<Effect> {
        let Some(focused) = self.current_workspace().get_focused_window() else {
            return vec![];
        };

        let prev_fullscreen = self.current_workspace().fullscreen_window();
        let turning_off = prev_fullscreen == Some(focused);

        if turning_off {
            self.current_workspace_mut().set_fullscreen(None);
        } else {
            self.current_workspace_mut().set_fullscreen(Some(focused));
        }

        let mut effects = Vec::new();

        // Clear old fullscreen state if needed.
        if let Some(old) = prev_fullscreen
            && (turning_off || old != focused)
        {
            effects.push(self.ewmh().window_fullscreen_state_effect(old, false));
        }

        // Apply state to the target window.
        effects.push(
            self.ewmh()
                .window_fullscreen_state_effect(focused, !turning_off),
        );

        // Relayout and re-assert focus/border policy.
        effects.extend(self.configure_windows(self.current_workspace));
        if let Some(idx) = self.current_workspace().index_of_window(focused) {
            effects.extend(self.set_focus(idx));
        }
        if !turning_off {
            effects.push(Effect::Raise(focused));
        }

        effects.extend(self.ewmh_state_effects());
        effects
    }

    fn all_managed_windows(&self) -> Vec<Window> {
        // window_to_workspace is authoritative for client windows.
        let mut entries = self
            .window_to_workspace
            .iter()
            .map(|(w, ws)| (*ws, w.resource_id(), *w))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(ws, id, _w)| (*ws, *id));

        let mut out = entries
            .into_iter()
            .map(|(_ws, _id, w)| w)
            .collect::<Vec<_>>();

        // Docks are managed but not part of workspaces.
        let mut docks = self.dock_windows.clone();
        docks.sort_by_key(xcb::Xid::resource_id);
        out.extend(docks);

        out
    }

    fn ewmh_state_effects(&self) -> Vec<Effect> {
        let windows = self.all_managed_windows();
        let mut effects = self.ewmh().client_list_effects(&windows);
        effects.push(
            self.ewmh()
                .active_window_effect(self.current_workspace().get_focused_window()),
        );
        effects
    }

    fn focus_window(&mut self, window: Window) -> Vec<Effect> {
        let mut effects = Vec::new();

        let workspace_id = self
            .window_to_workspace
            .get(&window)
            .copied()
            .or_else(|| self.ewmh().get_window_desktop(window).map(|d| d as usize));

        let Some(workspace_id) = workspace_id else {
            return effects;
        };

        if workspace_id < NUM_WORKSPACES && workspace_id != self.current_workspace {
            effects.extend(self.go_to_workspace(workspace_id));
        }

        if let Some(idx) = self.current_workspace().index_of_window(window) {
            effects.extend(self.set_focus(idx));
        }

        effects
    }

    fn close_window(&mut self, window: Window) -> Vec<Effect> {
        // Keep internal state until Unmap/Destroy arrives; just request close.
        match self.x11.supports_wm_delete(window) {
            Ok(true) => vec![Effect::SendWmDelete(window)],
            Ok(false) => vec![Effect::KillClient(window)],
            Err(e) => {
                error!(
                    "Failed to query WM_PROTOCOLS for {window:?}: {e:?}. Falling back to force kill."
                );
                vec![Effect::KillClient(window)]
            }
        }
    }

    fn handle_client_message(&mut self, ev: &x::ClientMessageEvent) -> Vec<Effect> {
        let atoms = self.x11.atoms();
        let msg_type = ev.r#type();

        let data32 = match ev.data() {
            x::ClientMessageData::Data32(d) => d,
            _ => return vec![],
        };

        if msg_type == atoms.current_desktop {
            return self.go_to_workspace(data32[0] as usize);
        }

        if msg_type == atoms.active_window {
            let target = ev.window();
            let mut effects = self.focus_window(target);
            effects.extend(self.ewmh_state_effects());
            return effects;
        }

        if msg_type == atoms.close_window {
            let target = ev.window();
            let mut effects = self.close_window(target);
            effects.extend(self.ewmh_state_effects());
            return effects;
        }

        vec![]
    }

    /*

      ▄    ▄▄ ▗▄▄▄▖ ▄▄▄  ▗▄▖ ▗▄ ▗▖     ▗▖ ▗▖  ▄  ▗▄ ▗▖▗▄▄  ▗▖   ▗▄▄▄▖▗▄▄▖  ▗▄▖
     ▐█▌  █▀▀▌▝▀█▀▘ ▀█▀  █▀█ ▐█ ▐▌     ▐▌ ▐▌ ▐█▌ ▐█ ▐▌▐▛▀█ ▐▌   ▐▛▀▀▘▐▛▀▜▌▗▛▀▜
     ▐█▌ ▐▛     █    █  ▐▌ ▐▌▐▛▌▐▌     ▐▌ ▐▌ ▐█▌ ▐▛▌▐▌▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▙
     █ █ ▐▌     █    █  ▐▌ ▐▌▐▌█▐▌     ▐███▌ █ █ ▐▌█▐▌▐▌ ▐▌▐▌   ▐███ ▐███  ▜█▙
     ███ ▐▙     █    █  ▐▌ ▐▌▐▌▐▟▌     ▐▌ ▐▌ ███ ▐▌▐▟▌▐▌ ▐▌▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▗█ █▖ █▄▄▌  █   ▄█▄  █▄█ ▐▌ █▌     ▐▌ ▐▌▗█ █▖▐▌ █▌▐▙▄█ ▐▙▄▄▖▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▘ ▝▘  ▀▀   ▀   ▀▀▀  ▝▀▘ ▝▘ ▀▘     ▝▘ ▝▘▝▘ ▝▘▝▘ ▀▘▝▀▀  ▝▀▀▀▘▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn spawn_client(&self, cmd: &str) {
        info!("Spawning command: {cmd}");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            error!("Empty command provided");
            return;
        }

        let mut command = Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }

        match command.spawn() {
            Ok(_) => info!("Successfully spawned: {cmd}"),
            Err(e) => error!("Failed to spawn {cmd}: {e:?}"),
        }
    }

    fn kill_client(&mut self) -> Vec<Effect> {
        if let Some(window) = self.current_workspace_mut().removed_focused_window() {
            self.window_to_workspace.remove(&window);
            info!("Killing client window: {window:?}");

            match self.x11.supports_wm_delete(window) {
                Ok(true) => vec![Effect::SendWmDelete(window)],
                Ok(false) => vec![Effect::KillClient(window)],
                Err(e) => {
                    error!(
                        "Failed to query WM_PROTOCOLS for {window:?}: {e:?}. Falling back to force kill."
                    );
                    vec![Effect::KillClient(window)]
                }
            }
        } else {
            vec![]
        }
    }

    fn next_window_index(&mut self, direction: isize) -> Option<usize> {
        let curr_workspace = self.current_workspace_mut();
        let window_count: isize = curr_workspace.num_of_windows() as isize;

        if window_count == 0 {
            debug!("No windows to focus");
            return None;
        }

        let curr = curr_workspace.get_focus().unwrap_or(0) as isize;
        Some(((curr + direction).rem_euclid(window_count)) as usize)
    }

    fn shift_focus(&mut self, direction: isize) -> Vec<Effect> {
        let Some(next_focus) = self.next_window_index(direction) else {
            return vec![];
        };

        debug!("Focus shifted to window index: {next_focus}");
        self.set_focus(next_focus)
    }

    fn swap_window(&mut self, direction: isize) -> Vec<Effect> {
        let Some(next_window) = self.next_window_index(direction) else {
            return vec![];
        };

        let Some(focus) = self.current_workspace().get_focus() else {
            error!(
                "Failed to get focus for current current_workspace {}",
                self.current_workspace
            );
            return vec![];
        };

        {
            let curr_workspace = self.current_workspace_mut();
            curr_workspace.swap_windows(focus, next_window);
        }

        let mut effects = self.set_focus(next_window);
        effects.extend(self.configure_windows(self.current_workspace));
        effects
    }

    fn increase_window_weight(&mut self, increment: u32) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];
        if let Some(focused_win) = self.current_workspace_mut().get_focused_client_mut() {
            focused_win.increase_window_size(increment);
            effects = self.configure_windows(self.current_workspace);
        }

        effects
    }

    fn decrease_window_weight(&mut self, increment: u32) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];
        if let Some(focused_win) = self.current_workspace_mut().get_focused_client_mut() {
            focused_win.decrease_window_size(increment);
            effects = self.configure_windows(self.current_workspace);
        }
        effects
    }

    fn increase_window_gap(&mut self, increment: u32) -> Vec<Effect> {
        self.window_gap += increment;
        self.configure_windows(self.current_workspace)
    }

    fn decrease_window_gap(&mut self, decrement: u32) -> Vec<Effect> {
        let new_gap = self.window_gap.saturating_sub(decrement);

        if new_gap == self.window_gap {
            return vec![];
        }

        self.window_gap = new_gap;
        self.configure_windows(self.current_workspace)
    }

    fn go_to_workspace(&mut self, new_workspace_id: usize) -> Vec<Effect> {
        let mut effects: Vec<Effect> = vec![];

        if self.current_workspace == new_workspace_id || new_workspace_id >= NUM_WORKSPACES {
            return effects;
        }

        debug!(
            "Switching from current_workspace {} to {new_workspace_id}",
            self.current_workspace
        );

        let old_workspace_id = self.current_workspace;
        let old_windows: Vec<Window> = self
            .workspaces
            .get(old_workspace_id)
            .expect("Workspace should never be out of bounds")
            .iter_windows()
            .copied()
            .collect();

        {
            // WM-driven Unmap: keep internal is_mapped consistent.
            let old_ws = self
                .workspaces
                .get_mut(old_workspace_id)
                .expect("Workspace should never be out of bounds");
            for &win in &old_windows {
                old_ws.set_client_mapped(win, false);
            }
        }

        for win in old_windows {
            debug!("Unmapping {win:?}");
            effects.push(Effect::Unmap(win));
        }

        self.current_workspace = new_workspace_id;

        let new_windows: Vec<Window> = self.current_workspace().iter_windows().copied().collect();

        {
            // WM-driven Map: keep internal is_mapped consistent.
            let new_ws = self.current_workspace_mut();
            for &win in &new_windows {
                new_ws.set_client_mapped(win, true);
            }
        }

        for win in new_windows {
            debug!("Mapping {win:?}");
            effects.push(Effect::Map(win));
        }

        effects.extend(self.configure_windows(self.current_workspace));
        effects.push(self.ewmh().current_desktop_effect(self.current_workspace));
        if let Some(focus) = self.current_workspace().get_focus() {
            effects.extend(self.set_focus(focus));
        } else {
            effects.push(self.ewmh().active_window_effect(None));
        }

        effects
    }

    fn send_to_workspace(&mut self, workspace_id: usize) -> Vec<Effect> {
        let mut effects = Vec::new();
        if workspace_id >= NUM_WORKSPACES {
            return effects;
        }

        match self.current_workspace_mut().removed_focused_window() {
            Some(window_to_send) => {
                // Fullscreen is per-workspace; moving a window clears its fullscreen state.
                effects.push(
                    self.ewmh()
                        .window_fullscreen_state_effect(window_to_send, false),
                );
                if let Some(new_workspace) = self.workspaces.get_mut(workspace_id) {
                    new_workspace.push_window(window_to_send);
                    // We immediately Unmap it (since it's not on the current workspace anymore).
                    // Keep internal state consistent so layouts don't skip/phantom include it.
                    new_workspace.set_client_mapped(window_to_send, false);
                    self.window_to_workspace
                        .insert(window_to_send, workspace_id);
                    effects.push(Effect::Unmap(window_to_send));
                    effects.push(Effect::SetBorder {
                        window: window_to_send,
                        pixel: self.screen.normal_border_pixel,
                        width: self.border_width,
                    });
                    effects.extend(self.configure_windows(self.current_workspace));
                    effects.extend(self.configure_windows(workspace_id));
                    effects.extend(self.shift_focus(0));
                    effects.push(
                        self.ewmh()
                            .window_desktop_effect(window_to_send, workspace_id as u32),
                    );
                }
            }
            None => error!(
                "Failed to remove focused window from current_workspace {}",
                self.current_workspace
            ),
        }

        effects
    }

    /*

    ▗▄▄▄▖▗▖ ▗▖▗▄▄▄▖▗▄ ▗▖▗▄▄▄▖     ▗▖ ▗▖  ▄  ▗▄ ▗▖▗▄▄  ▗▖   ▗▄▄▄▖▗▄▄▖  ▗▄▖
    ▐▛▀▀▘▝█ █▘▐▛▀▀▘▐█ ▐▌▝▀█▀▘     ▐▌ ▐▌ ▐█▌ ▐█ ▐▌▐▛▀█ ▐▌   ▐▛▀▀▘▐▛▀▜▌▗▛▀▜
    ▐▌    █ █ ▐▌   ▐▛▌▐▌  █       ▐▌ ▐▌ ▐█▌ ▐▛▌▐▌▐▌ ▐▌▐▌   ▐▌   ▐▌ ▐▌▐▙
    ▐███  █ █ ▐███ ▐▌█▐▌  █       ▐███▌ █ █ ▐▌█▐▌▐▌ ▐▌▐▌   ▐███ ▐███  ▜█▙
    ▐▌    ▐█▌ ▐▌   ▐▌▐▟▌  █       ▐▌ ▐▌ ███ ▐▌▐▟▌▐▌ ▐▌▐▌   ▐▌   ▐▌▝█▖   ▜▌
    ▐▙▄▄▖ ▐█▌ ▐▙▄▄▖▐▌ █▌  █       ▐▌ ▐▌▗█ █▖▐▌ █▌▐▙▄█ ▐▙▄▄▖▐▙▄▄▖▐▌ ▐▌▐▄▄▟▘
    ▝▀▀▀▘ ▝▀▘ ▝▀▀▀▘▝▘ ▀▘  ▀       ▝▘ ▝▘▝▘ ▝▘▝▘ ▀▘▝▀▀  ▝▀▀▀▘▝▀▀▀▘▝▘ ▝▀ ▀▀▘

    */

    fn handle_key_press(&mut self, ev: &x::KeyPressEvent) -> Vec<Effect> {
        let keycode = ev.detail();
        let modifiers = ModMask::from_bits_truncate(ev.state().bits());

        if let Some(action) = self.key_bindings.get(&(keycode, modifiers)) {
            match action {
                ActionEvent::Spawn(cmd) => {
                    self.spawn_client(cmd);
                    vec![]
                }
                ActionEvent::Kill => self.kill_client(),
                ActionEvent::NextWindow => self.shift_focus(1),
                ActionEvent::PrevWindow => self.shift_focus(-1),
                ActionEvent::GoToWorkspace(workspace_id) => self.go_to_workspace(*workspace_id),
                ActionEvent::SendToWorkspace(workspace_id) => self.send_to_workspace(*workspace_id),
                ActionEvent::SwapRight => self.swap_window(1),
                ActionEvent::SwapLeft => self.swap_window(-1),
                ActionEvent::IncreaseWindowWeight(increment) => {
                    self.increase_window_weight(*increment)
                }
                ActionEvent::DecreaseWindowWeight(increment) => {
                    self.decrease_window_weight(*increment)
                }
                ActionEvent::IncreaseWindowGap(increment) => self.increase_window_gap(*increment),
                ActionEvent::DecreaseWindowGap(increment) => self.decrease_window_gap(*increment),
                ActionEvent::ToggleFullscreen => self.toggle_fullscreen(),
            }
        } else {
            error!("No binding found for keycode: {keycode} with modifiers: {modifiers:?}",);
            vec![]
        }
    }

    fn handle_map_request(&mut self, window: Window) -> Vec<Effect> {
        let mut effects = Vec::new();
        // Check if this is a dock window
        if self.x11.is_dock_window(window) {
            debug!("Mapping dock window: {window:?}");
            let was_empty = self.dock_windows.is_empty();
            self.dock_windows.push(window);
            effects.push(Effect::Map(window));
            effects.extend(self.configure_dock_windows());

            if was_empty {
                effects.push(self.ewmh().workarea_effect(
                    0,
                    0,
                    self.screen.width,
                    self.usable_screen_height(),
                ));
            }
        } else {
            // Regular window - add to current current_workspace
            match self
                .current_workspace_mut()
                .get_client_mut(&window.resource_id())
            {
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

            if let Some(fs) = self.current_workspace().fullscreen_window()
                && self.current_workspace().is_window_mapped(fs)
            {
                // Fullscreen stays focused and on top; new windows should not steal focus.
                effects.extend(self.configure_windows(self.current_workspace));
                if let Some(fs_idx) = self.current_workspace().index_of_window(fs) {
                    effects.extend(self.set_focus(fs_idx));
                }
            } else {
                let idx = self.current_workspace().num_of_windows().saturating_sub(1);
                effects.extend(self.set_focus(idx));
                effects.extend(self.configure_windows(self.current_workspace));
            }
            effects.push(
                self.ewmh()
                    .window_desktop_effect(window, self.current_workspace as u32),
            );
        }

        effects.extend(self.ewmh_state_effects());

        effects
    }

    fn handle_destroy_event(&mut self, window: Window) -> Vec<Effect> {
        // Check if it's a dock window
        let window_id = window.resource_id();
        let was_dock = self
            .dock_windows
            .iter()
            .any(|w| w.resource_id() == window_id);

        if was_dock {
            debug!("Dock window destroyed: {window:?}");
            let was_nonempty = !self.dock_windows.is_empty();
            self.dock_windows.retain(|w| w.resource_id() != window_id);

            let mut effects = Vec::new();
            if was_nonempty && self.dock_windows.is_empty() {
                effects.push(self.ewmh().workarea_effect(
                    0,
                    0,
                    self.screen.width,
                    self.usable_screen_height(),
                ));
            }
            effects.extend(self.ewmh_state_effects());
            return effects;
        }

        if let Some(workspace_id) = self.window_to_workspace.remove(&window)
            && let Some(current_workspace) = self.workspaces.get_mut(workspace_id)
        {
            current_workspace.remove_client(&window_id);
        }

        let mut effects = self.shift_focus(0);
        effects.extend(self.configure_windows(self.current_workspace));
        effects.extend(self.ewmh_state_effects());
        effects
    }

    fn handle_unmap_event(&mut self, window: Window) -> Vec<Effect> {
        let Some(&workspace_id) = self.window_to_workspace.get(&window) else {
            // Likely a dock or unmanaged window.
            return vec![];
        };

        let mut changed = false;
        if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
            if let Some(client) = workspace.get_client_mut(&window.resource_id())
                && client.is_mapped()
            {
                client.set_mapped(false);
                changed = true;
            }
        }

        // If the window is not in the current workspace, avoid perturbing focus/layout.
        if workspace_id != self.current_workspace {
            return vec![];
        }

        if !changed {
            return vec![];
        }

        let mut effects = Vec::new();
        effects.extend(self.shift_focus(-1));
        effects.extend(self.configure_windows(self.current_workspace));
        effects.extend(self.ewmh_state_effects());
        effects
    }

    /*

    ▗▄ ▄▖  ▄   ▄▄▄ ▗▄ ▗▖     ▗▖    ▗▄▖  ▗▄▖ ▗▄▄▖
    ▐█ █▌ ▐█▌  ▀█▀ ▐█ ▐▌     ▐▌    █▀█  █▀█ ▐▛▀▜▖
    ▐███▌ ▐█▌   █  ▐▛▌▐▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐▌ ▐▌
    ▐▌█▐▌ █ █   █  ▐▌█▐▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐██▛
    ▐▌▀▐▌ ███   █  ▐▌▐▟▌     ▐▌   ▐▌ ▐▌▐▌ ▐▌▐▌
    ▐▌ ▐▌▗█ █▖ ▄█▄ ▐▌ █▌     ▐▙▄▄▖ █▄█  █▄█ ▐▌
    ▝▘ ▝▘▝▘ ▝▘ ▀▀▀ ▝▘ ▀▘     ▝▀▀▀▘ ▝▀▘  ▝▀▘ ▝▘

    */

    fn spawn_autostart() {
        match Command::new("sh")
            .arg("-c")
            .arg("exec ~/.config/rdwm/autostart.sh")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => debug!("Ran autostart succesfully!"),
            Err(e) => debug!("Failed to run autostart: {e:?}"),
        }
    }

    fn grab_windows(&mut self) -> Vec<Effect> {
        match self.get_root_window_children() {
            Ok(children) => {
                children.iter().for_each(|window| {
                    if let Some(workspace_id) = self.ewmh().get_window_desktop(*window)
                        && let Some(current_workspace) =
                            self.get_workspace_mut(workspace_id as usize)
                    {
                        debug!("Assigning {window:?} to desktop {workspace_id}");
                        current_workspace.push_window(*window);
                        self.window_to_workspace
                            .insert(*window, workspace_id as usize);
                    };
                });
            }

            Err(e) => error!("Failed to grab children of root at startup: {e:?}"),
        }

        if let Some(workspace_id) = self.ewmh().get_current_desktop() {
            debug!("Desktop upon restart is {workspace_id}");
            self.current_workspace = (workspace_id as usize + 1) % NUM_WORKSPACES;
            return self.go_to_workspace(workspace_id as usize);
        }

        vec![]
    }

    pub fn run(&mut self) -> xcb::Result<()> {
        Self::spawn_autostart();
        let startup_effects = self.grab_windows();
        self.x11.apply_effects_unchecked(&startup_effects);
        loop {
            let event = match self.x11.wait_for_event() {
                Ok(ev) => ev,
                Err(xcb::Error::Protocol(e)) => {
                    error!("X11 protocol error: {e:?}");
                    continue;
                }
                Err(e) => return Err(e),
            };

            match event {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    debug!("Received KeyPress event: {ev:?}");
                    let effects = self.handle_key_press(&ev);
                    self.x11.apply_effects_unchecked(&effects);
                }

                xcb::Event::X(x::Event::MapRequest(ev)) => {
                    debug!("Received MapRequest event for {:?}", ev.window());
                    let effects = self.handle_map_request(ev.window());
                    self.x11.apply_effects_unchecked(&effects);
                }

                xcb::Event::X(x::Event::DestroyNotify(ev)) => {
                    debug!("Received DestroyNotify event for  {:?}", ev.window());
                    let effects = self.handle_destroy_event(ev.window());
                    self.x11.apply_effects_unchecked(&effects);
                }

                xcb::Event::X(x::Event::UnmapNotify(ev)) => {
                    debug!("Received UnmapNotify event for {:?}", ev.window());
                    let effects = self.handle_unmap_event(ev.window());
                    self.x11.apply_effects_unchecked(&effects);
                }

                xcb::Event::X(x::Event::ClientMessage(ev)) => {
                    debug!("Received ClientMessage event: {ev:?}");
                    let effects = self.handle_client_message(&ev);
                    self.x11.apply_effects_unchecked(&effects);
                }

                xcb::Event::X(x::Event::MapNotify(ev)) => {
                    debug!("Window mapped: {:?}", ev.window());
                }

                ev => {
                    debug!("Ignoring event: {ev:?}");
                }
            }
        }
    }
}
