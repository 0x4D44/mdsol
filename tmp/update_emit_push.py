from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "        anim.clones.push(ClassicClone {\n            card: emitter.card,\n            pos: emitter.pos,\n        });\n"
new = "        anim.record_clone(emitter.card, emitter.pos);\n"
if old not in text:
    raise SystemExit('pattern not found for emit push')
path.write_text(text.replace(old, new, 1))
