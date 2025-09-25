from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("let draw_placeholder = |dc: HDC")
end = text.index("            let draw_face_down", start)
print(text[start:end])
