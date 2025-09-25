from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("impl ClassicVictoryAnimation {")
end = text.index("enum VictoryAnimation")
old_impl = text[start:end]
print(old_impl)
