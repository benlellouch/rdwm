use xcb::{x, Xid};

use crate::{config::NUM_WORKSPACES, rdwm::Effect, x11::X11};

pub struct EwmhManager<'a> {
    x11: &'a X11,
}

impl<'a> EwmhManager<'a> {
    pub const fn new(x11: &'a X11) -> Self {
        Self { x11 }
    }

    pub fn publish_hints(&self) -> Vec<Effect> {
        let atoms = self.x11.atoms();
        let root = self.x11.root();
        let check = self.x11.wm_check_window();

        let supported_atoms = [
            atoms.supported,
            atoms.supporting_wm_check,
            atoms.number_of_desktops,
            atoms.current_desktop,
            atoms.wm_window_type,
            atoms.wm_window_type_dock,
        ];

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
        ]
    }

    pub fn current_desktop_effect(&self, current_workspace: usize) -> Effect {
        Effect::SetCardinal32 {
            window: self.x11.root(),
            atom: self.x11.atoms().current_desktop,
            value: current_workspace as u32,
        }
    }

    pub fn window_desktop_effect(&self, window: x::Window, workspace: u32) -> Effect {
        Effect::SetCardinal32 {
            window,
            atom: self.x11.atoms().wm_desktop,
            value: workspace,
        }
    }

    pub fn get_window_desktop(&self, window: x::Window) -> Option<u32> {
        self.x11.get_cardinal32(window, self.x11.atoms().wm_desktop)
    }

    pub fn get_current_desktop(&self) -> Option<u32> {
        self.x11
            .get_cardinal32(self.x11.root(), self.x11.atoms().current_desktop)
    }
}
