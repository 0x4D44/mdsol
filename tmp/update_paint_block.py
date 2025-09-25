from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "        if let Some(back) = state.back.as_ref() {\n            FillRect(back.dc, &draw_rect, state.bg_brush);\n\n            let draw_placeholder = |dc: HDC, x: i32, y: i32| {\n                draw_card_placeholder_dc(dc, &metrics, x, y);\n            };\n\n            let draw_face_up = |card: &Card, x: i32, y: i32| {\n                draw_card_face_up_to_dc(state, &metrics, back.dc, card, x, y);\n            };\n\n"
if old not in text:
    raise SystemExit('paint_window block not found')
new = "        if let Some(back) = state.back.as_ref() {\n            FillRect(back.dc, &draw_rect, state.bg_brush);\n\n            let card_image = state.card.as_ref();\n            let card_dc = state.card_dc;\n\n            let draw_placeholder = |dc: HDC, x: i32, y: i32| {\n                draw_card_placeholder_dc(dc, &metrics, x, y);\n            };\n\n            let draw_face_up = |card: &Card, x: i32, y: i32| {\n                draw_card_face_up_to_dc(card_image, card_dc, &metrics, back.dc, card, x, y);\n            };\n\n"
path.write_text(text.replace(old, new, 1))
