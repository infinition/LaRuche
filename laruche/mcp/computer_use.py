import sys
import base64
import pyautogui
from io import BytesIO
from mcp.server.fastmcp import FastMCP, Image

# Configure PyAutoGUI
pyautogui.FAILSAFE = False
pyautogui.PAUSE = 0.5

# Create the MCP server
mcp = FastMCP("laruche-computer-use")

@mcp.tool()
def computer(action: str, coordinate: list[int] = None, text: str = None):
    """Use the computer to interact with the GUI.
    action: One of: key, type, mouse_move, left_click, left_click_drag, right_click, middle_click, double_click, screenshot, cursor_position
    coordinate: [x, y] for mouse_move and click actions.
    text: text to type for 'type' action, or key to press for 'key' action.
    """
    
    if action == "cursor_position":
        x, y = pyautogui.position()
        return f"Cursor position: X={x}, Y={y}"
        
    elif action == "screenshot":
        img = pyautogui.screenshot()
        buffered = BytesIO()
        img.save(buffered, format="PNG")
        return Image(data=buffered.getvalue(), format="png")
        
    elif action == "mouse_move":
        if not coordinate or len(coordinate) != 2:
            return "Error: coordinate [x, y] is required for mouse_move"
        pyautogui.moveTo(coordinate[0], coordinate[1])
        return f"Mouse moved to {coordinate[0]}, {coordinate[1]}"
        
    elif action == "left_click":
        if coordinate and len(coordinate) == 2:
            pyautogui.click(x=coordinate[0], y=coordinate[1], button="left")
            return f"Clicked left at {coordinate[0]}, {coordinate[1]}"
        else:
            pyautogui.click(button="left")
            return "Clicked left at current position"
            
    elif action == "right_click":
        if coordinate and len(coordinate) == 2:
            pyautogui.click(x=coordinate[0], y=coordinate[1], button="right")
            return f"Clicked right at {coordinate[0]}, {coordinate[1]}"
        else:
            pyautogui.click(button="right")
            return "Clicked right at current position"
            
    elif action == "middle_click":
        pyautogui.click(button="middle")
        return "Clicked middle at current position"
        
    elif action == "double_click":
        if coordinate and len(coordinate) == 2:
            pyautogui.doubleClick(x=coordinate[0], y=coordinate[1])
            return f"Double clicked at {coordinate[0]}, {coordinate[1]}"
        else:
            pyautogui.doubleClick()
            return "Double clicked at current position"
            
    elif action == "left_click_drag":
        if not coordinate or len(coordinate) != 2:
            return "Error: coordinate [x, y] is required for left_click_drag"
        pyautogui.dragTo(coordinate[0], coordinate[1], button="left")
        return f"Dragged left to {coordinate[0]}, {coordinate[1]}"
        
    elif action == "type":
        if text is None:
            return "Error: text is required for type action"
        pyautogui.write(text, interval=0.01)
        return f"Typed: {text}"
        
    elif action == "key":
        if text is None:
            return "Error: text (key name) is required for key action"
        keys = text.split('+')
        if len(keys) > 1:
            pyautogui.hotkey(*keys)
        else:
            pyautogui.press(text)
        return f"Pressed key: {text}"
        
    else:
        return f"Error: unknown action {action}"

if __name__ == "__main__":
    mcp.run()
