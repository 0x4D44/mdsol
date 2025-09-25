from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "        VictoryAnimation::Classic(anim) => {\n            anim.card_height = metrics.card_h as f32;\n            anim.card_width = metrics.card_w as f32;\n            anim.viewport_width = width.max(1) as f32;\n\n            let mut delta = (now - anim.last_tick).as_secs_f32();\n"
if old not in text:
    raise SystemExit('pattern not found for classic branch header')
new = "        VictoryAnimation::Classic(anim) => {\n            anim.card_height = metrics.card_h as f32;\n            anim.card_width = metrics.card_w as f32;\n            anim.viewport_width = width.max(1) as f32;\n            anim.layer_size = (width.max(1), height.max(1));\n\n            let mut delta = (now - anim.last_tick).as_secs_f32();\n"
path.write_text(text.replace(old, new, 1))
