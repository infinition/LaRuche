import * as vscode from 'vscode';
import { LaRucheClient } from './client';

export type AgentMode = 'auto' | 'ask' | 'readonly';

interface EditRecord {
    uri: vscode.Uri;
    originalContent: string;
    newContent: string;
    timestamp: number;
    prompt: string;
}

export class AgentProvider {
    private editHistory: EditRecord[] = [];
    private client: LaRucheClient;

    constructor(client: LaRucheClient) {
        this.client = client;
    }

    getMode(): AgentMode {
        return vscode.workspace.getConfiguration('laruche').get<AgentMode>('agentMode', 'ask');
    }

    getHistory(): EditRecord[] {
        return [...this.editHistory];
    }

    /**
     * Run the agent on the current file with the given instructions.
     * Depending on the agent mode:
     * - auto: apply changes immediately
     * - ask: show diff and ask for confirmation
     * - readonly: show suggestion without applying
     */
    async run(editor: vscode.TextEditor, instructions: string): Promise<void> {
        const doc = editor.document;
        const originalContent = doc.getText();
        const fileName = doc.fileName;
        const language = doc.languageId;

        // Build prompt with file context
        const prompt = `Act as an expert software developer. You will receive a file and instructions.
Your task is to emit a diff of the required changes.

Format your response EXACTLY like this:
<<<<
[exact lines to be removed, including whitespace]
====
[new replacement lines]
>>>>

You can specify multiple <<<< ==== >>>> blocks.
IMPORTANT: Include enough context lines in the <<<< block so the search matches exactly one place in the file.
DO NOT return the entire file, only the blocks that change.

File: ${fileName}
Language: ${language}

Instructions: ${instructions}

Current file content:
\`\`\`${language}
${originalContent}
\`\`\`

Diff blocks:`;

        const mode = this.getMode();

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Window,
            title: `LaRuche Agent (${mode})`,
            cancellable: true,
        }, async (progress, token) => {
            progress.report({ message: 'Analyzing...' });

            try {
                const resp = await this.client.infer(prompt, 'code');

                let newContent = originalContent;
                const blockRegex = /<<<<\n([\s\S]*?)\n====\n([\s\S]*?)\n>>>>/g;
                let match;
                let blocksApplied = 0;

                while ((match = blockRegex.exec(resp.response)) !== null) {
                    const search = match[1];
                    const replace = match[2];
                    if (newContent.includes(search)) {
                        newContent = newContent.replace(search, replace);
                        blocksApplied++;
                    } else {
                        vscode.window.showWarningMessage('LaRuche Agent: A diff block failed to match the file exactly.');
                    }
                }

                // Fallback if the model didn't use blocks and just outputted the full file
                if (blocksApplied === 0) {
                    let cleaned = resp.response
                        .replace(/^```\w*\n?/, '')
                        .replace(/\n?```\s*$/, '')
                        .trim();

                    if (cleaned.length > 50 && !cleaned.includes('<<<<')) {
                        newContent = cleaned;
                    } else {
                        vscode.window.showInformationMessage('LaRuche Agent: No applicable changes generated.');
                        return;
                    }
                }

                if (token.isCancellationRequested) { return; }

                switch (mode) {
                    case 'auto':
                        await this.applyEdit(doc, originalContent, newContent, instructions);
                        vscode.window.showInformationMessage(
                            `LaRuche Agent: Changes applied (${resp.tokens_generated} tokens)`,
                            'Undo'
                        ).then(choice => {
                            if (choice === 'Undo') { this.undoLast(); }
                        });
                        break;

                    case 'ask':
                        await this.showDiffAndAsk(doc, originalContent, newContent, instructions, resp.tokens_generated);
                        break;

                    case 'readonly':
                        await this.showSuggestion(doc, originalContent, newContent, language, resp.tokens_generated);
                        break;
                }
            } catch (err: any) {
                vscode.window.showErrorMessage(`LaRuche Agent: ${err.message}`);
            }
        });
    }

    /**
     * Apply an edit directly to the document.
     */
    private async applyEdit(
        doc: vscode.TextDocument,
        originalContent: string,
        newContent: string,
        prompt: string
    ): Promise<void> {
        // Record for undo
        this.editHistory.push({
            uri: doc.uri,
            originalContent,
            newContent,
            timestamp: Date.now(),
            prompt,
        });

        const edit = new vscode.WorkspaceEdit();
        const fullRange = new vscode.Range(
            doc.positionAt(0),
            doc.positionAt(doc.getText().length)
        );
        edit.replace(doc.uri, fullRange, newContent);
        await vscode.workspace.applyEdit(edit);
    }

    /**
     * Show a diff view and ask the user to accept or reject.
     */
    private async showDiffAndAsk(
        doc: vscode.TextDocument,
        originalContent: string,
        newContent: string,
        instructions: string,
        tokens: number
    ): Promise<void> {
        // Create a temp document with the suggestion
        const suggestedUri = vscode.Uri.parse(
            `untitled:${doc.fileName}.suggested`
        );
        const suggestedDoc = await vscode.workspace.openTextDocument({
            content: newContent,
            language: doc.languageId,
        });

        // Show diff
        await vscode.commands.executeCommand(
            'vscode.diff',
            doc.uri,
            suggestedDoc.uri,
            `LaRuche Agent: ${instructions.substring(0, 50)}... (${tokens} tokens)`,
            { preview: true }
        );

        // Ask user
        const choice = await vscode.window.showInformationMessage(
            `LaRuche Agent: Apply these changes?`,
            { modal: false },
            'Accept', 'Reject'
        );

        if (choice === 'Accept') {
            await this.applyEdit(doc, originalContent, newContent, instructions);
            vscode.window.showInformationMessage(
                'LaRuche Agent: Changes applied!',
                'Undo'
            ).then(c => {
                if (c === 'Undo') { this.undoLast(); }
            });
        } else {
            vscode.window.showInformationMessage('LaRuche Agent: Changes rejected.');
        }
    }

    /**
     * Show suggestion in a read-only side panel.
     */
    private async showSuggestion(
        doc: vscode.TextDocument,
        originalContent: string,
        newContent: string,
        language: string,
        tokens: number
    ): Promise<void> {
        const suggestedDoc = await vscode.workspace.openTextDocument({
            content: newContent,
            language,
        });

        await vscode.commands.executeCommand(
            'vscode.diff',
            doc.uri,
            suggestedDoc.uri,
            `LaRuche Suggestion (readonly, ${tokens} tokens)`,
            { preview: true }
        );

        vscode.window.showInformationMessage(
            'LaRuche Agent (readonly): Suggestion shown in diff view. No changes applied.'
        );
    }

    /**
     * Undo the last edit made by the agent.
     */
    async undoLast(): Promise<void> {
        const last = this.editHistory.pop();
        if (!last) {
            vscode.window.showWarningMessage('LaRuche Agent: Nothing to undo.');
            return;
        }

        try {
            const doc = await vscode.workspace.openTextDocument(last.uri);
            const edit = new vscode.WorkspaceEdit();
            const fullRange = new vscode.Range(
                doc.positionAt(0),
                doc.positionAt(doc.getText().length)
            );
            edit.replace(last.uri, fullRange, last.originalContent);
            await vscode.workspace.applyEdit(edit);

            vscode.window.showInformationMessage('LaRuche Agent: Edit undone.');
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche Agent: Failed to undo — ${err.message}`);
        }
    }

    /**
     * Show the edit history as a Quick Pick for selective undo.
     */
    async showHistory(): Promise<void> {
        if (this.editHistory.length === 0) {
            vscode.window.showInformationMessage('LaRuche Agent: No edit history.');
            return;
        }

        const items = this.editHistory.map((r, i) => ({
            label: `$(history) ${r.prompt.substring(0, 60)}...`,
            description: new Date(r.timestamp).toLocaleTimeString(),
            detail: vscode.workspace.asRelativePath(r.uri),
            index: i,
        }));

        items.reverse(); // Most recent first

        const selected = await vscode.window.showQuickPick(items, {
            title: 'LaRuche Agent — Edit History',
            placeHolder: 'Select an edit to revert',
        });

        if (selected) {
            const record = this.editHistory[selected.index];
            try {
                const doc = await vscode.workspace.openTextDocument(record.uri);
                const edit = new vscode.WorkspaceEdit();
                const fullRange = new vscode.Range(
                    doc.positionAt(0),
                    doc.positionAt(doc.getText().length)
                );
                edit.replace(record.uri, fullRange, record.originalContent);
                await vscode.workspace.applyEdit(edit);
                this.editHistory.splice(selected.index, 1);
                vscode.window.showInformationMessage('LaRuche Agent: Edit reverted.');
            } catch (err: any) {
                vscode.window.showErrorMessage(`LaRuche Agent: ${err.message}`);
            }
        }
    }
}
