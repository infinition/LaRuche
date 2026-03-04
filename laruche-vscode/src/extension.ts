import * as vscode from 'vscode';
import { LaRucheClient } from './client';
import { getChatHtml } from './chatView';
import { AgentProvider } from './agentProvider';

let client: LaRucheClient;
let agent: AgentProvider;
let statusBarItem: vscode.StatusBarItem;
let chatPanel: vscode.WebviewPanel | undefined;
let pollInterval: NodeJS.Timeout | undefined;

// ======================== Activation ========================

export function activate(context: vscode.ExtensionContext) {
    console.log('LaRuche extension activated');

    // Initialize client and agent
    updateClient();

    // Status bar
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.command = 'laruche.showSwarm';
    statusBarItem.text = '$(beaker) LaRuche';
    statusBarItem.tooltip = 'Click to view Swarm status';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('laruche.ask', cmdAsk),
        vscode.commands.registerCommand('laruche.explainSelection', cmdExplainSelection),
        vscode.commands.registerCommand('laruche.refactorSelection', cmdRefactorSelection),
        vscode.commands.registerCommand('laruche.openChat', () => openChatPanel(context)),
        vscode.commands.registerCommand('laruche.showSwarm', cmdShowSwarm),
        vscode.commands.registerCommand('laruche.agentEdit', cmdAgentEdit),
        vscode.commands.registerCommand('laruche.undoLastEdit', () => agent.undoLast()),
        vscode.commands.registerCommand('laruche.agentHistory', () => agent.showHistory()),
    );

    // Register sidebar webview
    const chatProvider = new ChatViewProvider(context);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider('laruche.chatView', chatProvider),
    );

    // Listen for config changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('laruche')) {
                updateClient();
            }
        }),
    );

    // Start polling for status
    pollStatus();
    pollInterval = setInterval(pollStatus, 5000);
    context.subscriptions.push({ dispose: () => { if (pollInterval) { clearInterval(pollInterval); } } });
}

export function deactivate() {
    if (pollInterval) {
        clearInterval(pollInterval);
    }
}

// ======================== Client Setup ========================

function getBaseUrl(): string {
    const config = vscode.workspace.getConfiguration('laruche');
    const nodeUrl = config.get<string>('nodeUrl', '');
    if (nodeUrl) {
        return nodeUrl;
    }
    const port = config.get<number>('apiPort', 8419);
    return `http://localhost:${port}`;
}

function updateClient() {
    client = new LaRucheClient(getBaseUrl());
    agent = new AgentProvider(client);
}

// ======================== Agent ========================

async function cmdAgentEdit() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('LaRuche Agent: No active editor.');
        return;
    }

    const mode = agent.getMode();
    const instructions = await vscode.window.showInputBox({
        prompt: `LaRuche Agent [${mode}]: What should I do with this file?`,
        placeHolder: 'Add error handling, refactor the loops, fix the bug on line 42...',
    });

    if (!instructions) { return; }

    await agent.run(editor, instructions);
}

// ======================== Status Polling ========================

async function pollStatus() {
    try {
        const swarm = await client.swarm();
        const nodeCount = swarm.total_nodes;
        const tps = swarm.collective_tps.toFixed(1);
        statusBarItem.text = `$(beaker) LaRuche: ${nodeCount} node${nodeCount > 1 ? 's' : ''} | ${tps} t/s`;
        statusBarItem.tooltip = `Swarm: ${nodeCount} nodes\nCollective: ${tps} tokens/sec\nQueue: ${swarm.collective_queue}\nClick to view details`;
        statusBarItem.backgroundColor = undefined;

        // Update chat panel status if open
        if (chatPanel) {
            chatPanel.webview.postMessage({ type: 'status', text: `${nodeCount} node${nodeCount > 1 ? 's' : ''} | ${tps} t/s` });
        }
    } catch {
        statusBarItem.text = '$(beaker) LaRuche: offline';
        statusBarItem.tooltip = 'No LaRuche node found. Start one with: cargo run -p laruche-node';
        statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    }
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
    if (!editor) {
        vscode.window.showWarningMessage('No active editor with selection.');
        return;
    }

    const selection = editor.document.getText(editor.selection);
    if (!selection) {
        vscode.window.showWarningMessage('Please select some code first.');
        return;
    }

    const lang = editor.document.languageId;
    const prompt = `Explain this ${lang} code clearly and concisely:\n\n\`\`\`${lang}\n${selection}\n\`\`\``;

    await askAndShow(prompt);
}

