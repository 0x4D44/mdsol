from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("let draw_placeholder = |dc: HDC, x: i32, y: i32|")
end = text.index("            let draw_face_down", start)
old = text[start:end]
new = "let draw_placeholder = |dc: HDC, x: i32, y: i32| {\n                draw_card_placeholder_dc(dc, &metrics, x, y);\n            };\n\n            let draw_face_up = |card: &Card, x: i32, y: i32| {\n                draw_card_face_up_to_dc(state, &metrics, back.dc, card, x, y);\n            };\n\n"
text = text[:start] + new + text[end:]
Path(r"C:\language\mdsol\src\main.rs").write_text(text)
