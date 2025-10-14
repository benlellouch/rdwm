use crate::key_mapping::ActionMapping;

pub static ACTION_MAPPINGS: &[ActionMapping] = &[
    ActionMapping {
        key: "Return",
        modifiers: &["Mod4"],
        action: "xterm",
    },
    ActionMapping {
        key: "d",
        modifiers: &["Mod4"],
        action: "xeyes",
    },
];
