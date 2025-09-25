from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
pattern_start = text.index("fn emit_classic_card")
pattern_end = text.index("fn integrate_classic_emitters", pattern_start)
block = text[pattern_start:pattern_end]
new_block = "fn emit_classic_card(anim: &mut ClassicVictoryAnimation, index: usize) {\n    let mut clone = None;\n    if let Some(emitter) = anim.emitters.get_mut(index) {\n        emitter.emitted = true;\n        emitter.finished = false;\n        emitter.dy = 0.0;\n        emitter.pos = emitter.start_pos;\n        clone = Some((emitter.card, emitter.pos));\n        if let Some(foundation_idx) = emitter.foundation {\n            let emitted = &mut anim.foundation_emitted[foundation_idx];\n            *emitted = emitted.saturating_add(1);\n        }\n    }\n    if let Some((card, pos)) = clone {\n        anim.record_clone(card, pos);\n    }\n}\n\n"
updated = text[:pattern_start] + new_block + text[pattern_end:]
path.write_text(updated)
