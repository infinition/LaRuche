import * as vscode from 'vscode';

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
    --amber-dim: rgba(245,158,11,.12);
    --green: #22c55e;
    --bg: #1e1e1e;
    --bg-input: #2d2d2d;
    --bg-msg-user: #1e3a2e;
    --bg-msg-ai: #1c1917;
    --bg-toolbar: #252525;
    --text: #cccccc;
    --text-dim: #777;
    --border: #3e3e3e;
    --code-bg: #0d0d0d;
}
* { margin:0; padding:0; box-sizing:border-box; }
body {
    font-family: var(--vscode-font-family,'Segoe UI',sans-serif);
    font-size: 13px;
    background: var(--bg);
    color: var(--text);
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

/* ── Header ── */
.header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
    color: var(--amber);
    font-size: 13px;
    flex-shrink: 0;
    background: linear-gradient(135deg, #252525, #1e1e1e);
}
.hex {
    width: 18px; height: 18px;
    background: var(--amber);
    clip-path: polygon(50% 0%,100% 25%,100% 75%,50% 100%,0% 75%,0% 25%);
    flex-shrink: 0;
}
.header-right { margin-left: auto; display: flex; align-items: center; gap: 6px; }
.status-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: var(--green);
    animation: pulse 2s infinite;
}
.status-dot.offline { background: #ef4444; }
#status-text { font-size: 10px; color: var(--text-dim); font-weight: 400; }

/* ── Toolbar (node/model selector) ── */
.toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-toolbar);
    flex-shrink: 0;
    flex-wrap: wrap;
}
.toolbar-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    font-size: 10px;
    color: var(--text-dim);
    cursor: pointer;
    transition: border-color .15s, color .15s;
    white-space: nowrap;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
}
.toolbar-pill:hover { border-color: var(--amber); color: var(--amber); }
.toolbar-pill .icon { opacity: .6; }
.toolbar-sep { color: var(--border); font-size: 16px; line-height: 1; }

/* ── Messages ── */
.messages {
    flex: 1;
    overflow-y: auto;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.messages::-webkit-scrollbar { width: 4px; }
.messages::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

.msg {
    max-width: 95%;
    padding: 10px 13px;
    border-radius: 10px;
    line-height: 1.55;
    word-break: break-word;
}
.msg.user {
    align-self: flex-end;
    background: var(--bg-msg-user);
    border: 1px solid rgba(34,197,94,.2);
    border-bottom-right-radius: 3px;
}
.msg.assistant {
    align-self: flex-start;
    background: var(--bg-msg-ai);
    border: 1px solid var(--border);
    border-bottom-left-radius: 3px;
}
.msg.system-msg {
    align-self: center;
    background: transparent;
    border: 1px dashed var(--border);
    font-size: 11px;
    color: var(--text-dim);
    padding: 4px 10px;
    border-radius: 20px;
}
.msg .meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    font-size: 10px;
    color: var(--text-dim);
    margin-top: 6px;
    padding-top: 5px;
    border-top: 1px solid rgba(255,255,255,.06);
}
.meta span { display: flex; align-items: center; gap: 2px; }
.meta .model-tag { color: var(--amber); }

