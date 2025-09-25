from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("fn flush_pending")
end = text.index("}\n}\n\nimpl Drop for ClassicVictoryAnimation", start)
old = text[start:end]
print(old)
