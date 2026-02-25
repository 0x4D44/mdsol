# Doc generator
content = open("template.txt", "r").read()
with open("2025.11.06 - Scoring and Undo System.md", "w") as out:
    out.write(content)
