from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "            while anim.accumulator >= CLASSIC_FIXED_DT {\n                anim.accumulator -= CLASSIC_FIXED_DT;\n                integrate_classic_emitters(anim, floor_y);\n            }\n\n            anim.next_emit >= anim.emitters.len()\n"
if old not in text:
    raise SystemExit('pattern not found for classic flush insertion')
new = "            while anim.accumulator >= CLASSIC_FIXED_DT {\n                anim.accumulator -= CLASSIC_FIXED_DT;\n                integrate_classic_emitters(anim, floor_y);\n            }\n\n            anim.flush_pending(state, &metrics);\n\n            anim.next_emit >= anim.emitters.len()\n"
path.write_text(text.replace(old, new, 1))
