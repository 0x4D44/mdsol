from pathlib import Path
text = Path(r"C:\language\mdsol\src\main.rs").read_text().splitlines()
for idx, line in enumerate(text, start=1):
    if line.strip().startswith("fn flush_pending("):
        print(idx)
