import os

base_dir = r"C:\Users\infinition\Desktop\laruche-v2\laruche\laruche-cli\src"
tui_path = os.path.join(base_dir, "tui.rs")

with open(tui_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

def write_file(filename, start_line, end_line, prepend=""):
    path = os.path.join(base_dir, "ui", filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(prepend)
        f.writelines(lines[start_line-1:end_line])

# Make sure ui directory exists
os.makedirs(os.path.join(base_dir, "ui"), exist_ok=True)

# Write ui/network.rs (lines 271-637)
write_file("network.rs", 271, 637, "use super::app::TuiEvent;\nuse super::app::dirs_config_path;\n\n")

print("Created ui/network.rs")
