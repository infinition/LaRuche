import re

filepath = r"C:\Users\infinition\Desktop\laruche-v2\laruche\laruche-cli\src\ui\mod.rs"

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Make the structs and enums public
content = re.sub(r'enum TuiEvent', r'pub enum TuiEvent', content)
content = re.sub(r'struct ChatMessage', r'pub struct ChatMessage', content)
content = re.sub(r'struct App', r'pub struct App', content)
content = re.sub(r'enum SidebarPanel', r'pub enum SidebarPanel', content)
content = re.sub(r'enum ChatView', r'pub enum ChatView', content)
content = re.sub(r'enum Panel', r'pub enum Panel', content)

# Make fields public for App and ChatMessage (simplified)
content = content.replace("role: String,", "pub role: String,")
content = content.replace("text: String,", "pub text: String,")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Made types public.")
