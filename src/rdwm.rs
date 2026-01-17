use log::{debug, error, info};
use std::process::Command;
use std::{collections::HashMap, process::Stdio};

use xcb::{
    Connection,
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
use crate::state::{ScreenConfig, State};
use crate::x11::{WindowType, X11};

pub struct WindowManager {
    x11: X11,
    ewmh: EwmhManager,
    key_bindings: HashMap<(u8, ModMask), ActionEvent>,
    state: State,
}

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, _) = Connection::connect(None)?;
        info!("Connected to X.");

        let key_bindings = Self::setup_key_bindings(&conn);
        let (screen, root_window) = Self::setup_root(&conn);
        let atoms = Atoms::intern_all(&conn).expect("Failed to intern Atoms");

        // Create WM check window
        let wm_check_window = Self::create_wm_check_window(&conn, root_window);
        let x11 = X11::new(conn, root_window, atoms, wm_check_window);
        let ewmh = EwmhManager::new(&x11);

        let state = State::new(
            screen,
            DEFAULT_BORDER_WIDTH,
            DEFAULT_WINDOW_GAP,
            DEFAULT_DOCK_HEIGHT,
        );

        let wm = Self {
            x11,
            ewmh,
            key_bindings,
            state,
        };

        wm.x11.set_root_event_mask()?;
        info!("Successfully set substructure redirect");

        // Key grabs
        let keygrab_effects = wm.keygrab_effects();
        wm.x11.apply_effects_checked(&keygrab_effects);

        // EWMH hints
        let ewmh_effects = wm.ewmh.publish_hints();
        wm.x11.apply_effects_unchecked(&ewmh_effects);

        // Publish geometry now that we know screen size, then sync full EWMH state.
        let screen = wm.state.screen();
        let mut ewmh_runtime_effects =
            vec![wm.ewmh.desktop_geometry_effect(screen.width, screen.height)];
        ewmh_runtime_effects.extend(wm.ewmh_sync_effects());
        wm.x11.apply_effects_unchecked(&ewmh_runtime_effects);

        Ok(wm)
    }

    fn ewmh_sync_effects(&self) -> Vec<Effect> {
        let ewmh = &self.ewmh;
        let screen = self.state.screen();

        let client_list = self.state.client_list_windows();
        let managed = self.state.managed_windows_sorted();

        let mut effects = Vec::new();
        effects.extend(ewmh.client_list_effects(&client_list));
        effects.push(ewmh.current_desktop_effect(self.state.current_workspace_id()));
        effects.push(ewmh.active_window_effect(self.state.focused_window()));
        effects.push(ewmh.workarea_effect(0, 0, screen.width, self.state.usable_screen_height()));

        for window in managed {
            if let Some(workspace) = self.state.window_workspace(window) {
                effects.push(ewmh.window_desktop_effect(window, workspace as u32));
            }
            effects.push(
                ewmh.window_fullscreen_state_effect(
                    window,
                    self.state.is_window_fullscreen(window),
                ),
            );
        }

        effects
    }

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

    fn close_window(&self, window: Window) -> Vec<Effect> {
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

    fn handle_key_press(&mut self, ev: &x::KeyPressEvent) -> Vec<Effect> {
        let keycode = ev.detail();
        let modifiers = ModMask::from_bits_truncate(ev.state().bits());

        let Some(action) = self.key_bindings.get(&(keycode, modifiers)) else {
            error!("No binding found for keycode: {keycode} with modifiers: {modifiers:?}");
            return vec![];
        };

        match action {
            ActionEvent::Spawn(cmd) => {
                self.spawn_client(cmd);
                vec![]
            }
            ActionEvent::Kill => {
                let Some(window) = self.state.focused_window() else {
                    return vec![];
                };

                self.close_window(window)
            }
            _ => {
                let mut effects = self.state.apply_action(*action);
                effects.extend(self.ewmh_sync_effects());
                effects
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
            let mut effects = self.state.go_to_workspace(data32[0] as usize);
            effects.extend(self.ewmh_sync_effects());
            return effects;
        }

        if msg_type == atoms.active_window {
            let target = ev.window();
            let desktop_hint = self
                .ewmh
                .get_window_desktop(&self.x11, target)
                .map(|d| d as usize);
            let mut effects = self.state.focus_window(target, desktop_hint);
            effects.extend(self.ewmh_sync_effects());
            return effects;
        }

        if msg_type == atoms.close_window {
            let target = ev.window();
            return self.close_window(target);
        }

        vec![]
    }

    fn grab_windows(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();

        match self.x11.get_root_window_children() {
            Ok(children) => {
                debug!("Startup scan: {} root children", children.len());
                for window in children {
                    match self.x11.classify_window(window) {
                        WindowType::Dock => {
                            self.state.track_startup_dock(window);
                        }
                        WindowType::Managed => {
                            if let Some(workspace_id) =
                                self.ewmh.get_window_desktop(&self.x11, window)
                                && (workspace_id as usize) < NUM_WORKSPACES
                            {
                                self.state
                                    .track_startup_managed(window, workspace_id as usize);
                            }
                        }
                        WindowType::Unmanaged => {
                            continue;
                        }
                    }
                }
            }
            Err(e) => error!("Failed to grab children of root at startup: {e:?}"),
        }

        let current_desktop = self.ewmh.get_current_desktop(&self.x11).map(|d| d as usize);
        effects.extend(self.state.startup_finalize(current_desktop));
        effects.extend(self.ewmh_sync_effects());
        effects
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
                    let wt = self.x11.classify_window(ev.window());
                    let mut effects = self.state.on_map_request(ev.window(), wt);
                    effects.extend(self.ewmh_sync_effects());
                    self.x11.apply_effects_unchecked(&effects);
                }
                xcb::Event::X(x::Event::DestroyNotify(ev)) => {
                    debug!("Received DestroyNotify event for  {:?}", ev.window());
                    let mut effects = self.state.on_destroy(ev.window());
                    effects.extend(self.ewmh_sync_effects());
                    self.x11.apply_effects_unchecked(&effects);
                }
                xcb::Event::X(x::Event::UnmapNotify(ev)) => {
                    debug!("Received UnmapNotify event for {:?}", ev.window());
                    let mut effects = self.state.on_unmap(ev.window());
                    effects.extend(self.ewmh_sync_effects());
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
