from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text()
first = text.index("VictoryAnimation::Classic(anim) => anim.emitted_from(index)")
second = text.index("VictoryAnimation::Classic(anim) => {", first + 1)
print(text[second:second+500])