/* Markdown rendering */
.msg p { margin: 4px 0; }
.msg h1,.msg h2,.msg h3 { color: var(--amber); margin: 8px 0 4px; font-size: 13px; font-weight: 700; }
.msg ul,.msg ol { margin: 4px 0 4px 18px; }
.msg li { margin: 2px 0; }
.msg strong { color: #e5e5e5; font-weight: 600; }
.msg em { color: #d4d4d4; font-style: italic; }
.msg code {
    background: rgba(255,255,255,.08);
    padding: 1px 5px;
    border-radius: 3px;
    font-family: 'Cascadia Code','Fira Code',monospace;
    font-size: 11.5px;
}
.msg pre {
    background: var(--code-bg);
    padding: 10px 12px;
    border-radius: 6px;
    margin: 8px 0;
    overflow-x: auto;
    border: 1px solid #2a2a2a;
    font-family: 'Cascadia Code','Fira Code',monospace;
    font-size: 11.5px;
    line-height: 1.45;
}
.msg pre code { background: none; padding: 0; }

/* Attached file preview */
.file-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    background: var(--amber-dim);
    border: 1px solid rgba(245,158,11,.3);
    border-radius: 4px;
    font-size: 10px;
    color: var(--amber);
    margin-bottom: 6px;
}

/* ── Thinking ── */
.thinking {
    align-self: flex-start;
    padding: 10px 14px;
    background: var(--bg-msg-ai);
    border: 1px solid var(--border);
    border-radius: 10px;
    border-bottom-left-radius: 3px;
    color: var(--amber);
    font-size: 12px;
    display: none;
    font-style: italic;
}
.thinking.visible { display: block; }
.thinking::after {
    content: '…';
    animation: dots 1.5s steps(3,end) infinite;
}
@keyframes dots { 0%,33%{content:'.'} 34%,66%{content:'..'} 67%,100%{content:'…'} }
@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.3} }

/* ── Input area ── */
.input-wrap {
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-toolbar);
}
.mode-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 14px 0;
}
.mode-btn {
    background: transparent;
    color: var(--text-dim);
    border: none;
    padding: 3px 10px;
    font-size: 10px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    transition: all .15s;
    letter-spacing: .3px;
}
.mode-btn.active {
    background: var(--bg-input);
    color: var(--amber);
    border: 1px solid var(--border);
}
.mode-btn:not(.active):hover { color: var(--text); }

.input-row {
    display: flex;
    gap: 6px;
    padding: 6px 14px 10px;
    align-items: flex-end;
}
textarea#input {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 7px 10px;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    resize: none;
    outline: none;
    min-height: 36px;
    max-height: 120px;
    line-height: 1.45;
}
textarea#input:focus { border-color: var(--amber); }
.btn-attach {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text-dim);
    font-size: 15px;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: border-color .15s, color .15s;
    flex-shrink: 0;
    user-select: none;
}
.btn-attach:hover { border-color: var(--amber); color: var(--amber); }
.btn-send {
    background: var(--amber);
    color: #000;
    border: none;
    border-radius: 7px;
    padding: 0 14px;
    font-weight: 700;
    font-size: 13px;
    height: 34px;
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity .15s;
}
.btn-send:hover { opacity: .85; }
.btn-send:disabled { opacity: .35; cursor: not-allowed; }
</style>
</head>
<body>

<div class="header">
    <div class="hex"></div>
    LaRuche
    <div class="header-right">
        <div class="status-dot" id="status-dot"></div>
        <span id="status-text">Connecting…</span>
    </div>
</div>

<div class="toolbar">
    <div class="toolbar-pill" id="node-pill" onclick="requestSelectNode()" title="Click to change node">
        <span class="icon">⬡</span>
        <span id="node-label">discovering…</span>
    </div>
    <span class="toolbar-sep">·</span>
    <div class="toolbar-pill" id="model-pill" onclick="requestSelectModel()" title="Click to change model">
        <span class="icon">◈</span>
        <span id="model-label">default</span>
    </div>
</div>

<div class="messages" id="messages">
    <div class="msg assistant">
        Bienvenue ! Je suis votre assistant LaRuche local.
        <br><br>
        <strong>Chat</strong> — posez-moi une question.<br>
        <strong>Agent</strong> — donnez-moi des instructions sur le fichier actif.<br>
        <strong>📎</strong> — attachez le fichier actif comme contexte.
        <div class="meta"><span>LaRuche v0.2.0</span><span>LAND Protocol</span></div>
    </div>
</div>

<div class="thinking" id="thinking">LaRuche réfléchit</div>

