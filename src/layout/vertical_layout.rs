use crate::layout::{Layout, Rect};

pub struct VerticalLayout;

impl Layout for VerticalLayout {
    fn generate_layout(
        &self,
        area: Rect,
        weights: &[u32],
        border_width: u32,
        window_gap: u32,
    ) -> Vec<Rect> {
        let total_weights: u32 = weights.iter().sum();
        let total_border = border_width + window_gap;
        let inner_h = (area.h - 2 * total_border).max(1);
        let partitions = area.w / total_weights;

        let mut cumulative = 0u32;
        let layout: Vec<Rect> = weights
            .iter()
            .map(|weight| {
                let cell = (area.w * weight) / total_weights;
                let inner_w = (cell - 2 * total_border).max(1);
                let x = cumulative * partitions + window_gap;
                cumulative += weight;
                Rect {
                    x: x as i32,
                    y: window_gap as i32,
                    w: inner_w,
                    h: inner_h,
                }
            })
            .collect();
        layout
    }
}
