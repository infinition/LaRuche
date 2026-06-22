import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# We need to extract the nested @media block.
nested_block = """@media (max-width: 900px) {
  .chat-sidebar, .mem2-side, .mis-side {
    position: fixed !important; left: 0; top: 0; bottom: 0; z-index: 120 !important;
    width: 90% !important; max-width: none !important;
    transform: translateX(-100%);
    transition: transform var(--transition-med);
    box-shadow: 4px 0 24px rgba(0,0,0,0.6);
    background: var(--bg-panel);
    border-right: 1px solid var(--border) !important;
  }
  .chat-sidebar.open, .mem2-side.open, .mis-side.open {
    transform: translateX(0);
  }
  .mis-layout, .mem2-layout {
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .mem2-main, .mem2-detail, .mis-dossier-body {
    flex: 1;
    min-height: 50vh;
  }
}"""

if nested_block in content:
    # Remove the nested block from its current location
    content = content.replace('\n' + nested_block, '')
    
    # Add it just before </style>
    content = content.replace('</style>', nested_block + '\n</style>')

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print("CSS un-nested successfully.")
else:
    print("Nested block not found. Trying regex.")
    # Fallback if there are minor whitespace differences
    match = re.search(r'(@media \(max-width: 900px\) \{\s*\.chat-sidebar, \.mem2-side, \.mis-side \{.*?\n\})', content, re.DOTALL)
    if match:
        block = match.group(1)
        content = content.replace('\n' + block, '')
        content = content.replace('</style>', block + '\n</style>')
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print("CSS un-nested successfully via regex.")
    else:
        print("Could not find the block to un-nest.")
