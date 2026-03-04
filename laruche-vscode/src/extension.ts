import * as vscode from 'vscode';
import { LaRucheClient, SwarmData, ModelsResponse } from './client';
import { LandDiscovery, DiscoveredLandNode } from './discovery';
import { getChatHtml } from './chatView';
import { AgentProvider } from './agentProvider';

// ======================== State ========================

let client: LaRucheClient;
let agent: AgentProvider;
let discovery: LandDiscovery;
let statusBarItem: vscode.StatusBarItem;
let chatPanel: vscode.WebviewPanel | undefined;
let sidebarProvider: ChatViewProvider | undefined;
let pollInterval: NodeJS.Timeout | undefined;

/** URL of the currently active node */
let activeNodeUrl: string = '';
/** Active model override (empty = node default) */
let activeModel: string = '';
/** All nodes known from swarm + mDNS */
let knownNodes: Array<{ url: string; name: string; model?: string; capabilities: string[] }> = [];

// ======================== Activation ========================

export function activate(context: vscode.ExtensionContext) {
    console.log('LaRuche extension activated (v0.2.0)');

    // Restore persisted state
    activeNodeUrl = context.workspaceState.get<string>('laruche.activeNodeUrl', '');
    activeModel = context.workspaceState.get<string>('laruche.activeModel', '');

    initClient();

    // Status bar (left side, clickable to select node)
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.command = 'laruche.showSwarm';
    statusBarItem.text = '$(beaker) LaRuche';
    statusBarItem.tooltip = 'Click to view Swarm status';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    // Register all commands
    context.subscriptions.push(
        vscode.commands.registerCommand('laruche.ask', cmdAsk),
        vscode.commands.registerCommand('laruche.explainSelection', cmdExplainSelection),
        vscode.commands.registerCommand('laruche.refactorSelection', cmdRefactorSelection),
        vscode.commands.registerCommand('laruche.openChat', () => openChatPanel(context)),
        vscode.commands.registerCommand('laruche.showSwarm', cmdShowSwarm),
        vscode.commands.registerCommand('laruche.selectNode', () => cmdSelectNode(context)),
        vscode.commands.registerCommand('laruche.selectModel', () => cmdSelectModel(context)),
        vscode.commands.registerCommand('laruche.agentEdit', cmdAgentEdit),
        vscode.commands.registerCommand('laruche.undoLastEdit', () => agent.undoLast()),
        vscode.commands.registerCommand('laruche.agentHistory', () => agent.showHistory()),
    );

    // Sidebar chat view
    sidebarProvider = new ChatViewProvider(context);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider('laruche.chatView', sidebarProvider),
    );

    // Config change listener
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('laruche')) {
                initClient();
            }
        }),
    );

    // Start mDNS discovery (LAND protocol)
    discovery = new LandDiscovery(
        (node: DiscoveredLandNode) => onNodeDiscovered(node, context),
        (url: string) => onNodeLost(url),
    );
    const mdnsStarted = discovery.start();
    if (mdnsStarted) {
        console.log('LaRuche: LAND mDNS discovery started');
    } else {
        console.log('LaRuche: mDNS unavailable, using configured/localhost URL');
    }
    context.subscriptions.push({ dispose: () => discovery.stop() });

    // Start polling
    pollStatus();
    pollInterval = setInterval(pollStatus, 5000);
    context.subscriptions.push({
        dispose: () => { if (pollInterval) { clearInterval(pollInterval); } },
    });
}

export function deactivate() {
    if (pollInterval) { clearInterval(pollInterval); }
    discovery?.stop();
}

// ======================== Client Setup ========================

function initClient() {
    const config = vscode.workspace.getConfiguration('laruche');
    const configuredUrl = config.get<string>('nodeUrl', '');
    const configuredModel = config.get<string>('model', '');

    // Priority: manual config > mDNS discovered > localhost fallback
    if (configuredUrl) {
        activeNodeUrl = configuredUrl;
    } else if (!activeNodeUrl) {
        const port = config.get<number>('apiPort', 8419);
        activeNodeUrl = `http://localhost:${port}`;
    }

    if (configuredModel && !activeModel) {
        activeModel = configuredModel;
    }

    client = new LaRucheClient(activeNodeUrl);
    agent = new AgentProvider(client, activeModel || undefined);
}

