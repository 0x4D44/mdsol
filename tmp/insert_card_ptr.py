from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "    let Some(animation) = state.win_anim.as_mut() else {\n        return;\n    };\n"
if old not in text:
    raise SystemExit('win_anim as_mut block not found')
new = "    let card_dc = state.card_dc;\n    let card_image_ptr = state.card.as_ref().map(|img| img as *const CardImage);\n    let Some(animation) = state.win_anim.as_mut() else {\n        return;\n    };\n"
path.write_text(text.replace(old, new, 1))
