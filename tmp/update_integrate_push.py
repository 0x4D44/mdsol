from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "        let pos = (new_x, new_y);\n        anim.clones.push(ClassicClone {\n            card: emitter.card,\n            pos,\n        });\n        emitter.pos = pos;\n\n"
if old not in text:
    raise SystemExit('pattern not found for integrate push')
new = "        let pos = (new_x, new_y);\n        anim.record_clone(emitter.card, pos);\n        emitter.pos = pos;\n\n"
path.write_text(text.replace(old, new, 1))