function setActiveNode(url: string, context: vscode.ExtensionContext) {
    activeNodeUrl = url;
    context.workspaceState.update('laruche.activeNodeUrl', url);
    client.setBaseUrl(url);
    agent = new AgentProvider(client, activeModel || undefined);
}

function setActiveModel(model: string, context: vscode.ExtensionContext) {
    activeModel = model;
    context.workspaceState.update('laruche.activeModel', model);
    agent = new AgentProvider(client, model || undefined);
}

// ======================== mDNS Callbacks ========================

function onNodeDiscovered(node: DiscoveredLandNode, context: vscode.ExtensionContext) {
    console.log(`LaRuche: Discovered node via LAND: ${node.name} @ ${node.url}`);

    // Add to known nodes
    const idx = knownNodes.findIndex(n => n.url === node.url);
    const entry = { url: node.url, name: node.name, model: node.model, capabilities: node.capabilities };
    if (idx >= 0) { knownNodes[idx] = entry; } else { knownNodes.push(entry); }

    // Auto-connect to this node if:
    // - no manual URL is configured, AND
    // - the current active node is localhost (first-time) OR offline
    const hasManualUrl = !!vscode.workspace.getConfiguration('laruche').get<string>('nodeUrl');
    const isLocalhost = activeNodeUrl.includes('localhost') || activeNodeUrl.includes('127.0.0.1');
    if (!hasManualUrl && (isLocalhost || !activeNodeOnline) && node.url !== activeNodeUrl) {
        const testClient = new LaRucheClient(node.url);
        testClient.health().then(ok => {
            if (ok) {
                setActiveNode(node.url, context);
                vscode.window.showInformationMessage(
                    `LaRuche: Connecté à ${node.name} via LAND${node.model ? ` (${node.model})` : ''}`,
                    'OK', 'Changer de nœud',
                ).then(choice => {
                    if (choice === 'Changer de nœud') { cmdSelectNode(context); }
                });
                pollStatus();
            }
        }).catch(() => { /* ignore */ });
    }

    notifyWebviews({ type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel });
}

function onNodeLost(url: string) {
    knownNodes = knownNodes.filter(n => n.url !== url);
    notifyWebviews({ type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel });

    if (url === activeNodeUrl) {
        // Our active node disappeared — try to fall back to another
        const fallback = knownNodes[0];
        if (fallback) {
            activeNodeUrl = fallback.url;
            client.setBaseUrl(fallback.url);
            vscode.window.showWarningMessage(
                `LaRuche: Node offline. Switched to ${fallback.name}`,
            );
        } else {
            statusBarItem.text = '$(beaker) LaRuche: offline';
            statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        }
    }
}

// Track whether the active node is reachable
let activeNodeOnline = false;

// ======================== Status Polling ========================

async function pollStatus() {
    try {
        const swarm = await client.swarm();
        activeNodeOnline = true;

        // Merge swarm nodes into knownNodes (they come from mDNS on the node side)
        for (const n of swarm.nodes) {
            const url = `http://${n.host}:8419`;
            const existing = knownNodes.find(k => k.url === url || k.name === n.name);
            if (!existing) {
                knownNodes.push({
                    url,
                    name: n.name || n.host,
                    model: n.model ?? undefined,
                    capabilities: n.capabilities,
                });
            }
        }

        const nodeCount = swarm.total_nodes;
        const tps = swarm.collective_tps.toFixed(1);
        const modelLabel = activeModel ? ` · ${activeModel}` : '';
        statusBarItem.text = `$(beaker) ${nodeCount} node${nodeCount !== 1 ? 's' : ''} · ${tps} t/s${modelLabel}`;
        statusBarItem.tooltip = buildSwarmTooltip(swarm);
        statusBarItem.backgroundColor = undefined;

        notifyWebviews({
            type: 'status',
            text: `${nodeCount} node${nodeCount !== 1 ? 's' : ''} | ${tps} t/s`,
        });
        notifyWebviews({ type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel });
    } catch {
        activeNodeOnline = false;
        statusBarItem.text = '$(beaker) LaRuche: offline';
        statusBarItem.tooltip = 'No LaRuche node reachable.\nUse "LaRuche: Select Active Node" to connect.';
        statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    }
}

