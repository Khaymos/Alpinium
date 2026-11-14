//! Pure window-layout calculations used by the compositor.
//!
//! This module deliberately has no Smithay dependency so layout policy can be
//! tested independently from the Wayland backend.

/// A rectangle in logical output coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl LayoutRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Calculate a master-and-stack layout.
///
/// The first window receives the master area on the left. Remaining windows
/// are stacked vertically on the right. Gaps are clamped so ordinary small
/// outputs never produce negative dimensions.
pub fn master_stack_layout(
    area: LayoutRect,
    window_count: usize,
    outer_gap: u32,
    inner_gap: u32,
) -> Vec<LayoutRect> {
    if window_count == 0 {
        return Vec::new();
    }

    let width = area.width.max(1);
    let height = area.height.max(1);
    let requested_outer = i32::try_from(outer_gap).unwrap_or(i32::MAX);
    let outer_x = requested_outer.min((width - 1) / 2);
    let outer_y = requested_outer.min((height - 1) / 2);

    let x = area.x.saturating_add(outer_x);
    let y = area.y.saturating_add(outer_y);
    let usable_width = (width - outer_x * 2).max(1);
    let usable_height = (height - outer_y * 2).max(1);

    if window_count == 1 {
        return vec![LayoutRect::new(x, y, usable_width, usable_height)];
    }

    // A master/stack split needs at least one pixel for each column. On an
    // extremely narrow output, degrade gracefully to a single vertical stack.
    if usable_width < 2 {
        return vertical_stack(x, y, usable_width, usable_height, window_count, inner_gap);
    }

    let requested_inner = i32::try_from(inner_gap).unwrap_or(i32::MAX);
    let column_gap = requested_inner.min((usable_width - 2).max(0));
    let content_width = usable_width - column_gap;
    let master_width = (((content_width as i64) * 55) / 100) as i32;
    let master_width = master_width.clamp(1, content_width - 1);
    let stack_width = content_width - master_width;
    let stack_x = x.saturating_add(master_width).saturating_add(column_gap);

    let mut placements = Vec::with_capacity(window_count);
    placements.push(LayoutRect::new(x, y, master_width, usable_height));
    placements.extend(vertical_stack(
        stack_x,
        y,
        stack_width,
        usable_height,
        window_count - 1,
        inner_gap,
    ));
    placements
}

fn vertical_stack(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    count: usize,
    requested_gap: u32,
) -> Vec<LayoutRect> {
    if count == 0 {
        return Vec::new();
    }

    let count_i32 = i32::try_from(count).unwrap_or(i32::MAX).max(1);
    let requested_gap = i32::try_from(requested_gap).unwrap_or(i32::MAX);
    let max_gap = if count_i32 > 1 {
        (height.saturating_sub(count_i32).max(0)) / (count_i32 - 1)
    } else {
        0
    };
    let gap = requested_gap.min(max_gap);
    let total_gap = gap.saturating_mul(count_i32.saturating_sub(1));
    let distributable_height = height.saturating_sub(total_gap).max(0);
    let base_height = distributable_height / count_i32;
    let remainder = distributable_height % count_i32;

    let mut placements = Vec::with_capacity(count);
    let mut next_y = y;

    for index in 0..count_i32 {
        let extra_pixel = if index < remainder { 1 } else { 0 };
        let tile_height = base_height + extra_pixel;
        placements.push(LayoutRect::new(x, next_y, width, tile_height));
        next_y = next_y.saturating_add(tile_height).saturating_add(gap);
    }

    placements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_windows_produces_no_placements() {
        assert!(master_stack_layout(LayoutRect::new(0, 0, 1280, 720), 0, 8, 8).is_empty());
    }

    #[test]
    fn one_window_uses_the_output_inside_outer_gaps() {
        let result = master_stack_layout(LayoutRect::new(0, 0, 1280, 720), 1, 20, 10);

        assert_eq!(result, vec![LayoutRect::new(20, 20, 1240, 680)]);
    }

    #[test]
    fn two_windows_split_into_master_and_stack_columns() {
        let result = master_stack_layout(LayoutRect::new(0, 0, 1200, 800), 2, 20, 10);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], LayoutRect::new(20, 20, 632, 760));
        assert_eq!(result[1], LayoutRect::new(662, 20, 518, 760));
    }

    #[test]
    fn additional_windows_share_the_stack_height_evenly() {
        let result = master_stack_layout(LayoutRect::new(0, 0, 1200, 800), 4, 20, 10);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], LayoutRect::new(20, 20, 632, 760));
        assert_eq!(result[1], LayoutRect::new(662, 20, 518, 247));
        assert_eq!(result[2], LayoutRect::new(662, 277, 518, 247));
        assert_eq!(result[3], LayoutRect::new(662, 534, 518, 246));
    }

    #[test]
    fn output_origin_is_preserved() {
        let result = master_stack_layout(LayoutRect::new(100, 50, 800, 600), 1, 12, 8);

        assert_eq!(result, vec![LayoutRect::new(112, 62, 776, 576)]);
    }

    #[test]
    fn tiny_outputs_do_not_produce_negative_dimensions() {
        let result = master_stack_layout(LayoutRect::new(0, 0, 2, 2), 5, 50, 50);

        assert_eq!(result.len(), 5);
        assert!(
            result
                .iter()
                .all(|rect| rect.width >= 0 && rect.height >= 0)
        );
    }

    #[test]
    fn very_large_gaps_are_clamped_to_the_output() {
        let result = master_stack_layout(LayoutRect::new(0, 0, 100, 60), 2, u32::MAX, u32::MAX);

        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .all(|rect| rect.width >= 1 && rect.height >= 1)
        );
        assert!(result[0].x >= 0 && result[0].y >= 0);
    }
}
