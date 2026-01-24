use xcb::{
    Xid,
    x::{self, Window},
};

use std::process;

use crate::{atoms::Atoms, config::NUM_WORKSPACES, effect::Effect, x11::X11};

pub struct EwmhManager {
    atoms: Atoms,
    root: Window,
    wm_check_window: Window,
}

impl EwmhManager {
    pub fn new(atoms: Atoms, root: Window, wm_check_window: Window) -> Self {
        Self {
            atoms,
            root,
            wm_check_window,
        }
    }

    pub fn publish_hints(&self) -> Vec<Effect> {
        let atoms = &self.atoms;
        let root = self.root;
        let check = self.wm_check_window;

        let supported_atoms = [
            atoms.supported,
            atoms.supporting_wm_check,
            atoms.number_of_desktops,
            atoms.current_desktop,
            atoms.desktop_names,
            atoms.desktop_viewport,
            atoms.desktop_geometry,
            atoms.workarea,
            atoms.showing_desktop,
            atoms.active_window,
            atoms.client_list,
            atoms.client_list_stacking,
            atoms.wm_name,
            atoms.wm_pid,
            atoms.wm_window_type,
            atoms.wm_window_type_dock,
            atoms.wm_strut_partial,
            atoms.wm_state,
            atoms.wm_state_fullscreen,
            atoms.wm_desktop,
            atoms.close_window,
        ];

        let mut desktop_names = String::new();
        for i in 0..NUM_WORKSPACES {
            desktop_names.push_str(&(i + 1).to_string());
            desktop_names.push('\0');
        }

        let viewport_zeros = vec![0u32; NUM_WORKSPACES * 2];

        vec![
            Effect::SetWindowProperty {
                window: root,
                atom: atoms.supporting_wm_check,
                values: vec![check.resource_id()],
            },
            Effect::SetWindowProperty {
                window: check,
                atom: atoms.supporting_wm_check,
                values: vec![check.resource_id()],
            },
            Effect::SetUtf8String {
                window: check,
                atom: atoms.wm_name,
                value: "rdwm".to_string(),
            },
            Effect::SetCardinal32 {
                window: check,
                atom: atoms.wm_pid,
                value: process::id(),
            },
            Effect::SetAtomList {
                window: root,
                atom: atoms.supported,
                values: supported_atoms
                    .iter()
                    .map(xcb::Xid::resource_id)
                    .collect::<Vec<_>>(),
            },
            Effect::SetCardinal32 {
                window: root,
                atom: atoms.number_of_desktops,
                value: NUM_WORKSPACES as u32,
            },
            Effect::SetCardinal32 {
                window: root,
                atom: atoms.current_desktop,
                value: 0,
            },
            Effect::SetCardinal32 {
                window: root,
                atom: atoms.showing_desktop,
                value: 0,
            },
            Effect::SetCardinal32List {
                window: root,
                atom: atoms.desktop_viewport,
                values: viewport_zeros,
            },
            Effect::SetUtf8String {
                window: root,
                atom: atoms.desktop_names,
                value: desktop_names,
            },
            Effect::SetWindowProperty {
                window: root,
                atom: atoms.client_list,
                values: vec![],
            },
            Effect::SetWindowProperty {
                window: root,
                atom: atoms.client_list_stacking,
                values: vec![],
            },
            Effect::SetWindowProperty {
                window: root,
                atom: atoms.active_window,
                values: vec![],
            },
        ]
    }

    pub fn desktop_geometry_effect(&self, width: u32, height: u32) -> Effect {
        Effect::SetCardinal32List {
            window: self.root,
            atom: self.atoms.desktop_geometry,
            values: vec![width, height],
        }
    }

    pub fn workarea_effect(&self, x: u32, y: u32, w: u32, h: u32) -> Effect {
        let mut values = Vec::with_capacity(NUM_WORKSPACES * 4);
        for _ in 0..NUM_WORKSPACES {
            values.extend_from_slice(&[x, y, w, h]);
        }

        Effect::SetCardinal32List {
            window: self.root,
            atom: self.atoms.workarea,
            values,
        }
    }

    pub fn active_window_effect(&self, window: Option<Window>) -> Effect {
        Effect::SetWindowProperty {
            window: self.root,
            atom: self.atoms.active_window,
            values: window.map(|w| vec![w.resource_id()]).unwrap_or_default(),
        }
    }

    pub fn client_list_effects(&self, windows: &[Window]) -> Vec<Effect> {
        let values = windows
            .iter()
            .map(xcb::Xid::resource_id)
            .collect::<Vec<_>>();
        vec![
            Effect::SetWindowProperty {
                window: self.root,
                atom: self.atoms.client_list,
                values: values.clone(),
            },
            Effect::SetWindowProperty {
                window: self.root,
                atom: self.atoms.client_list_stacking,
                values,
            },
        ]
    }

    pub fn current_desktop_effect(&self, current_workspace: usize) -> Effect {
        Effect::SetCardinal32 {
            window: self.root,
            atom: self.atoms.current_desktop,
            value: current_workspace as u32,
        }
    }

    pub fn window_desktop_effect(&self, window: Window, workspace: u32) -> Effect {
        Effect::SetCardinal32 {
            window,
            atom: self.atoms.wm_desktop,
            value: workspace,
        }
    }

    pub fn get_window_desktop(&self, x11: &X11, window: Window) -> Option<u32> {
        x11.get_cardinal32(window, self.atoms.wm_desktop)
    }

    pub fn get_current_desktop(&self, x11: &X11) -> Option<u32> {
        x11.get_cardinal32(self.root, self.atoms.current_desktop)
    }

    pub fn window_fullscreen_state_effect(&self, window: Window, fullscreen: bool) -> Effect {
        let atoms = &self.atoms;
        Effect::SetAtomList {
            window,
            atom: atoms.wm_state,
            values: if fullscreen {
                vec![atoms.wm_state_fullscreen.resource_id()]
            } else {
                vec![]
            },
        }
    }
}