function buildSwarmTooltip(swarm: SwarmData): string {
    const lines = [
        `Swarm: ${swarm.total_nodes} nodes | ${swarm.collective_tps.toFixed(1)} tok/s`,
        `Queue: ${swarm.collective_queue} | RAM: ${formatMB(swarm.total_ram_mb)}`,
        `Active node: ${activeNodeUrl}`,
        activeModel ? `Active model: ${activeModel}` : 'Model: node default',
        '',
        'Click to view details',
    ];
    return lines.join('\n');
}

// ======================== Commands ========================

async function cmdAsk() {
    const prompt = await vscode.window.showInputBox({
        prompt: 'What do you want to ask LaRuche?',
        placeHolder: 'Explain how async/await works in Rust...',
    });
    if (!prompt) { return; }
    await askAndShow(prompt);
}

async function cmdExplainSelection() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { vscode.window.showWarningMessage('No active editor with selection.'); return; }
    const selection = editor.document.getText(editor.selection);
    if (!selection) { vscode.window.showWarningMessage('Please select some code first.'); return; }

    const lang = editor.document.languageId;
    await askAndShow(
        `Explain this ${lang} code clearly and concisely:\n\n\`\`\`${lang}\n${selection}\n\`\`\``,
    );
}

async function cmdRefactorSelection() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { vscode.window.showWarningMessage('No active editor.'); return; }
    const selection = editor.document.getText(editor.selection);
    if (!selection) { vscode.window.showWarningMessage('Please select some code first.'); return; }

    const lang = editor.document.languageId;
    const prompt = `Refactor and improve this ${lang} code. Return ONLY the improved code, no explanations:\n\n\`\`\`${lang}\n${selection}\n\`\`\``;

    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'LaRuche: Refactoring...',
        cancellable: false,
    }, async () => {
        try {
            const resp = await client.infer(prompt, 'code', activeModel || undefined);
            const cleaned = resp.response
                .replace(/^```\w*\n?/, '')
                .replace(/\n?```$/, '')
                .trim();
            await editor.edit(b => b.replace(editor.selection, cleaned));
            vscode.window.showInformationMessage(
                `LaRuche: Refactored (${resp.tokens_generated} tokens, ${(resp.latency_ms / 1000).toFixed(1)}s, ${resp.model})`,
            );
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche: ${err.message}`);
        }
    });
}

async function cmdShowSwarm() {
    try {
        const swarm = await client.swarm();
        const items: vscode.QuickPickItem[] = [];

        items.push({
            label: `$(zap) Collective Power`,
            description: `${swarm.total_nodes} nodes · ${swarm.collective_tps.toFixed(1)} t/s · Q:${swarm.collective_queue}`,
            detail: `RAM: ${formatMB(swarm.total_ram_mb)} | VRAM: ${formatMB(swarm.total_vram_mb)}`,
        });

        for (const n of swarm.nodes) {
            const modelLabel = n.model ? ` [${n.model}]` : '';
            const isActive = activeNodeUrl.includes(n.host);
            items.push({
                label: `${isActive ? '$(check) ' : '$(server) '}${n.name || 'Unknown'}${modelLabel}`,
                description: `${n.host} · ${n.tokens_per_sec?.toFixed(1) || '?'} t/s · Q:${n.queue_depth || 0}`,
                detail: `Capabilities: ${n.capabilities.join(', ') || 'none'}`,
            });
        }

        await vscode.window.showQuickPick(items, {
            title: 'LaRuche Swarm Intelligence',
            placeHolder: 'Your local AI collective',
        });
    } catch (err: any) {
        vscode.window.showErrorMessage(`LaRuche: Cannot reach node — ${err.message}`);
    }
}

async function cmdSelectNode(context: vscode.ExtensionContext) {
    const items: vscode.QuickPickItem[] = [];

    // Add mDNS-discovered nodes
    const mdnsNodes = discovery.getNodes();
    for (const n of mdnsNodes) {
        const isActive = n.url === activeNodeUrl;
        items.push({
            label: `${isActive ? '$(check) ' : '$(remote-explorer) '}${n.name}`,
            description: `${n.url}${n.model ? ` · ${n.model}` : ''}`,
            detail: `Discovered via LAND · ${n.capabilities.join(', ') || 'no capabilities'}`,
        });
    }

    // Add swarm nodes (from HTTP)
    for (const n of knownNodes) {
        if (!mdnsNodes.find(m => m.url === n.url)) {
            const isActive = n.url === activeNodeUrl;
            items.push({
                label: `${isActive ? '$(check) ' : '$(server) '}${n.name}`,
                description: `${n.url}${n.model ? ` · ${n.model}` : ''}`,
                detail: `Discovered via Swarm · ${n.capabilities.join(', ') || 'no capabilities'}`,
            });
        }
    }

    // Always offer manual entry
    items.push({
        label: '$(add) Enter URL manually...',
        description: 'http://192.168.1.x:8419',
        detail: 'Connect to a node not yet discovered',
    });

    const selected = await vscode.window.showQuickPick(items, {
        title: 'LaRuche — Select Active Node',
        placeHolder: items.length === 1 ? 'No nodes discovered yet' : 'Choose which node to use',
    });

    if (!selected) { return; }

    if (selected.label.includes('Enter URL manually')) {
        const url = await vscode.window.showInputBox({
            prompt: 'LaRuche node URL',
            placeHolder: 'http://192.168.1.42:8419',
            value: activeNodeUrl,
        });
        if (url) {
            setActiveNode(url.trim(), context);
            vscode.window.showInformationMessage(`LaRuche: Connected to ${url}`);
            pollStatus();
        }
    } else {
        // Extract URL from description
        const url = selected.description?.split(' · ')[0];
        if (url) {
            setActiveNode(url, context);
            vscode.window.showInformationMessage(`LaRuche: Switched to ${selected.label.replace(/^\$\([\w-]+\) /, '')}`);
            pollStatus();
        }
    }
}

async function cmdSelectModel(context: vscode.ExtensionContext) {
    let modelItems: vscode.QuickPickItem[] = [];

    try {
        const resp: ModelsResponse = await client.models();
        modelItems = resp.models.map(m => ({
            label: `$(symbol-namespace) ${m.name}`,
            description: `${m.size_gb.toFixed(1)} GB · ${m.digest}`,
            detail: m.name === resp.default_model ? '★ Default model' : undefined,
        }));
    } catch {
        vscode.window.showWarningMessage('LaRuche: Could not fetch model list from node. Enter manually.');
    }

    modelItems.unshift({
        label: '$(circle-slash) Node default',
        description: 'Use the model configured on the node',
        detail: activeModel ? `Currently: ${activeModel} — clear this to use node default` : 'Currently active',
    });

    const selected = await vscode.window.showQuickPick(modelItems, {
        title: `LaRuche — Select Model (node: ${activeNodeUrl})`,
        placeHolder: 'Choose the model to use for all requests',
    });

    if (!selected) { return; }

    if (selected.label.includes('Node default')) {
        setActiveModel('', context);
        vscode.window.showInformationMessage('LaRuche: Using node default model');
    } else {
        const model = selected.label.replace(/^\$\([\w-]+\) /, '');
        setActiveModel(model, context);
        vscode.window.showInformationMessage(`LaRuche: Active model set to ${model}`);
    }

    notifyWebviews({ type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel });
}

async function cmdAgentEdit() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('LaRuche Agent: No active editor.');
        return;
    }

    const mode = agent.getMode();
    const instructions = await vscode.window.showInputBox({
        prompt: `LaRuche Agent [${mode}]${activeModel ? ` · ${activeModel}` : ''}: What should I do?`,
        placeHolder: 'Add error handling, refactor the loop, fix the bug on line 42...',
    });

    if (!instructions) { return; }
    await agent.run(editor, instructions);
}

// ======================== Chat Panel ========================

function openChatPanel(context: vscode.ExtensionContext) {
    if (chatPanel) {
        chatPanel.reveal(vscode.ViewColumn.Beside);
        return;
    }

    chatPanel = vscode.window.createWebviewPanel(
        'larucheChat',
        'LaRuche Chat',
        vscode.ViewColumn.Beside,
        { enableScripts: true, retainContextWhenHidden: true },
    );

    chatPanel.webview.html = getChatHtml(chatPanel.webview);
    chatPanel.webview.onDidReceiveMessage(
        msg => handleWebviewMessage(msg, chatPanel!.webview, context),
        undefined,
        context.subscriptions,
    );
    chatPanel.onDidDispose(() => { chatPanel = undefined; }, null, context.subscriptions);

    // Send initial state
    setTimeout(() => {
        chatPanel?.webview.postMessage({
            type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel,
        });
    }, 300);
}

// ======================== Sidebar Chat View ========================

class ChatViewProvider implements vscode.WebviewViewProvider {
    private view: vscode.WebviewView | undefined;

    constructor(private readonly context: vscode.ExtensionContext) { }

    postMessage(msg: object): void {
        this.view?.webview.postMessage(msg);
    }

    resolveWebviewView(webviewView: vscode.WebviewView) {
        this.view = webviewView;
        webviewView.webview.options = { enableScripts: true };
        webviewView.webview.html = getChatHtml(webviewView.webview);

        webviewView.webview.onDidReceiveMessage(
            msg => handleWebviewMessage(msg, webviewView.webview, this.context),
            undefined,
            this.context.subscriptions,
        );

        // Send initial state shortly after the view loads
        setTimeout(() => {
            webviewView.webview.postMessage({
                type: 'nodesUpdate', nodes: knownNodes, activeNodeUrl, activeModel,
            });
        }, 300);
    }
}

// ======================== Shared Message Handler ========================

async function handleWebviewMessage(
    msg: any,
    webview: vscode.Webview,
    context: vscode.ExtensionContext,
): Promise<void> {
    switch (msg.type) {
        case 'ask':
            await handleChatAsk(msg, webview, context);
            break;

        case 'attachFile': {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                webview.postMessage({ type: 'error', text: 'No active editor open.' });
                return;
            }
            const content = editor.document.getText();
            const lang = editor.document.languageId;
            const name = vscode.workspace.asRelativePath(editor.document.uri);
            webview.postMessage({
                type: 'fileAttached',
                fileName: name,
                language: lang,
                content: content.slice(0, 40000), // cap at 40k chars
            });
            break;
        }

        case 'selectNode':
            await cmdSelectNode(context);
            break;

        case 'selectModel':
            await cmdSelectModel(context);
            break;

        case 'confirmNewChat': {
            const answer = await vscode.window.showInformationMessage(
                'Démarrer une nouvelle conversation ?',
                { modal: false },
                'Oui', 'Non',
            );
            if (answer !== 'Oui') { break; }
            if (msg.currentHtml && msg.currentHtml.trim().length > 200) {
                saveChatToHistory(context, msg.currentHtml);
            }
            webview.postMessage({ type: 'resetChat' });
            break;
        }

        case 'getHistory': {
            const history = getChatHistory(context);
            if (history.length === 0) {
                vscode.window.showInformationMessage('LaRuche: Aucun historique de conversation.');
                return;
            }
            const histItems = history.map((h: any, i: number) => ({
                label: `$(history) ${h.title}`,
                description: new Date(h.timestamp).toLocaleString(),
                index: i,
            }));
            const sel = await vscode.window.showQuickPick(histItems, {
                title: 'LaRuche — Historique des conversations',
                placeHolder: 'Sélectionnez une conversation à charger',
            });
            if (sel) {
                webview.postMessage({ type: 'loadChat', html: history[(sel as any).index].html });
            }
            break;
        }

        case 'upload': {
            const fileUri = await vscode.window.showOpenDialog({
                canSelectMany: false,
                openLabel: 'Joindre',
                filters: {
                    'Fichiers': ['png', 'jpg', 'jpeg', 'gif', 'txt', 'md', 'js', 'ts', 'py', 'rs', 'html', 'css', 'json', 'toml', 'yaml', 'yml'],
                },
            });
            if (!fileUri || !fileUri[0]) { break; }
            const uri = fileUri[0];
            const fileName = uri.path.split('/').pop() || 'file';
            const ext = fileName.split('.').pop()?.toLowerCase() || '';
            try {
                const data = await vscode.workspace.fs.readFile(uri);
                const isImage = ['png', 'jpg', 'jpeg', 'gif'].includes(ext);
                if (isImage) {
                    const b64 = Buffer.from(data).toString('base64');
                    webview.postMessage({
                        type: 'fileContent',
                        name: fileName,
                        fileType: `image/${ext}`,
                        data: `data:image/${ext};base64,${b64}`,
                        content: `[Image: ${fileName}]`,
                    });
                } else {
                    webview.postMessage({
                        type: 'fileContent',
                        name: fileName,
                        fileType: 'text/plain',
                        content: Buffer.from(data).toString('utf8').slice(0, 40000),
                    });
                }
            } catch (err: any) {
                vscode.window.showErrorMessage(`LaRuche: Erreur lecture fichier — ${err.message}`);
            }
            break;
        }
    }
}

async function handleChatAsk(
    msg: any,
    webview: vscode.Webview,
    context: vscode.ExtensionContext,
): Promise<void> {
    if (msg.mode === 'edit') {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            webview.postMessage({ type: 'error', text: 'No active editor. Open a file to use Agent mode.' });
            return;
        }
        webview.postMessage({ type: 'status', text: 'Agent working...' });
        try {
            await agent.run(editor, msg.prompt);
            webview.postMessage({ type: 'agentDone', text: 'Agent finished.' });
        } catch (err: any) {
            webview.postMessage({ type: 'error', text: err.message });
        }
        return;
    }

    // Chat mode
    const modelOverride = msg.model || activeModel || undefined;
    const capability = msg.capability || 'llm';

    try {
        const resp = await client.inferChat(msg.prompt, capability, modelOverride);
        webview.postMessage({
            type: 'response',
            text: resp.response,
            model: resp.model,
            tokens: resp.tokens_generated,
            latency: resp.latency_ms,
            node: resp.node_name,
        });
    } catch (err: any) {
        webview.postMessage({ type: 'error', text: err.message });
    }
}

// ======================== Helpers ========================

async function askAndShow(prompt: string) {
    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'LaRuche is thinking...',
        cancellable: false,
    }, async () => {
        try {
            const resp = await client.inferChat(prompt, 'llm', activeModel || undefined);
            const doc = await vscode.workspace.openTextDocument({
                content: resp.response,
                language: 'markdown',
            });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
            vscode.window.showInformationMessage(
                `LaRuche: ${resp.tokens_generated} tokens · ${(resp.latency_ms / 1000).toFixed(1)}s · ${resp.model} @ ${resp.node_name}`,
            );
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche: ${err.message}`);
        }
    });
}

function notifyWebviews(msg: object) {
    chatPanel?.webview.postMessage(msg);
    sidebarProvider?.postMessage(msg);
}

function formatMB(mb: number): string {
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
}

// ======================== Chat History ========================

function saveChatToHistory(context: vscode.ExtensionContext, html: string) {
    const history = context.globalState.get<any[]>('laruche.chatHistory', []);

    // Extract first user message text as title (works with msg-container structure)
    let title = 'Conversation ' + new Date().toLocaleTimeString();
    const match = html.match(/class="msg user"[^>]*>([\s\S]*?)<\/div>/);
    if (match) {
        const raw = match[1].replace(/<[^>]*>/g, '').replace(/\s+/g, ' ').trim();
        if (raw.length > 0) { title = raw.slice(0, 55) + (raw.length > 55 ? '…' : ''); }
    }

    history.push({ title, html, timestamp: Date.now() });
    if (history.length > 20) { history.shift(); }
    context.globalState.update('laruche.chatHistory', history);
}

function getChatHistory(context: vscode.ExtensionContext): any[] {
    return [...(context.globalState.get<any[]>('laruche.chatHistory', []))].reverse();
}
