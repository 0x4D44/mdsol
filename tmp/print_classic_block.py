from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
second = text.index("VictoryAnimation::Classic(anim) => {", text.index("VictoryAnimation::Classic(anim) => anim.emitted_from(index)") + 1)
print(text[second:second+1600])
