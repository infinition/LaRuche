
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
const SAFETY_TIMEOUT_MS = 180000; // 3min max wait (agent loops can be long)

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
    // Build structured attachments for the agent mode
    var structuredAttachments = attachments.map(function(a) {
        return { name: a.name, content: a.content || '', language: a.language || '' };
    });
    if (attachments.length > 0) {
        var parts = attachments.map(function(a) {
            if (a.type && a.type.startsWith('image/')) { return '[Image: ' + a.name + ']'; }
            return 'File: ' + a.name + '\n---\n' + a.content;
        }).join('\n\n');
        combinedPrompt = text ? text + '\n\nContexte:\n' + parts : parts;
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

    // In agent mode, send structured attachments so the backend can include them in context
    var msgPayload = { type: 'ask', mode: currentMode, prompt: combinedPrompt, attachments: [] };
    if (currentMode === 'edit' && structuredAttachments.length > 0) {
        msgPayload.attachments = structuredAttachments;
    }
    vscode.postMessage(msgPayload);
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

function sanitizeAgentText(text) {
    if (!text) { return text; }
    // Strip <tool_call>...</tool_call> blocks
    var cleaned = text.replace(/<tool_call>[sS]*?</tool_call>/g, '');
    // Strip code blocks containing tool calls (use RegExp constructor to avoid backtick in template)
    var t = String.fromCharCode(96);
    cleaned = cleaned.replace(new RegExp(t+t+t+'(?:json)?[\\s\\S]*?'+t+t+t, 'g'), function(m) {
        return (m.indexOf('"name"') !== -1 && m.indexOf('"args"') !== -1) ? '' : m;
    });
    // Strip bare JSON tool call objects
    cleaned = cleaned.replace(/{s*"name"s*:s*"w+"s*,s*"args"s*:s*{[^}]*}s*}/g, '');
    // Strip <tool_result> blocks
    cleaned = cleaned.replace(/<tool_result[sS]*?</tool_result>/g, '');
    return cleaned.trim();
}
function addMessage(role, text, metaHtml) {
    var display = (role === 'assistant') ? sanitizeAgentText(text) : text;
    if (!display) { display = '(empty response)'; }
    addRawMessage(role, renderMarkdown(display), metaHtml);
}
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
    text = text.replace(/\`\`\`([\w-]*)\n?([\s\S]*?)\`\`\`/g, function(_, lang, code) {
        var idx = codeBlocks.length;
        var label = lang ? '<span class="lang-label">' + escHtml(lang) + '</span>' : '';
        codeBlocks.push('<div class="code-block">' + label + '<pre><code>' + escHtml(code.trim()) + '</code></pre></div>');
        return PFX + 'CB' + idx + SFX;
    });
    
    // 2. Inline code
    text = text.replace(/\`([^\`\n]+)\`/g, function(_, c) {
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

    text = text.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    text = text.replace(/\*(.+?)\*/g, '<em>$1</em>');

    text = text.replace(/^&gt; (.+)$/gm, '<blockquote>$1</blockquote>');
    text = text.replace(/^[-*] (.+)$/gm, '<li>$1</li>');
    text = text.replace(/((?:<li>.*<\/li>\n?)+)/g, function(m) {
        return '<ul>' + m + '</ul>';
    });
    
    text = text.replace(/^\d+\. (.+)$/gm, '<li class="ol-li">$1</li>');
    text = text.replace(/((?:<li class="ol-li">.*<\/li>\n?)+)/g, function(m) {
        return '<ol>' + m.replace(/ class="ol-li"/g, '') + '</ol>';
    });

    text = text.replace(/\n/g, '<br>');
    text = text.replace(/<br>(<\/?(ul|ol|li|h[1-3]|blockquote))/g, '$1');
    text = text.replace(/(<\/(?:ul|ol|li|h[1-3]|blockquote)>)<br>/g, '$1');

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
            if (msg.text && msg.text.length > 5) {
                var agentMeta = [
                    msg.model ? '<span class="model-tag">◈ ' + escHtml(msg.model) + '</span>' : '',
                    msg.tokens ? '<span>' + msg.tokens + ' tokens</span>' : '',
                    msg.iterations ? '<span>' + msg.iterations + ' iteration(s)</span>' : '',
                ].filter(Boolean).join('');
                addMessage('assistant', msg.text, agentMeta);
            } else {
                addSystemMessage('✓ ' + (msg.text || 'Agent terminé'));
            }
            break;
        case 'agentProgress':
            // Update the thinking indicator with progress text
            thinkingEl.textContent = msg.text || 'Agent working';
            thinkingEl.classList.add('visible');
            // Also reset safety timeout since we know the agent is alive
            if (safetyTimeout) { clearTimeout(safetyTimeout); }
            safetyTimeout = setTimeout(function() {
                if (sendBtn.disabled) {
                    thinkingEl.classList.remove('visible');
                    sendBtn.disabled = false;
                    addMessage('assistant', '⚠ Timeout : l\'agent ne répond plus.');
                }
            }, SAFETY_TIMEOUT_MS);
            break;
        case 'agentToolResult':
            // Show tool execution as a system message
            var icon = msg.success ? '✓' : '✗';
            var toolMsg = icon + ' ' + escHtml(msg.tool);
            if (msg.preview) { toolMsg += ' — ' + escHtml(msg.preview).slice(0, 120); }
            addSystemMessage(toolMsg);
            break;
        case 'agentThinking':
            // Show intermediate reasoning from the agent (strip any tool call artifacts)
            var thinking = msg.text ? sanitizeAgentText(msg.text) : '';
            if (thinking && thinking.length > 3) { addSystemMessage('💭 ' + escHtml(thinking).slice(0, 200)); }
            break;
        case 'fileAttached':
            attachments.push({ name: msg.fileName, type: 'text/plain', content: msg.content, language: msg.language || '' });
            updateAttachmentUI();
            addSystemMessage('📎 ' + msg.fileName + ' attaché');
            break;
        case 'fileContent':
            attachments.push({ name: msg.name, type: msg.fileType, content: msg.content, data: msg.data, language: msg.language || '' });
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
