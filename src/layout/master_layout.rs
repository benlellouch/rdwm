use crate::layout::{Layout, Rect, pad};

pub struct MasterLayout;

impl Layout for MasterLayout {
    fn generate_layout(
        &self,
        area: Rect,
        weights: &[u32],
        border_width: u32,
        window_gap: u32,
    ) -> Vec<Rect> {
        let total_border = border_width + (window_gap / 2);
        let mut prev_x: u32 = window_gap;
        let mut prev_y: u32 = window_gap;
        let mut prev_h: u32 = area.h - window_gap;
        let mut prev_w: u32 = area.w - window_gap;
        let layout: Vec<Rect> = weights
            .iter()
            .enumerate()
            .map(|(i, _weight)| {
                if weights.len() - 1 == i {
                    Rect {
                        x: prev_x as i32,
                        y: prev_y as i32,
                        w: pad(prev_w, total_border),
                        h: pad(prev_h, total_border),
                    }
                } else if i % 2 == 0 {
                    let inner_w = prev_w / 2;
                    let rect = Rect {
                        x: prev_x as i32,
                        y: prev_y as i32,
                        w: pad(inner_w, total_border),
                        h: pad(prev_h, total_border),
                    };
                    prev_x += inner_w;
                    prev_w = inner_w;
                    rect
                } else {
                    let inner_h = prev_h / 2;
                    let rect = Rect {
                        x: prev_x as i32,
                        y: prev_y as i32,
                        w: pad(prev_w, total_border),
                        h: pad(inner_h, total_border),
                    };
                    prev_y += inner_h;
                    prev_h = inner_h;
                    rect
                }
            })
            .collect();

        layout
    }
}
