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
    /** Optional model override (e.g. 'deepseek-coder'). Empty = node default. */
    private model: string | undefined;

    constructor(client: LaRucheClient, model?: string) {
        this.client = client;
        this.model = model || undefined;
    }

    setModel(model: string | undefined): void {
        this.model = model || undefined;
    }

    getMode(): AgentMode {
        return vscode.workspace.getConfiguration('laruche').get<AgentMode>('agentMode', 'ask');
    }

    getHistory(): EditRecord[] {
        return [...this.editHistory];
    }

    /**
     * Run the agent on the active file.
     *
     * The agent sends the full file + instructions to the model and expects a
     * diff in the format:
     *
     *   <<<<
     *   [lines to remove — must match exactly]
     *   ====
     *   [replacement lines]
     *   >>>>
     *
     * Multiple blocks are supported. Falls back to full-file replacement if the
     * model returns the whole file instead of diff blocks.
     */
    async run(editor: vscode.TextEditor, instructions: string): Promise<void> {
        const doc = editor.document;
        const originalContent = doc.getText();
        const fileName = vscode.workspace.asRelativePath(doc.uri);
        const language = doc.languageId;

        const prompt = `You are an expert software developer making precise edits to a file.

Return ONLY the changed sections using this diff format:
<<<<
[exact existing lines to replace, including all whitespace]
====
[new replacement lines]
>>>>

Rules:
- You MAY emit multiple <<<< ==== >>>> blocks for multiple changes.
- Include 2-3 lines of surrounding context in the <<<< block so it matches exactly ONE location.
- Do NOT return the entire file.
- Do NOT add markdown fences or explanations outside the diff blocks.
- If no changes are needed, respond with: NO_CHANGES

File: ${fileName}
Language: ${language}
Instructions: ${instructions}

Current file content:
\`\`\`${language}
${originalContent}
\`\`\`

Diff:`;

        const mode = this.getMode();

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Window,
            title: `LaRuche Agent (${mode})${this.model ? ` · ${this.model}` : ''}`,
            cancellable: true,
        }, async (progress, token) => {
            progress.report({ message: 'Analyzing…' });

            try {
                const resp = await this.client.infer(prompt, 'code', this.model);

                if (token.isCancellationRequested) { return; }

                const rawResponse = resp.response.trim();

                if (rawResponse === 'NO_CHANGES' || rawResponse.toUpperCase().includes('NO_CHANGES')) {
                    vscode.window.showInformationMessage('LaRuche Agent: No changes needed.');
                    return;
                }

                let newContent = originalContent;
                let blocksApplied = 0;
                let blocksFailed = 0;

                // Parse all <<<< ==== >>>> blocks
                // Use a flexible regex that handles CRLF and trailing whitespace variations
                const blockRegex = /<<<<\r?\n([\s\S]*?)\r?\n====\r?\n([\s\S]*?)\r?\n>>>>/g;
                let match: RegExpExecArray | null;

                while ((match = blockRegex.exec(rawResponse)) !== null) {
                    const search = match[1];
                    const replace = match[2];

                    if (newContent.includes(search)) {
                        newContent = newContent.replace(search, replace);
                        blocksApplied++;
                    } else {
                        // Try with normalized line endings
                        const searchNorm = search.replace(/\r\n/g, '\n');
                        const contentNorm = newContent.replace(/\r\n/g, '\n');
                        if (contentNorm.includes(searchNorm)) {
                            newContent = newContent.replace(/\r\n/g, '\n').replace(searchNorm, replace);
                            blocksApplied++;
                        } else {
                            blocksFailed++;
                        }
                    }
                }

                if (blocksFailed > 0) {
                    vscode.window.showWarningMessage(
                        `LaRuche Agent: ${blocksFailed} diff block(s) failed to match — the model may have produced imprecise context.`,
                    );
                }

                // Fallback: if no diff blocks found, check if the model returned the full file
                if (blocksApplied === 0) {
                    const cleaned = rawResponse
                        .replace(/^```[\w-]*\r?\n?/, '')
                        .replace(/\r?\n?```\s*$/, '')
                        .trim();

                    // Heuristic: if it looks like a full file (≥50 chars, no diff markers)
                    if (cleaned.length >= 50 && !cleaned.includes('<<<<')) {
                        const proceed = await vscode.window.showWarningMessage(
                            'LaRuche Agent: Model returned full file instead of diff. Apply as full replacement?',
                            'Apply', 'Cancel',
                        );
                        if (proceed !== 'Apply') { return; }
                        newContent = cleaned;
                    } else {
                        vscode.window.showInformationMessage('LaRuche Agent: No applicable changes generated.');
                        return;
                    }
                }

                if (newContent === originalContent) {
                    vscode.window.showInformationMessage('LaRuche Agent: Content unchanged after applying diff.');
                    return;
                }

                switch (mode) {
                    case 'auto':
                        await this.applyEdit(doc, originalContent, newContent, instructions);
                        vscode.window.showInformationMessage(
                            `LaRuche Agent: ${blocksApplied || 1} change(s) applied (${resp.tokens_generated} tokens · ${resp.model})`,
                            'Undo',
                        ).then(choice => {
                            if (choice === 'Undo') { this.undoLast(); }
                        });
                        break;

                    case 'ask':
                        await this.showDiffAndAsk(doc, originalContent, newContent, instructions, resp.tokens_generated);
                        break;

                    case 'readonly':
                        await this.showSuggestion(doc, newContent, language, resp.tokens_generated);
                        break;
                }
            } catch (err: any) {
                if (!token.isCancellationRequested) {
                    vscode.window.showErrorMessage(`LaRuche Agent: ${err.message}`);
                }
            }
        });
    }

    private async applyEdit(
        doc: vscode.TextDocument,
        originalContent: string,
        newContent: string,
        prompt: string,
    ): Promise<void> {
        this.editHistory.push({
            uri: doc.uri,
            originalContent,
            newContent,
            timestamp: Date.now(),
            prompt,
        });

        const edit = new vscode.WorkspaceEdit();
        const fullRange = new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length));
        edit.replace(doc.uri, fullRange, newContent);
        await vscode.workspace.applyEdit(edit);
    }

    private async showDiffAndAsk(
        doc: vscode.TextDocument,
        originalContent: string,
        newContent: string,
        instructions: string,
        tokens: number,
    ): Promise<void> {
        const suggestedDoc = await vscode.workspace.openTextDocument({
            content: newContent,
            language: doc.languageId,
        });

        await vscode.commands.executeCommand(
            'vscode.diff',
            doc.uri,
            suggestedDoc.uri,
            `LaRuche Agent: ${instructions.slice(0, 50)}… (${tokens} tokens)`,
            { preview: true },
        );

        const choice = await vscode.window.showInformationMessage(
            'LaRuche Agent: Apply these changes?',
            { modal: false },
            'Accept', 'Reject',
        );

        if (choice === 'Accept') {
            await this.applyEdit(doc, originalContent, newContent, instructions);
            vscode.window.showInformationMessage('LaRuche Agent: Changes applied!', 'Undo')
                .then(c => { if (c === 'Undo') { this.undoLast(); } });
        } else {
            vscode.window.showInformationMessage('LaRuche Agent: Changes discarded.');
        }
    }

    private async showSuggestion(
        doc: vscode.TextDocument,
        newContent: string,
        language: string,
        tokens: number,
    ): Promise<void> {
        const suggestedDoc = await vscode.workspace.openTextDocument({ content: newContent, language });
        await vscode.commands.executeCommand(
            'vscode.diff',
            doc.uri,
            suggestedDoc.uri,
            `LaRuche Suggestion (readonly · ${tokens} tokens)`,
            { preview: true },
        );
        vscode.window.showInformationMessage('LaRuche Agent (readonly): Diff shown — no changes applied.');
    }

    async undoLast(): Promise<void> {
        const last = this.editHistory.pop();
        if (!last) {
            vscode.window.showWarningMessage('LaRuche Agent: Nothing to undo.');
            return;
        }
        try {
            const doc = await vscode.workspace.openTextDocument(last.uri);
            const edit = new vscode.WorkspaceEdit();
            const fullRange = new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length));
            edit.replace(last.uri, fullRange, last.originalContent);
            await vscode.workspace.applyEdit(edit);
            vscode.window.showInformationMessage('LaRuche Agent: Edit undone.');
        } catch (err: any) {
            vscode.window.showErrorMessage(`LaRuche Agent: Failed to undo — ${err.message}`);
        }
    }

    async showHistory(): Promise<void> {
        if (this.editHistory.length === 0) {
            vscode.window.showInformationMessage('LaRuche Agent: No edit history.');
            return;
        }

        const items = [...this.editHistory].reverse().map((r, i) => ({
            label: `$(history) ${r.prompt.slice(0, 60)}`,
            description: new Date(r.timestamp).toLocaleTimeString(),
            detail: vscode.workspace.asRelativePath(r.uri),
            index: this.editHistory.length - 1 - i,
        }));

        const selected = await vscode.window.showQuickPick(items, {
            title: 'LaRuche Agent — Edit History',
            placeHolder: 'Select an edit to revert',
        });

        if (selected) {
            const record = this.editHistory[selected.index];
            try {
                const doc = await vscode.workspace.openTextDocument(record.uri);
                const edit = new vscode.WorkspaceEdit();
                const fullRange = new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length));
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
