from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
insertion_point = text.index("fn draw_round_outline")
end_of_outline = text.index("}\n\n", insertion_point) + 3
helper_functions = """fn draw_card_placeholder_dc(dc: HDC, metrics: &CardMetrics, x: i32, y: i32) {
    let rect = make_rect(x, y, metrics.card_w, metrics.card_h);
    let radius = (metrics.card_w.min(metrics.card_h) / 6).max(6);
    draw_round_rect_fill(dc, rect, radius, rgb(8, 96, 24), rgb(0, 0, 0));
    let inner = inset_rect(rect, 3);
    draw_round_outline(dc, inner, (radius - 2).max(4), rgb(0, 0, 0), 1);
}

fn draw_card_face_up_to_dc(
    state: &WindowState,
    metrics: &CardMetrics,
    target_dc: HDC,
    card: &Card,
    x: i32,
    y: i32,
) {
    let rect = make_rect(x, y, metrics.card_w, metrics.card_h);
    unsafe {
        if let (Some(image), true) = (state.card.as_ref(), state.card_dc.0 != 0) {
            let radius = (metrics.card_w.min(metrics.card_h) / 6).max(6);
            draw_round_rect_fill(
                target_dc,
                rect,
                radius,
                rgb(252, 252, 252),
                rgb(204, 204, 204),
            );
            let sprite = card.sprite_index as i32;
            let src_x = (sprite % CARD_SPRITE_COLS) * image.cell_w;
            let src_y = (sprite / CARD_SPRITE_COLS) * image.cell_h;
            let trim_x = 1;
            let trim_y = 1;
            let src_w = (image.cell_w - trim_x * 2).max(1);
            let src_h = (image.cell_h - trim_y * 2).max(1);
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let max_inset_w = ((rect.right - rect.left) / 2).saturating_sub(1);
            let max_inset_h = ((rect.bottom - rect.top) / 2).saturating_sub(1);
            let face_gap = (metrics.card_w.min(metrics.card_h) / 32).max(2);
            let inset = metrics
                .face_inset
                .saturating_add(face_gap)
                .min(max_inset_w)
                .min(max_inset_h)
                .max(0);
            let inner = if inset > 0 {
                inset_rect(rect, inset)
            } else {
                rect
            };
            let dest_w = (inner.right - inner.left).max(0);
            let dest_h = (inner.bottom - inner.top).max(0);
            if dest_w > 0 && dest_h > 0 {
                AlphaBlend(
                    target_dc,
                    inner.left,
                    inner.top,
                    dest_w,
                    dest_h,
                    state.card_dc,
                    src_x + trim_x,
                    src_y + trim_y,
                    src_w,
                    src_h,
                    blend,
                );
            }
        } else {
            draw_card_placeholder_dc(target_dc, metrics, x, y);
        }
    }
}

"""
updated = text[:end_of_outline] + helper_functions + text[end_of_outline:]
path.write_text(updated)
