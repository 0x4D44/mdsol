from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("struct ClassicVictoryAnimation")
end = text.index("impl ClassicVictoryAnimation")
old = text[start:end]
print(repr(old))
