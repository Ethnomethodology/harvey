import re

with open("src-tauri/src/projectview/core_commands.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i in range(len(lines) - 20, len(lines)):
    print(f"{i + 1}: {lines[i]}")