async function cmdRefactorSelection() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No active editor with selection.');
        return;
    }

    const selection = editor.document.getText(editor.selection);
    if (!selection) {
        vscode.window.showWarningMessage('Please select some code first.');
        return;
    }

    const lang = editor.document.languageId;
    const prompt = `Refactor and improve this ${lang} code. Return ONLY the improved code without explanations:\n\n\`\`\`${lang}\n${selection}\n\`\`\``;

    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'LaRuche: Refactoring...',
        cancellable: false,
    }, async () => {
        try {
            const resp = await client.infer(prompt, 'code');
            // Replace selection with refactored code
            const cleaned = resp.response
                .replace(/^```\w*\n?/, '')
                .replace(/\n?```$/, '')
                .trim();

            await editor.edit(editBuilder => {
                editBuilder.replace(editor.selection, cleaned);
            });

            vscode.window.showInformationMessage(
                `LaRuche: Code refactored (${resp.tokens_generated} tokens, ${(resp.latency_ms / 1000).toFixed(1)}s)`
            );
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche: ${err.message}`);
        }
    });
}

async function cmdShowSwarm() {
    try {
        const swarm = await client.swarm();
        const items: vscode.QuickPickItem[] = swarm.nodes.map(n => ({
            label: `$(server) ${n.name || 'Unknown'}`,
            description: `${n.host} | ${n.tokens_per_sec?.toFixed(1) || '?'} t/s | Q:${n.queue_depth || 0}`,
            detail: `Capabilities: ${n.capabilities.join(', ') || 'none'}`,
        }));

        items.unshift({
            label: `$(zap) Collective Power`,
            description: `${swarm.total_nodes} nodes | ${swarm.collective_tps.toFixed(1)} t/s | Queue: ${swarm.collective_queue}`,
            detail: `VRAM: ${formatMB(swarm.total_vram_mb)} | RAM: ${formatMB(swarm.total_ram_mb)}`,
        });

        const selected = await vscode.window.showQuickPick(items, {
            title: 'LaRuche Swarm Intelligence',
            placeHolder: 'Your local AI collective',
        });

    } catch (err: any) {
        vscode.window.showErrorMessage(`LaRuche: Cannot reach node. ${err.message}`);
    }
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
        {
            enableScripts: true,
            retainContextWhenHidden: true,
        },
    );

    chatPanel.webview.html = getChatHtml(chatPanel.webview);

    chatPanel.webview.onDidReceiveMessage(async (msg) => {
        if (msg.type === 'ask') {
            try {
                const resp = await client.infer(msg.prompt);
                chatPanel?.webview.postMessage({
                    type: 'response',
                    text: resp.response,
                    model: resp.model,
                    tokens: resp.tokens_generated,
                    latency: resp.latency_ms,
                    node: resp.node_name,
                });
            } catch (err: any) {
                chatPanel?.webview.postMessage({
                    type: 'error',
                    text: err.message,
                });
            }
        }
    }, undefined, context.subscriptions);

    chatPanel.onDidDispose(() => {
        chatPanel = undefined;
    }, null, context.subscriptions);
}

// ======================== Sidebar Chat View ========================

class ChatViewProvider implements vscode.WebviewViewProvider {
    constructor(private readonly context: vscode.ExtensionContext) { }

    resolveWebviewView(webviewView: vscode.WebviewView) {
        webviewView.webview.options = {
            enableScripts: true,
        };

        webviewView.webview.html = getChatHtml(webviewView.webview);

        webviewView.webview.onDidReceiveMessage(async (msg) => {
            if (msg.type === 'ask') {
                try {
                    const resp = await client.infer(msg.prompt);
                    webviewView.webview.postMessage({
                        type: 'response',
                        text: resp.response,
                        model: resp.model,
                        tokens: resp.tokens_generated,
                        latency: resp.latency_ms,
                        node: resp.node_name,
                    });
                } catch (err: any) {
                    webviewView.webview.postMessage({
                        type: 'error',
                        text: err.message,
                    });
                }
            }
        }, undefined, this.context.subscriptions);
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
            const resp = await client.infer(prompt);
            const doc = await vscode.workspace.openTextDocument({
                content: resp.response,
                language: 'markdown',
            });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);

            vscode.window.showInformationMessage(
                `LaRuche: ${resp.tokens_generated} tokens | ${(resp.latency_ms / 1000).toFixed(1)}s | ${resp.model} @ ${resp.node_name}`
            );
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche: ${err.message}`);
        }
    });
}

function formatMB(mb: number): string {
    if (mb >= 1024) { return (mb / 1024).toFixed(1) + ' GB'; }
    return mb + ' MB';
}
