from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
start = text.index("VictoryAnimation::Classic(classic) => {")
end = text.index("                    }")
print(text[start:start+400])