<div class="input-wrap">
    <div class="mode-row">
        <button class="mode-btn active" id="btn-chat" onclick="setMode('chat')">Chat</button>
        <button class="mode-btn" id="btn-edit" onclick="setMode('edit')">Agent (Edit)</button>
    </div>
    <div class="input-row">
        <textarea id="input" rows="1" placeholder="Posez une question…"></textarea>
        <div class="btn-attach" id="btn-attach" onclick="attachFile()" title="Attach active file as context">📎</div>
        <button class="btn-send" id="btn-send" onclick="sendMessage()">→</button>
    </div>
</div>

<script>
const vscode = acquireVsCodeApi();
const messagesEl = document.getElementById('messages');
const inputEl = document.getElementById('input');
const sendBtn = document.getElementById('btn-send');
const thinkingEl = document.getElementById('thinking');
const statusEl = document.getElementById('status-text');
const statusDot = document.getElementById('status-dot');
const nodeLabelEl = document.getElementById('node-label');
const modelLabelEl = document.getElementById('model-label');
const btnChat = document.getElementById('btn-chat');
const btnEdit = document.getElementById('btn-edit');

let currentMode = 'chat';
let attachedFile = null; // { fileName, language, content }

// Restore state
const prev = vscode.getState();
if (prev && prev.html) {
    messagesEl.innerHTML = prev.html;
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function saveState() {
    vscode.setState({ html: messagesEl.innerHTML });
}

// ── Mode ──
function setMode(mode) {
    currentMode = mode;
    btnChat.classList.toggle('active', mode === 'chat');
    btnEdit.classList.toggle('active', mode === 'edit');
    inputEl.placeholder = mode === 'chat'
        ? 'Posez une question…'
        : 'Instructions pour le fichier actif…';
}

// ── Auto-resize textarea ──
inputEl.addEventListener('input', () => {
    inputEl.style.height = 'auto';
    inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
});

// ── Enter to send ──
inputEl.addEventListener('keydown', e => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); }
});

