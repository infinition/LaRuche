import * as vscode from 'vscode';

interface ChatMessage {
    role: 'user' | 'assistant' | 'system';
    content: string;
    model?: string;
    latency?: number;
    tokens?: number;
}

export function getChatHtml(webview: vscode.Webview): string {
    return /*html*/`<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline';">
<style>
:root {
    --amber: #f59e0b;
    --bg: #1e1e1e;
    --bg-input: #2d2d2d;
    --bg-msg-user: #2b4a3e;
    --bg-msg-ai: #1c1917;
    --text: #cccccc;
    --text-dim: #888;
    --border: #3e3e3e;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
    font-family: var(--vscode-font-family, 'Segoe UI', sans-serif);
    font-size: 13px;
    background: var(--bg);
    color: var(--text);
    height: 100vh;
    display: flex;
    flex-direction: column;
}
.header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
    color: var(--amber);
    font-size: 14px;
    flex-shrink: 0;
}
.header .hex {
    width: 20px; height: 20px;
    background: var(--amber);
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
}
.status {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-dim);
    font-weight: 400;
}
.status .dot {
    display: inline-block;
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #22c55e;
    margin-right: 4px;
    animation: pulse 2s infinite;
}
@keyframes pulse { 0%,100% { opacity:1; } 50% { opacity:0.3; } }
.messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.msg {
    max-width: 90%;
    padding: 10px 14px;
    border-radius: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
}
.msg.user {
    align-self: flex-end;
    background: var(--bg-msg-user);
    border-bottom-right-radius: 4px;
}
.msg.assistant {
    align-self: flex-start;
    background: var(--bg-msg-ai);
    border: 1px solid var(--border);
    border-bottom-left-radius: 4px;
}
.msg .meta {
    font-size: 10px;
    color: var(--text-dim);
    margin-top: 6px;
    display: flex;
    gap: 8px;
}
.msg code {
    background: rgba(255,255,255,0.08);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: 'Fira Code', 'Cascadia Code', monospace;
    font-size: 12px;
}
.msg pre {
    background: #0d0d0d;
    padding: 10px;
    border-radius: 6px;
    margin: 8px 0;
    overflow-x: auto;
    font-family: 'Fira Code', 'Cascadia Code', monospace;
    font-size: 12px;
    line-height: 1.4;
}
.thinking {
    align-self: flex-start;
    padding: 10px 14px;
    background: var(--bg-msg-ai);
    border: 1px solid var(--border);
    border-radius: 12px;
    border-bottom-left-radius: 4px;
    color: var(--amber);
    font-size: 12px;
    display: none;
}
.thinking.visible { display: block; }
.thinking::after {
    content: '...';
    animation: dots 1.5s infinite;
}
@keyframes dots {
    0% { content: '.'; }
    33% { content: '..'; }
    66% { content: '...'; }
}
.input-area {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
}
.input-area textarea {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    resize: none;
    outline: none;
    min-height: 38px;
    max-height: 120px;
}
.input-area textarea:focus {
    border-color: var(--amber);
}
.input-area button {
    background: var(--amber);
    color: #000;
    border: none;
    border-radius: 8px;
    padding: 0 16px;
    font-weight: 700;
    font-size: 13px;
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity 0.2s;
}
.input-area button:hover { opacity: 0.85; }
.input-area button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
</head>
<body>
<div class="header">
    <div class="hex"></div>
    LaRuche Chat
    <span class="status"><span class="dot"></span><span id="status-text">Connecting...</span></span>
</div>
<div class="messages" id="messages">
    <div class="msg assistant">Bienvenue! Je suis votre assistant LaRuche local. Posez-moi une question ou envoyez du code a analyser.
        <div class="meta"><span>LaRuche v0.1.0</span><span>LAND Protocol</span></div>
    </div>
</div>
<div class="thinking" id="thinking">LaRuche reflechit</div>
<div class="input-area">
    <textarea id="input" placeholder="Posez une question..." rows="1"></textarea>
    <button id="send" onclick="sendMessage()">Envoyer</button>
</div>
<script>
const vscode = acquireVsCodeApi();
const messagesEl = document.getElementById('messages');
const inputEl = document.getElementById('input');
const sendBtn = document.getElementById('send');
const thinkingEl = document.getElementById('thinking');
const statusEl = document.getElementById('status-text');

// Auto-resize textarea
inputEl.addEventListener('input', () => {
    inputEl.style.height = 'auto';
    inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
});

// Enter to send (Shift+Enter for newline)
inputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
    }
});

function sendMessage() {
    const text = inputEl.value.trim();
    if (!text) return;
    
    addMessage('user', text);
    inputEl.value = '';
    inputEl.style.height = 'auto';
    sendBtn.disabled = true;
    thinkingEl.classList.add('visible');
    
    vscode.postMessage({ type: 'ask', prompt: text });
}

function addMessage(role, content, meta) {
    const div = document.createElement('div');
    div.className = 'msg ' + role;
    
    // Basic markdown-like formatting
    let html = content
        .replace(/\`\`\`(\\w*)?\\n?([\\s\\S]*?)\`\`\`/g, '<pre>$2</pre>')
        .replace(/\`([^\`]+)\`/g, '<code>$1</code>');
    
    div.innerHTML = html;
    
    if (meta) {
        const metaDiv = document.createElement('div');
        metaDiv.className = 'meta';
        metaDiv.innerHTML = meta;
        div.appendChild(metaDiv);
    }
    
    messagesEl.appendChild(div);
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

// Handle messages from extension
window.addEventListener('message', event => {
    const msg = event.data;
    switch(msg.type) {
        case 'response':
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            const meta = [
                msg.model ? '<span>' + msg.model + '</span>' : '',
                msg.tokens ? '<span>' + msg.tokens + ' tokens</span>' : '',
                msg.latency ? '<span>' + (msg.latency / 1000).toFixed(1) + 's</span>' : '',
                msg.node ? '<span>' + msg.node + '</span>' : '',
            ].filter(Boolean).join('');
            addMessage('assistant', msg.text, meta);
            break;
        case 'error':
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', 'Erreur: ' + msg.text);
            break;
        case 'status':
            statusEl.textContent = msg.text;
            break;
        case 'context':
            inputEl.value = msg.text;
            inputEl.style.height = 'auto';
            inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
            inputEl.focus();
            break;
    }
});
</script>
</body>
</html>`;
}
