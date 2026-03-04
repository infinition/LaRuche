import * as vscode from 'vscode';

// ── Nonce helper (required for VS Code webview CSP) ──────────────────────────
function getNonce(): string {
    let text = '';
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}

export function getChatHtml(webview: vscode.Webview): string {
    const nonce = getNonce();
    return /*html*/`<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="Content-Security-Policy" content="
    default-src 'none';
    style-src 'unsafe-inline';
    script-src 'nonce-${nonce}';
    img-src data: https: ${webview.cspSource};
    media-src data: https:;
">
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
    --btn-hover: rgba(121,121,121,.31);
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
.header-actions { display: flex; gap: 2px; margin-left: 4px; }
.icon-btn {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all .2s;
}
.icon-btn:hover { background: var(--btn-hover); color: var(--text); }
.icon-btn svg { width: 14px; height: 14px; fill: currentColor; }
.header-right { margin-left: auto; display: flex; align-items: center; gap: 6px; }
.status-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: var(--green);
    animation: pulse 2s infinite;
}
.status-dot.offline { background: #ef4444; animation: none; }
#status-text { font-size: 10px; color: var(--text-dim); font-weight: 400; }

/* ── Toolbar ── */
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

.msg-container {
    display: flex;
    flex-direction: column;
    position: relative;
}
.msg-container.user { align-self: flex-end; max-width: 95%; }
.msg-container.assistant { align-self: flex-start; max-width: 95%; }

.msg {
    padding: 10px 13px;
    border-radius: 10px;
    line-height: 1.55;
    word-break: break-word;
}
.msg.user {
    background: var(--bg-msg-user);
    border: 1px solid rgba(34,197,94,.2);
    border-bottom-right-radius: 3px;
}
.msg.assistant {
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
.copy-btn {
    position: absolute;
    top: 4px; right: 4px;
    opacity: 0;
    pointer-events: none;
    background: var(--bg-input);
    border: 1px solid var(--border);
    padding: 4px;
    border-radius: 4px;
    z-index: 10;
    cursor: pointer;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    justify-content: center;
}
.msg-container:hover .copy-btn { opacity: 1; pointer-events: auto; }
.copy-btn:hover { color: var(--text); }
.copy-btn svg { width: 12px; height: 12px; fill: currentColor; }

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

/* Markdown */
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
.msg .code-block { position: relative; margin: 8px 0; }
.msg .lang-label {
    display: block;
    background: #161616;
    color: var(--text-dim);
    font-size: 10px;
    padding: 3px 10px;
    border-radius: 6px 6px 0 0;
    border: 1px solid #2a2a2a;
    border-bottom: none;
    font-family: 'Cascadia Code','Fira Code',monospace;
    letter-spacing: .5px;
}
.msg pre {
    background: var(--code-bg);
    padding: 10px 12px;
    border-radius: 0 6px 6px 6px;
    overflow-x: auto;
    border: 1px solid #2a2a2a;
    font-family: 'Cascadia Code','Fira Code',monospace;
    font-size: 11.5px;
    line-height: 1.45;
    margin: 0;
}
.msg .code-block:not(:has(.lang-label)) pre { border-radius: 6px; }
.msg pre code { background: none; padding: 0; }
.msg blockquote {
    border-left: 3px solid var(--amber);
    margin: 6px 0;
    padding: 4px 10px;
    color: var(--text-dim);
    font-style: italic;
    background: rgba(245,158,11,.05);
    border-radius: 0 4px 4px 0;
}

/* Attachments */
.attachment-preview { display: flex; flex-wrap: wrap; gap: 6px; padding: 4px 14px 0; }
.attachment-item {
    display: flex; align-items: center; gap: 5px;
    padding: 3px 8px;
    background: var(--amber-dim);
    border: 1px solid rgba(245,158,11,.3);
    border-radius: 4px;
    font-size: 10px; color: var(--amber);
}
.attachment-item img { max-width: 40px; max-height: 40px; border-radius: 2px; }
.attachment-item .remove { cursor: pointer; color: #ef4444; margin-left: 4px; font-size: 12px; }
.file-badge {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 7px;
    background: var(--amber-dim);
    border: 1px solid rgba(245,158,11,.3);
    border-radius: 4px;
    font-size: 10px; color: var(--amber);
    margin-bottom: 5px;
}

/* Thinking */
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
    margin: 0 14px;
}
.thinking.visible { display: block; }
.thinking::after { content: '…'; animation: dots 1.5s steps(3,end) infinite; }
@keyframes dots { 0%,33%{content:'.'} 34%,66%{content:'..'} 67%,100%{content:'…'} }
@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.3} }
@keyframes blink { 0%,100%{opacity:1} 50%{opacity:.4} }

/* Input */
.input-wrap { border-top: 1px solid var(--border); flex-shrink: 0; background: var(--bg-toolbar); }
.mode-row { display: flex; align-items: center; gap: 4px; padding: 5px 14px 0; }
.mode-btn {
    background: transparent; color: var(--text-dim); border: none;
    padding: 3px 10px; font-size: 10px; font-weight: 600;
    border-radius: 4px; cursor: pointer; transition: all .15s; letter-spacing: .3px;
}
.mode-btn.active { background: var(--bg-input); color: var(--amber); border: 1px solid var(--border); }
.mode-btn:not(.active):hover { color: var(--text); }
.input-row { display: flex; gap: 5px; padding: 6px 14px 10px; align-items: flex-end; }
.input-wrapper {
    flex: 1; position: relative; display: flex;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 7px; overflow: hidden;
}
.input-wrapper:focus-within { border-color: var(--amber); }
textarea#input {
    flex: 1; background: transparent; border: none;
    padding: 7px 10px; color: var(--text);
    font-family: inherit; font-size: 13px;
    resize: none; outline: none;
    min-height: 36px; max-height: 120px; line-height: 1.45;
}
.btn-icon {
    background: transparent; border: 1px solid var(--border); border-radius: 7px;
    color: var(--text-dim); font-size: 14px;
    width: 34px; height: 34px;
    display: flex; align-items: center; justify-content: center;
    cursor: pointer; transition: border-color .15s, color .15s;
    flex-shrink: 0; user-select: none;
}
.btn-icon:hover { border-color: var(--amber); color: var(--amber); }
.btn-voice {
    background: transparent; border: none;
    color: var(--text-dim); cursor: pointer;
    padding: 6px 8px; display: flex; align-items: center;
    align-self: flex-end; transition: color .15s; flex-shrink: 0;
}
.btn-voice:hover { color: var(--text); }
.btn-voice.recording { color: #ef4444; animation: blink 1s infinite; }
.btn-voice svg { width: 14px; height: 14px; fill: currentColor; }
.btn-send {
    background: var(--amber); color: #000; border: none;
    border-radius: 7px; padding: 0 14px;
    font-weight: 700; font-size: 13px; height: 34px;
    cursor: pointer; flex-shrink: 0; transition: opacity .15s;
}
.btn-send:hover { opacity: .85; }
.btn-send:disabled { opacity: .35; cursor: not-allowed; }
</style>
</head>
<body>

<div class="header">
    <div class="hex"></div>
    LaRuche
    <div class="header-actions">
        <button class="icon-btn" id="btn-new-chat" title="Nouvelle conversation">
            <svg viewBox="0 0 24 24"><path d="M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"/></svg>
        </button>
        <button class="icon-btn" id="btn-history" title="Historique">
            <svg viewBox="0 0 24 24"><path d="M13.5,8H12V13L16.28,15.54L17,14.33L13.5,12.25V8M13,3A9,9 0 0,0 4,12H1L4.96,16.03L9,12H6A7,7 0 0,1 13,5A7,7 0 0,1 20,12A7,7 0 0,1 13,19C11.07,19 9.32,18.21 8.06,16.94L6.64,18.36C8.27,20 10.5,21 13,21A9,9 0 0,0 22,12A9,9 0 0,0 13,3Z"/></svg>
        </button>
    </div>
    <div class="header-right">
        <div class="status-dot" id="status-dot"></div>
        <span id="status-text">Connexion…</span>
    </div>
</div>

<div class="toolbar">
    <div class="toolbar-pill" id="node-pill" title="Changer de nœud">
        <span class="icon">⬡</span>
        <span id="node-label">découverte…</span>
    </div>
    <span class="toolbar-sep">·</span>
    <div class="toolbar-pill" id="model-pill" title="Changer de modèle">
        <span class="icon">◈</span>
        <span id="model-label">défaut</span>
    </div>
</div>

<div class="messages" id="messages">
    <div class="msg-container assistant">
        <div class="msg assistant">
            Bienvenue ! Je suis votre assistant LaRuche local.<br><br>
            <strong>Chat</strong> — posez-moi une question.<br>
            <strong>Agent</strong> — donnez-moi des instructions sur le fichier actif.<br>
            <strong>📎</strong> — fichier actif · <strong>📁</strong> — importer un fichier.
            <div class="meta"><span>LaRuche v0.2.0</span><span>LAND Protocol</span></div>
        </div>
    </div>
</div>

<div class="thinking" id="thinking">LaRuche réfléchit</div>
<div id="attachment-preview" class="attachment-preview"></div>

<div class="input-wrap">
    <div class="mode-row">
        <button class="mode-btn active" id="btn-chat">Chat</button>
        <button class="mode-btn" id="btn-edit">Agent (Edit)</button>
    </div>
    <div class="input-row">
        <div class="btn-icon" id="btn-upload" title="Importer un fichier (dialog)">📁</div>
        <div class="input-wrapper">
            <textarea id="input" rows="1" placeholder="Posez une question…"></textarea>
            <button id="voice-btn" class="btn-voice" title="Dictée vocale">
                <svg viewBox="0 0 24 24"><path d="M12,2A3,3 0 0,1 15,5V11A3,3 0 0,1 12,14A3,3 0 0,1 9,11V5A3,3 0 0,1 12,2M19,11C19,14.53 16.39,17.44 13,17.93V21H11V17.93C7.61,17.44 5,14.53 5,11H7A5,5 0 0,0 12,16A5,5 0 0,0 17,11H19Z"/></svg>
            </button>
        </div>
        <div class="btn-icon" id="btn-attach" title="Attacher le fichier actif">📎</div>
        <button class="btn-send" id="btn-send">→</button>
    </div>
</div>

<script nonce="${nonce}">
(function() {
'use strict';

// ── Debug diagnostic bar ──────────────────────────────────────────────────────
var _dbg = document.createElement('div');
_dbg.id = 'lr-debug';
_dbg.style.cssText = 'background:#b91c1c;color:#fff;font-size:10px;padding:3px 10px;text-align:center;z-index:9999;';
_dbg.textContent = '⏳ JS loading...';
document.body.prepend(_dbg);

try { // Global error catch for diagnostics

// ── DOM refs ──────────────────────────────────────────────────────────────────
const vscode       = acquireVsCodeApi();
const messagesEl   = document.getElementById('messages');
const inputEl      = document.getElementById('input');
const sendBtn      = document.getElementById('btn-send');
const thinkingEl   = document.getElementById('thinking');
const statusEl     = document.getElementById('status-text');
const statusDot    = document.getElementById('status-dot');
const nodeLabelEl  = document.getElementById('node-label');
const modelLabelEl = document.getElementById('model-label');
const btnChat      = document.getElementById('btn-chat');
const btnEdit      = document.getElementById('btn-edit');
const attachPrev   = document.getElementById('attachment-preview');
const voiceBtn     = document.getElementById('voice-btn');

// ── State ─────────────────────────────────────────────────────────────────────
let currentMode = 'chat';
let attachments = [];
let isListening = false;
let recognition = null;
let safetyTimeout = null;
const SAFETY_TIMEOUT_MS = 60000; // 60s max wait

// ── Speech Recognition (lazy init) ───────────────────────────────────────────
(function initVoice() {
    const hasSR = ('webkitSpeechRecognition' in window) || ('SpeechRecognition' in window);
    if (!hasSR) { voiceBtn.style.display = 'none'; return; }
    try {
        const SR = window.SpeechRecognition || window.webkitSpeechRecognition;
        recognition = new SR();
        recognition.lang = 'fr-FR';
        recognition.continuous = false;
        recognition.interimResults = false;
        recognition.onresult = function(e) {
            const t = e.results[0][0].transcript;
            inputEl.value += (inputEl.value ? ' ' : '') + t;
            autoResize();
        };
        recognition.onend  = function() { isListening = false; voiceBtn.classList.remove('recording'); };
        recognition.onerror = function() { isListening = false; voiceBtn.classList.remove('recording'); };
    } catch(e) {
        voiceBtn.style.display = 'none';
    }
})();

// ── Restore persisted conversation ───────────────────────────────────────────
var prev = vscode.getState();
if (prev && prev.html) {
    messagesEl.innerHTML = prev.html;
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

// ── Helpers ───────────────────────────────────────────────────────────────────
function saveState() { vscode.setState({ html: messagesEl.innerHTML }); }

function autoResize() {
    inputEl.style.height = 'auto';
    inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
}

function setMode(mode) {
    currentMode = mode;
    btnChat.classList.toggle('active', mode === 'chat');
    btnEdit.classList.toggle('active', mode === 'edit');
    inputEl.placeholder = mode === 'chat' ? 'Posez une question…' : 'Instructions pour le fichier actif…';
}

function _doResetChat() {
    messagesEl.innerHTML =
        '<div class="msg-container assistant"><div class="msg assistant">' +
        'Bienvenue ! Nouvelle conversation.' +
        '<div class="meta"><span>LaRuche v0.2.0</span></div></div></div>';
    attachments = [];
    updateAttachmentUI();
    saveState();
}

// ── Button wiring ─────────────────────────────────────────────────────────────
document.getElementById('btn-new-chat').addEventListener('click', function() {
    vscode.postMessage({ type: 'confirmNewChat', currentHtml: messagesEl.innerHTML });
});
document.getElementById('btn-history').addEventListener('click', function() {
    vscode.postMessage({ type: 'getHistory' });
});
document.getElementById('node-pill').addEventListener('click', function() {
    vscode.postMessage({ type: 'selectNode' });
});
document.getElementById('model-pill').addEventListener('click', function() {
    vscode.postMessage({ type: 'selectModel' });
});
btnChat.addEventListener('click', function() { setMode('chat'); });
btnEdit.addEventListener('click', function() { setMode('edit'); });
document.getElementById('btn-upload').addEventListener('click', function() {
    vscode.postMessage({ type: 'upload' });
});
voiceBtn.addEventListener('click', function() {
    if (!recognition) return;
    if (isListening) { recognition.stop(); }
    else { recognition.start(); isListening = true; voiceBtn.classList.add('recording'); }
});
document.getElementById('btn-attach').addEventListener('click', function() {
    vscode.postMessage({ type: 'attachFile' });
});
sendBtn.addEventListener('click', sendMessage);

inputEl.addEventListener('input', autoResize);
inputEl.addEventListener('keydown', function(e) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); }
});

// ── Send ──────────────────────────────────────────────────────────────────────
function sendMessage() {
    var text = inputEl.value.trim();
    if (!text && attachments.length === 0) { return; }

    var displayHtml = '';
    attachments.forEach(function(a) {
        displayHtml += '<div class="file-badge">' +
            (a.type && a.type.startsWith('image/') ? '🖼 ' : '📄 ') +
            escHtml(a.name) + '</div>';
    });
    displayHtml += escHtml(text || '(Fichier joint)');
    addRawMessage('user', displayHtml);

    var combinedPrompt = text;
    if (attachments.length > 0) {
        var parts = attachments.map(function(a) {
            if (a.type && a.type.startsWith('image/')) { return '[Image: ' + a.name + ']'; }
            return 'File: ' + a.name + '\\n---\\n' + a.content;
        }).join('\\n\\n');
        combinedPrompt = text ? text + '\\n\\nContexte:\\n' + parts : parts;
    }

    inputEl.value = '';
    inputEl.style.height = 'auto';
    sendBtn.disabled = true;
    thinkingEl.classList.add('visible');
    attachments = [];
    updateAttachmentUI();

    if (safetyTimeout) { clearTimeout(safetyTimeout); }
    safetyTimeout = setTimeout(function() {
        if (sendBtn.disabled) {
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', '⚠ Timeout : le nœud LaRuche ne répond pas. Vérifiez la connexion.');
        }
    }, SAFETY_TIMEOUT_MS);

    vscode.postMessage({ type: 'ask', mode: currentMode, prompt: combinedPrompt });
}

// ── Message rendering ─────────────────────────────────────────────────────────
function addRawMessage(role, htmlContent, metaHtml) {
    if (role === 'system-msg') {
        var d = document.createElement('div');
        d.className = 'msg system-msg';
        d.textContent = htmlContent;
        messagesEl.appendChild(d);
        messagesEl.scrollTop = messagesEl.scrollHeight;
        saveState();
        return d;
    }
    var container = document.createElement('div');
    container.className = 'msg-container ' + role;

    var div = document.createElement('div');
    div.className = 'msg ' + role;
    div.innerHTML = htmlContent;

    if (metaHtml) {
        var m = document.createElement('div');
        m.className = 'meta';
        m.innerHTML = metaHtml;
        div.appendChild(m);
    }
    container.appendChild(div);

    if (role === 'assistant') {
        var copyBtn = document.createElement('button');
        copyBtn.className = 'copy-btn icon-btn';
        copyBtn.title = 'Copier';
        copyBtn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/></svg>';
        copyBtn.addEventListener('click', function() {
            var raw = div.innerText || div.textContent || '';
            var onOk = function() {
                copyBtn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M21,7L9,19L3.5,13.5L4.91,12.09L9,16.17L19.59,5.59L21,7Z"/></svg>';
                copyBtn.style.color = 'var(--green)';
                setTimeout(function() {
                    copyBtn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/></svg>';
                    copyBtn.style.color = '';
                }, 1500);
            };
            var fallback = function() {
                var el = document.createElement('textarea');
                el.value = raw; document.body.appendChild(el); el.select();
                document.execCommand('copy'); document.body.removeChild(el);
            };
            if (navigator.clipboard) {
                navigator.clipboard.writeText(raw).then(onOk).catch(function() { fallback(); onOk(); });
            } else { fallback(); onOk(); }
        });
        container.appendChild(copyBtn);
    }

    messagesEl.appendChild(container);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    saveState();
    return container;
}

function addMessage(role, text, metaHtml) { addRawMessage(role, renderMarkdown(text), metaHtml); }
function addSystemMessage(text) { addRawMessage('system-msg', text); }

// ── Attachments UI ────────────────────────────────────────────────────────────
function updateAttachmentUI() {
    attachPrev.innerHTML = '';
    attachments.forEach(function(a, i) {
        var div = document.createElement('div');
        div.className = 'attachment-item';
        if (a.type && a.type.startsWith('image/') && a.data) {
            div.innerHTML = '<img src="' + a.data + '"> <span>' + escHtml(a.name) + '</span>';
        } else {
            div.innerHTML = '<span>📄 ' + escHtml(a.name) + '</span>';
        }
        var rm = document.createElement('span');
        rm.className = 'remove'; rm.innerHTML = '✕';
        rm.addEventListener('click', (function(idx) {
            return function() { attachments.splice(idx, 1); updateAttachmentUI(); };
        })(i));
        div.appendChild(rm);
        attachPrev.appendChild(div);
    });
}

// ── Markdown renderer CORRIGÉ ─────────────────────────────────────────────────
var PFX = '~~LR_', SFX = '_LR~~';

function renderMarkdown(text) {
    var codeBlocks = [], inlineCodes = [];

    // 1. Fenced code blocks (Remplacement sécurisé pour le contexte Webview)
    text = text.replace(/\\\`\\\`\\\`([\\w-]*)\\n?([\\s\\S]*?)\\\`\\\`\\\`/g, function(_, lang, code) {
        var idx = codeBlocks.length;
        var label = lang ? '<span class="lang-label">' + escHtml(lang) + '</span>' : '';
        codeBlocks.push('<div class="code-block">' + label + '<pre><code>' + escHtml(code.trim()) + '</code></pre></div>');
        return PFX + 'CB' + idx + SFX;
    });
    
    // 2. Inline code
    text = text.replace(/\\\`([^\\\`\\n]+)\\\`/g, function(_, c) {
        var idx = inlineCodes.length;
        inlineCodes.push('<code>' + escHtml(c) + '</code>');
        return PFX + 'IC' + idx + SFX;
    });

    // 3. Escape HTML
    text = escHtml(text);

    // 4. Markdown transforms
    text = text.replace(/^### (.+)$/gm, '<h3>$1</h3>');
    text = text.replace(/^## (.+)$/gm, '<h2>$1</h2>');
    text = text.replace(/^# (.+)$/gm, '<h1>$1</h1>');

    text = text.replace(/\\*\\*(.+?)\\*\\*/g, '<strong>$1</strong>');
    text = text.replace(/\\*(.+?)\\*/g, '<em>$1</em>');

    text = text.replace(/^&gt; (.+)$/gm, '<blockquote>$1</blockquote>');
    text = text.replace(/^[-*] (.+)$/gm, '<li>$1</li>');
    text = text.replace(/((?:<li>.*<\\/li>\\n?)+)/g, function(m) {
        return '<ul>' + m + '</ul>';
    });
    
    text = text.replace(/^\\d+\\. (.+)$/gm, '<li class="ol-li">$1</li>');
    text = text.replace(/((?:<li class="ol-li">.*<\\/li>\\n?)+)/g, function(m) {
        return '<ol>' + m.replace(/ class="ol-li"/g, '') + '</ol>';
    });

    text = text.replace(/\\n/g, '<br>');
    text = text.replace(/<br>(<\\/?(ul|ol|li|h[1-3]|blockquote))/g, '$1');
    text = text.replace(/(<\\/(?:ul|ol|li|h[1-3]|blockquote)>)<br>/g, '$1');

    // 5. Restore
    codeBlocks.forEach(function(b, i) { text = text.split(PFX + 'CB' + i + SFX).join(b); });
    inlineCodes.forEach(function(c, i) { text = text.split(PFX + 'IC' + i + SFX).join(c); });

return text;
}

function escHtml(s) {
    return String(s)
        .replace(/&/g, '&amp;').replace(/</g, '&lt;')
        .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// ── Messages from extension ───────────────────────────────────────────────────
window.addEventListener('message', function (event) {
    var msg = event.data;
    switch (msg.type) {
        case 'response':
            if (safetyTimeout) { clearTimeout(safetyTimeout); safetyTimeout = null; }
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', msg.text, [
                msg.model ? '<span class="model-tag">◈ ' + escHtml(msg.model) + '</span>' : '',
                msg.tokens ? '<span>' + msg.tokens + ' tokens</span>' : '',
                msg.latency ? '<span>' + (msg.latency / 1000).toFixed(1) + 's</span>' : '',
                msg.node ? '<span>⬡ ' + escHtml(msg.node) + '</span>' : '',
            ].filter(Boolean).join(''));
            break;
        case 'error':
            if (safetyTimeout) { clearTimeout(safetyTimeout); safetyTimeout = null; }
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addMessage('assistant', '⚠ Erreur : ' + msg.text);
            break;
        case 'status':
            statusEl.textContent = msg.text;
            statusDot.classList.toggle('offline', msg.text.toLowerCase().includes('offline'));
            break;
        case 'agentDone':
            if (safetyTimeout) { clearTimeout(safetyTimeout); safetyTimeout = null; }
            thinkingEl.classList.remove('visible');
            sendBtn.disabled = false;
            addSystemMessage('✓ ' + (msg.text || 'Agent terminé'));
            break;
        case 'fileAttached':
            attachments.push({ name: msg.fileName, type: 'text/plain', content: msg.content });
            updateAttachmentUI();
            addSystemMessage('📎 ' + msg.fileName + ' attaché');
            break;
        case 'fileContent':
            attachments.push({ name: msg.name, type: msg.fileType, content: msg.content, data: msg.data });
            updateAttachmentUI();
            break;
        case 'loadChat':
            messagesEl.innerHTML = msg.html;
            messagesEl.scrollTop = messagesEl.scrollHeight;
            saveState();
            break;
        case 'resetChat':
            _doResetChat();
            break;
        case 'nodesUpdate':
            updateToolbar(msg.nodes, msg.activeNodeUrl, msg.activeModel);
            break;
        case 'context':
            inputEl.value = msg.text;
            autoResize();
            inputEl.focus();
            break;
    }
});

function updateToolbar(nodes, activeUrl, activeModel) {
    var nodeName = activeUrl || 'aucun';
    if (nodes && nodes.length > 0) {
        var active = nodes.find(function (n) { return n.url === activeUrl; });
        nodeName = active ? active.name : (nodes[0].name || activeUrl);
        statusDot.classList.remove('offline');
        statusEl.textContent = nodes.length + ' nœud' + (nodes.length > 1 ? 's' : '');
    } else {
        nodeName = 'offline';
        statusDot.classList.add('offline');
        statusEl.textContent = 'offline';
    }
    nodeLabelEl.textContent = nodeName.length > 18 ? nodeName.slice(0, 15) + '…' : nodeName;
    nodeLabelEl.title = activeUrl || nodeName;
    modelLabelEl.textContent = activeModel || 'défaut';
    modelLabelEl.title = activeModel || 'Modèle par défaut du nœud';
}

// ── Debug: confirm everything loaded ──────────────────────────────────────────
_dbg.textContent = '✓ JS OK — buttons wired';
_dbg.style.background = '#166534';
setTimeout(function () { _dbg.style.display = 'none'; }, 5000);

// Override button clicks to flash debug
['btn-new-chat', 'btn-history', 'node-pill', 'model-pill', 'btn-upload', 'btn-attach', 'btn-send'].forEach(function (id) {
    var el = document.getElementById(id);
    if (el) {
        el.addEventListener('click', function () {
            _dbg.style.display = 'block';
            _dbg.textContent = '🔔 Click: ' + id;
            _dbg.style.background = '#1e40af';
            setTimeout(function () { _dbg.style.display = 'none'; }, 1500);
        });
    }
});

} catch (e) {
    var _dbg2 = document.getElementById('lr-debug');
    if (_dbg2) {
        _dbg2.textContent = '❌ JS ERROR: ' + e.message;
        _dbg2.style.background = '#b91c1c';
    }
}

}) (); // end IIFE
</script>
    </body>
    </html>`;
}