from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("fn integrate_classic_emitters")
end = text.index("fn update_victory_animation", start)
print(text[start:end])
