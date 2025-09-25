from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("    fn flush_pending")
end = text.index("    }\n}\n\nimpl Drop", start)
old = text[start:end+6]
print(repr(old))