// ── Send message ──
function sendMessage() {
    const text = inputEl.value.trim();
    if (!text) return;

    // Build user message display
    let displayContent = text;
    if (attachedFile) {
        displayContent =
            \`<div class="file-badge">📎 \${escHtml(attachedFile.fileName)}</div>\` +
            escHtml(text);
    } else {
        displayContent = escHtml(text);
    }
    addRawMessage('user', displayContent);

    inputEl.value = '';
    inputEl.style.height = 'auto';
    sendBtn.disabled = true;
    thinkingEl.classList.add('visible');

    const payload = {
        type: 'ask',
        mode: currentMode,
        prompt: attachedFile
            ? text + '\\n\\nContext (file: ' + attachedFile.fileName + '):\\n\`\`\`' + attachedFile.language + '\\n' + attachedFile.content + '\\n\`\`\`'
            : text,
    };

    attachedFile = null; // consume
    document.getElementById('btn-attach').style.color = '';
    document.getElementById('btn-attach').title = 'Attach active file as context';

    vscode.postMessage(payload);
}

// ── Attach file ──
function attachFile() {
    vscode.postMessage({ type: 'attachFile' });
}

// ── Node / Model picker ──
function requestSelectNode() { vscode.postMessage({ type: 'selectNode' }); }
function requestSelectModel() { vscode.postMessage({ type: 'selectModel' }); }

// ── Add message helpers ──
function addRawMessage(role, htmlContent, metaHtml) {
    const div = document.createElement('div');
    div.className = 'msg ' + role;
    div.innerHTML = htmlContent;
    if (metaHtml) {
        const m = document.createElement('div');
        m.className = 'meta';
        m.innerHTML = metaHtml;
        div.appendChild(m);
    }
    messagesEl.appendChild(div);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    saveState();
    return div;
}

function addMessage(role, text, metaHtml) {
    addRawMessage(role, renderMarkdown(text), metaHtml);
}

function addSystemMessage(text) {
    const div = document.createElement('div');
    div.className = 'msg system-msg';
    div.textContent = text;
    messagesEl.appendChild(div);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    saveState();
}

// ── Markdown renderer ──
function renderMarkdown(text) {
    // Fenced code blocks
    text = text.replace(/\`\`\`([\\w-]*)\\n?([\\s\\S]*?)\`\`\`/g, (_, lang, code) =>
        '<pre><code>' + escHtml(code.trim()) + '</code></pre>'
    );
    // Inline code
    text = text.replace(/\`([^\`\\n]+)\`/g, (_, c) => '<code>' + escHtml(c) + '</code>');
    // Headers
    text = text.replace(/^### (.+)$/gm, '<h3>$1</h3>');
    text = text.replace(/^## (.+)$/gm, '<h2>$1</h2>');
    text = text.replace(/^# (.+)$/gm, '<h1>$1</h1>');
    // Bold / italic
    text = text.replace(/\\*\\*(.+?)\\*\\*/g, '<strong>$1</strong>');
    text = text.replace(/\\*(.+?)\\*/g, '<em>$1</em>');
    // Unordered lists
    text = text.replace(/^[-*] (.+)$/gm, '<li>$1</li>');
    text = text.replace(/(<li>.*<\\/li>)/s, '<ul>$1</ul>');
    // Line breaks for plain paragraphs (not inside block elements)
    text = text.replace(/([^>])\\n([^<])/g, '$1<br>$2');
    return text;
}

function escHtml(s) {
    return s
        .replace(/&/g,'&amp;')
        .replace(/</g,'&lt;')
        .replace(/>/g,'&gt;')
        .replace(/"/g,'&quot;');
}

// ── Handle messages from extension ──
window.addEventListener('message', event => {
    const msg = event.data;
    switch (msg.type) {
        case 'response':
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', msg.text,
                [
                    msg.model ? \`<span class="model-tag">◈ \${escHtml(msg.model)}</span>\` : '',
                    msg.tokens ? \`<span>\${msg.tokens} tokens</span>\` : '',
                    msg.latency ? \`<span>\${(msg.latency/1000).toFixed(1)}s</span>\` : '',
                    msg.node ? \`<span>⬡ \${escHtml(msg.node)}</span>\` : '',
                ].filter(Boolean).join('')
            );
            break;

        case 'error':
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', '⚠ Erreur : ' + msg.text);
            break;

        case 'status':
            statusEl.textContent = msg.text;
            statusDot.classList.toggle('offline', msg.text.toLowerCase().includes('offline'));
            break;

        case 'agentDone':
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addSystemMessage('✓ ' + (msg.text || 'Agent terminé'));
            break;

        case 'fileAttached':
            attachedFile = { fileName: msg.fileName, language: msg.language, content: msg.content };
            document.getElementById('btn-attach').style.color = 'var(--amber)';
            document.getElementById('btn-attach').title = 'Attached: ' + msg.fileName + ' — click to change';
            addSystemMessage('📎 ' + msg.fileName + ' attaché comme contexte');
            break;

        case 'nodesUpdate':
            updateToolbar(msg.nodes, msg.activeNodeUrl, msg.activeModel);
            break;

        case 'context':
            inputEl.value = msg.text;
            inputEl.style.height = 'auto';
            inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
            inputEl.focus();
            break;
    }
});

function updateToolbar(nodes, activeUrl, activeModel) {
    // Node pill
    let nodeName = activeUrl || 'none';
    if (nodes && nodes.length > 0) {
        const active = nodes.find(n => n.url === activeUrl);
        nodeName = active ? active.name : (nodes[0].name || activeUrl);
        statusDot.classList.remove('offline');
        statusEl.textContent = nodes.length + ' node' + (nodes.length > 1 ? 's' : '');
    } else if (!activeUrl) {
        nodeName = 'offline';
        statusDot.classList.add('offline');
        statusEl.textContent = 'offline';
    }
    // Truncate
    nodeLabelEl.textContent = nodeName.length > 18 ? nodeName.slice(0,15) + '…' : nodeName;
    nodeLabelEl.title = activeUrl || nodeName;

    // Model pill
    modelLabelEl.textContent = activeModel || 'default';
    modelLabelEl.title = activeModel || 'Using node default model';
}
</script>
</body>
</html>`;
}
