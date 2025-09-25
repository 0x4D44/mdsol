from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("fn emit_classic_card")
end = text.index("fn integrate_classic_emitters", start)
print(text[start:end])
