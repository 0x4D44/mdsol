from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "            VictoryAnimation::Classic(ClassicVictoryAnimation::new(\n                emitters,\n                metrics.card_h as f32,\n                metrics.card_w as f32,\n                width.max(1) as f32,\n                now,\n            ))\n"
new = "            VictoryAnimation::Classic(ClassicVictoryAnimation::new(\n                emitters,\n                metrics.card_h as f32,\n                metrics.card_w as f32,\n                width.max(1) as f32,\n                (width.max(1), height.max(1)),\n                now,\n            ))\n"
if old not in text:
    raise SystemExit('pattern not found for ClassicVictoryAnimation::new call')
path.write_text(text.replace(old, new, 1))
