use indexmap::IndexMap;
use log::{debug, error};

use crate::{
    config::DEFAULT_LAYOUT,
    layout::{master_layout::MasterLayout, horizontal_layout::HorizontalLayout},
};

pub mod master_layout;
pub mod horizontal_layout;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub trait Layout {
    fn generate_layout(
        &self,
        area: Rect,
        weights: &[u32],
        border_width: u32,
        window_gap: u32,
    ) -> Vec<Rect>;
}

macro_rules! define_layouts {
    ( $( $variant:ident => $ty:path ),+ $(,)? ) => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
        pub enum LayoutType {
            $( $variant ),+
        }

        fn build_layout_map() -> IndexMap<LayoutType, Box<dyn Layout>> {
            let mut map: IndexMap<LayoutType, Box<dyn Layout>> = IndexMap::default();
            $( map.insert(LayoutType::$variant, Box::new($ty)); )+
            map
        }
    };
}

define_layouts! {
    HorizontalLayout => HorizontalLayout,
    MasterLayout => MasterLayout,
}

pub(super) fn pad(dim: u32, border: u32) -> u32 {
    (dim - 2 * border).max(1)
}

pub struct LayoutManager {
    layout_map: IndexMap<LayoutType, Box<dyn Layout>>,
    current_layout: LayoutType,
}

impl LayoutManager {
    pub fn new() -> Self {
        let map = build_layout_map();

        if map.is_empty() {
            panic!(
                "No layouts defined, layouts need to be defined in layout/mod.rs using the define_layouts! macro."
            )
        }

        let current_layout = if map.contains_key(&DEFAULT_LAYOUT) {
            DEFAULT_LAYOUT
        } else {
            // This shouldn't be possible
            error!("Layout {DEFAULT_LAYOUT:?} not defined in LayoutType.");
            map.get_index(0).map(|(key, _)| *key).unwrap()
        };

        LayoutManager {
            layout_map: map,
            current_layout,
        }
    }

    pub fn get_current_layout(&self) -> &dyn Layout {
        self.layout_map
            .get(&self.current_layout)
            .map(|layout| layout.as_ref())
            .unwrap()
    }

    pub fn set_layout(&mut self, layout: LayoutType) {
        if self.layout_map.contains_key(&layout) {
            self.current_layout = layout
        }
    }

    pub fn cycle_layout(&mut self) {
        if let Some(current_idx) = self.layout_map.get_index_of(&self.current_layout) {
            let next_idx = (current_idx + 1) % self.layout_map.len();
            if let Some(layout) = self.layout_map.get_index(next_idx).map(|(key, _)| *key) {
                debug!("New layout activated: {layout:?}");
                self.current_layout = layout
            } else {
                error!("Failed to cycle layout");
            }
        }
    }
}
