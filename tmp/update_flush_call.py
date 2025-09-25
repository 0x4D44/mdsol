from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "            anim.flush_pending(state, &metrics);\n\n            anim.next_emit >= anim.emitters.len()\n"
if old not in text:
    raise SystemExit('flush_pending call not found')
new = "            let card_image = unsafe { card_image_ptr.map(|ptr| &*ptr) };\n            anim.flush_pending(card_image, card_dc, &metrics);\n\n            anim.next_emit >= anim.emitters.len()\n"
path.write_text(text.replace(old, new, 1))
