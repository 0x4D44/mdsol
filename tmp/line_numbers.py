from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text().splitlines()
keys = {
    'struct ClassicVictoryAnimation {': 'classic_struct',
    'impl ClassicVictoryAnimation {': 'classic_impl',
    'fn flush_pending(&mut self, card_image: Option<&CardImage>, card_dc: HDC, metrics: &CardMetrics)': 'flush_pending',
    'fn integrate_classic_emitters': 'integrate_classic_emitters',
    'fn emit_classic_card': 'emit_classic_card',
    'VictoryAnimation::Classic(anim) => {': 'update_classic_branch',
    'VictoryAnimation::Classic(classic) => {': 'paint_classic_branch',
    'fn draw_card_placeholder_dc': 'draw_placeholder_helper',
    'fn draw_card_face_up_to_dc': 'draw_face_helper',
    'struct BackBuffer {': 'backbuffer_struct'
}
for idx, line in enumerate(text, start=1):
    stripped = line.strip()
    for key, name in keys.items():
        if stripped.startswith(key):
            print(f"{name}:{idx}")
