from pathlib import Path
path = Path(r"C:\language\mdsol\src\main.rs")
text = path.read_text()
old = "struct ClassicVictoryAnimation {\n    emitters: Vec<ClassicEmitter>,\n    clones: Vec<ClassicClone>,\n    next_emit: usize,\n    emit_timer: f32,\n    accumulator: f32,\n    last_tick: Instant,\n    foundation_emitted: [usize; FOUNDATION_COLUMNS],\n    card_height: f32,\n    card_width: f32,\n    viewport_width: f32,\n}\n\n"
if old not in text:
    raise SystemExit('pattern not found for struct replace')
new = "struct ClassicVictoryAnimation {\n    emitters: Vec<ClassicEmitter>,\n    pending: Vec<ClassicClone>,\n    layer: Option<BackBuffer>,\n    next_emit: usize,\n    emit_timer: f32,\n    accumulator: f32,\n    last_tick: Instant,\n    foundation_emitted: [usize; FOUNDATION_COLUMNS],\n    card_height: f32,\n    card_width: f32,\n    viewport_width: f32,\n    layer_size: (i32, i32),\n}\n\n"
path.write_text(text.replace(old, new, 1))
