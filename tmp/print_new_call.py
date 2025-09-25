from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
idx = text.index("VictoryAnimation::Classic(ClassicVictoryAnimation::new(")
print(text[idx:idx+400])
