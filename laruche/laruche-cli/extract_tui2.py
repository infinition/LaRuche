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

app_prepend = """pub use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio::sync::mpsc;

"""
# Write ui/app.rs (types and App struct) from 23 to 269
write_file("app.rs", 23, 269, app_prepend)

print("Created ui/app.rs")
