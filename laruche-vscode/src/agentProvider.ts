import * as vscode from 'vscode';
import * as path from 'path';
import { LaRucheClient } from './client';

export type AgentMode = 'auto' | 'ask' | 'readonly';

// ── Types ────────────────────────────────────────────────────────────────────

interface EditRecord {
    uri: vscode.Uri;
    originalContent: string;
    newContent: string;
    timestamp: number;
    prompt: string;
}

interface ToolCall {
    name: string;
    args: Record<string, any>;
}

interface ToolResult {
    name: string;
    success: boolean;
    result: string;
}

/** Pending file change that can be previewed before applying. */
interface PendingFileChange {
    uri: vscode.Uri;
    originalContent: string;
    newContent: string;
    description: string;
}

/** Callback to send progress updates to the chat webview. */
export type AgentProgressCallback = (msg: object) => void;

/** Attachments provided by the user (files dragged / attached in chat). */
export interface AgentAttachment {
    name: string;
    content: string;
    language?: string;
}

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_ITERATIONS = 15;
const MAX_FILE_READ_SIZE = 80_000;   // chars
const MAX_SEARCH_RESULTS = 40;

// ── System prompt ────────────────────────────────────────────────────────────

function buildSystemPrompt(): string {
    const root = getWorkspaceRoot();
    const rootName = root ? path.basename(root) : '';
    return `You are an expert coding agent running inside VS Code.
You have access to the user's workspace and can read, write, search and list files.

## Workspace
${root ? `Workspace root: ${root}\nAll relative paths are resolved from this root.` : 'No workspace folder is open.'}

**IMPORTANT path rules:**
- Use paths **relative to the workspace root**. Do NOT include the workspace folder name itself.
${rootName ? `- The workspace folder is called "${rootName}". Do NOT prefix paths with "${rootName}/".
  ✗ WRONG: "${rootName}/_WIP/file.md"
  ✓ RIGHT: "_WIP/file.md"` : ''}
- You can also use absolute paths (starting with / or a drive letter like C:\\).
- Use forward slashes (/) in paths, not backslashes.

## Available tools

Call tools by emitting ONE OR MORE <tool_call> blocks in your response.
Each block must contain a valid JSON object with "name" and "args".

<tool_call>
{"name": "tool_name", "args": {"arg1": "value1"}}
</tool_call>

### read_file
Read the contents of a file in the workspace.
Args: path (string, required) — workspace-relative or absolute path.

### write_file
Create or completely overwrite a file.
Args: path (string, required), content (string, required).

### edit_file
Make a surgical text replacement inside a file (search-and-replace).
The old_text must match EXACTLY (including whitespace and indentation).
Include enough surrounding context (2-3 lines) so old_text matches only ONE location.
Args: path (string, required), old_text (string, required), new_text (string, required).

### list_files
List files in a directory with an optional glob pattern.
Args: directory (string, required — relative or absolute), pattern (string, optional — glob like "*.ts").

### search_files
Search for text across workspace files (case-insensitive).
Args: query (string, required), path (string, optional — restrict to a subdirectory).

### batch_read_files
Read multiple files at once in a single call (saves iterations).
Args: paths (array of strings, required — list of file paths to read, max 10).

### find_and_read
List files matching a pattern AND read their contents in one call.
This is much more efficient than calling list_files then read_file separately.
Args: directory (string, required), pattern (string, optional — glob like "*.md"), max_files (number, optional — default 5, max 10).

### project_structure
Get a tree view of the project directory structure.
Use this first when you need to understand the codebase layout.
Args: directory (string, optional — default "."), max_depth (number, optional — default 3, max 5).

### move
Rename or move a file or directory. Also use this for renaming.
Args: source (string, required), destination (string, required).
Example: rename file.txt → file_new.txt, or move src/old.ts → src/utils/old.ts.

### copy
Copy a file or directory to a new location.
Args: source (string, required), destination (string, required).

### delete
Delete a file or directory (moves to trash for safety).
Args: path (string, required).

### create_directory
Create a directory and any missing parent directories.
Args: path (string, required).

### file_info
Get info about a file or directory (size, type, timestamps).
Args: path (string, required).

## Rules
- Think step by step. Plan before coding.
- You may call MULTIPLE tools in a single response — just include multiple <tool_call> blocks.
- After you receive tool results, continue reasoning and make more tool calls if needed.
- When you are DONE (no more tool calls needed), write your final summary in plain text with NO <tool_call> blocks.
- For file edits, prefer edit_file over write_file when modifying existing files.
- Always read a file before editing it unless you just created it.
- When creating new files, use write_file.
- Never output <tool_call> blocks inside markdown code fences.
- If a tool returns an error about file not found, check your path — remember to use workspace-relative paths without the workspace folder name prefix.
- **Multi-file tasks**: Use find_and_read or batch_read_files to read multiple files at once — this is MUCH faster than reading files one by one. Use project_structure first to understand the codebase layout.
`;
}

// ── Tool implementations ─────────────────────────────────────────────────────

function getWorkspaceRoot(): string | undefined {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

/**
 * Resolve a file path to an absolute URI.
 *
 * Handles several common issues:
 * - Absolute paths → used directly
 * - Workspace-relative paths → joined with workspace root
 * - Paths that accidentally include the workspace folder name as prefix
 *   (e.g. "coding/_WIP/foo" when root is ".../coding") → stripped & resolved
 * - Backslash vs forward slash normalization
 */
function resolveWorkspacePath(filePath: string): vscode.Uri {
    const root = getWorkspaceRoot();

    // Normalize separators
    const normalized = filePath.replace(/\\/g, '/');

    if (path.isAbsolute(filePath)) {
        return vscode.Uri.file(filePath);
    }

    if (!root) {
        return vscode.Uri.file(filePath);
    }

    // Check if the relative path accidentally starts with the workspace folder name.
    // Example: workspace root is "C:/DEV/coding", LLM sends "coding/_WIP/test.md"
    //   → should resolve to "C:/DEV/coding/_WIP/test.md", not "C:/DEV/coding/coding/_WIP/test.md"
    const rootName = path.basename(root);
    if (normalized.startsWith(rootName + '/')) {
        const stripped = normalized.slice(rootName.length + 1);
        const candidate = path.join(root, stripped);
        // Prefer the stripped version — but we'll validate at read time
        return vscode.Uri.file(candidate);
    }

    return vscode.Uri.file(path.join(root, normalized));
}

/**
 * Try multiple path candidates to find one that exists.
 * Returns the first existing URI, or the original resolved URI as fallback.
 */
async function resolveWithFallback(filePath: string): Promise<vscode.Uri> {
    const root = getWorkspaceRoot();
    const normalized = filePath.replace(/\\/g, '/');

    // Build candidate list (ordered by likelihood)
    const candidates: vscode.Uri[] = [];

    // 1. Direct resolve (handles absolute + smart stripping)
    candidates.push(resolveWorkspacePath(filePath));

    if (root) {
        const rootName = path.basename(root);

        // 2. If we stripped the prefix, also try the non-stripped version
        if (normalized.startsWith(rootName + '/')) {
            candidates.push(vscode.Uri.file(path.join(root, normalized)));
        } else {
            // 3. If we didn't strip, try stripping in case the folder structure
            //    has a matching subfolder
            const stripped = normalized.startsWith(rootName + '/')
                ? normalized.slice(rootName.length + 1)
                : normalized;
            if (stripped !== normalized) {
                candidates.push(vscode.Uri.file(path.join(root, stripped)));
            }
        }

        // 4. Try as plain workspace-relative
        candidates.push(vscode.Uri.file(path.join(root, normalized)));
    }

    // 5. Absolute as-is
    if (path.isAbsolute(filePath)) {
        candidates.push(vscode.Uri.file(filePath));
    }

    // Dedupe by fsPath
    const seen = new Set<string>();
    const unique: vscode.Uri[] = [];
    for (const c of candidates) {
        if (!seen.has(c.fsPath)) {
            seen.add(c.fsPath);
            unique.push(c);
        }
    }

    // Return first one that exists
    for (const uri of unique) {
        try {
            await vscode.workspace.fs.stat(uri);
            return uri;
        } catch {
            // doesn't exist, try next
        }
    }

    // None exist — return the primary candidate (caller will get the ENOENT)
    return unique[0];
}

async function toolReadFile(args: Record<string, any>): Promise<ToolResult> {
    const filePath = args.path;
    if (!filePath) {
        return { name: 'read_file', success: false, result: 'Missing required arg: path' };
    }
    try {
        const uri = await resolveWithFallback(filePath);
        const data = await vscode.workspace.fs.readFile(uri);
        let content = Buffer.from(data).toString('utf8');
        if (content.length > MAX_FILE_READ_SIZE) {
            content = content.slice(0, MAX_FILE_READ_SIZE) + `\n\n... [truncated at ${MAX_FILE_READ_SIZE} chars]`;
        }
        return { name: 'read_file', success: true, result: content };
    } catch (err: any) {
        // Provide a helpful hint about the workspace root
        const root = getWorkspaceRoot() || '(unknown)';
        return { name: 'read_file', success: false, result: `Error reading file "${filePath}": ${err.message}.\nWorkspace root is: ${root}\nUse paths relative to the workspace root (e.g. "_WIP/file.md" not "${path.basename(root)}/_WIP/file.md").` };
    }
}

async function toolWriteFile(args: Record<string, any>): Promise<ToolResult> {
    const filePath = args.path;
    const content = args.content;
    if (!filePath || content === undefined) {
        return { name: 'write_file', success: false, result: 'Missing required args: path, content' };
    }
    try {
        const uri = resolveWorkspacePath(filePath); // write_file: use direct resolve (don't need fallback for new files)
        // Ensure parent directory exists
        const parentDir = vscode.Uri.file(path.dirname(uri.fsPath));
        try { await vscode.workspace.fs.stat(parentDir); } catch {
            await vscode.workspace.fs.createDirectory(parentDir);
        }
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf8'));
        return { name: 'write_file', success: true, result: `File written: ${filePath} (${content.length} chars)` };
    } catch (err: any) {
        return { name: 'write_file', success: false, result: `Error writing file: ${err.message}` };
    }
}

async function toolEditFile(args: Record<string, any>): Promise<ToolResult> {
    const filePath = args.path;
    const oldText = args.old_text;
    const newText = args.new_text;
    if (!filePath || oldText === undefined || newText === undefined) {
        return { name: 'edit_file', success: false, result: 'Missing required args: path, old_text, new_text' };
    }
    try {
        const uri = await resolveWithFallback(filePath);
        const data = await vscode.workspace.fs.readFile(uri);
        let content = Buffer.from(data).toString('utf8');

        if (content.includes(oldText)) {
            content = content.replace(oldText, newText);
            await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf8'));
            return { name: 'edit_file', success: true, result: `Edit applied to ${filePath}` };
        }

        // Try normalized line endings
        const oldNorm = oldText.replace(/\r\n/g, '\n');
        const contentNorm = content.replace(/\r\n/g, '\n');
        if (contentNorm.includes(oldNorm)) {
            content = contentNorm.replace(oldNorm, newText);
            await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf8'));
            return { name: 'edit_file', success: true, result: `Edit applied to ${filePath} (normalized line endings)` };
        }

        return { name: 'edit_file', success: false, result: `old_text not found in ${filePath}. Make sure it matches exactly (including whitespace). Read the file first to see its current content.` };
    } catch (err: any) {
        return { name: 'edit_file', success: false, result: `Error editing file: ${err.message}` };
    }
}

async function toolListFiles(args: Record<string, any>): Promise<ToolResult> {
    const dir = args.directory || '.';
    const pattern = args.pattern || '*';
    try {
        // Try smart resolution first, then fallback
        let uri: vscode.Uri;
        try {
            uri = await resolveWithFallback(dir);
        } catch {
            uri = resolveWorkspacePath(dir);
        }

        // Verify directory exists
        try {
            const stat = await vscode.workspace.fs.stat(uri);
            if (stat.type !== vscode.FileType.Directory) {
                // It's a file — list its parent
                uri = vscode.Uri.file(path.dirname(uri.fsPath));
            }
        } catch {
            // Directory doesn't exist — provide helpful error
            const root = getWorkspaceRoot() || '(unknown)';
            return {
                name: 'list_files',
                success: false,
                result: `Directory "${dir}" not found. Resolved to: ${uri.fsPath}\nWorkspace root: ${root}\nUse paths relative to root without the workspace folder name prefix.`,
            };
        }

        const globPattern = new vscode.RelativePattern(uri, pattern === '*' ? '**/*' : `**/${pattern}`);
        const files = await vscode.workspace.findFiles(globPattern, '**/node_modules/**', 200);
        const root = getWorkspaceRoot() || '';
        const relativePaths = files.map(f => {
            const rel = path.relative(root, f.fsPath);
            return rel || f.fsPath;
        }).sort();
        if (relativePaths.length === 0) {
            return { name: 'list_files', success: true, result: `No files found in ${dir} matching "${pattern}". Directory resolved to: ${uri.fsPath}` };
        }
        const listing = relativePaths.join('\n');
        return { name: 'list_files', success: true, result: `Found ${relativePaths.length} files:\n${listing}` };
    } catch (err: any) {
        return { name: 'list_files', success: false, result: `Error listing files: ${err.message}` };
    }
}

async function toolSearchFiles(args: Record<string, any>): Promise<ToolResult> {
    const query = args.query;
    const searchPath = args.path;
    if (!query) {
        return { name: 'search_files', success: false, result: 'Missing required arg: query' };
    }
    try {
        // Use VS Code's built-in text search API
        const root = getWorkspaceRoot();
        if (!root) {
            return { name: 'search_files', success: false, result: 'No workspace folder open' };
        }

        let searchUri: vscode.Uri | undefined;
        if (searchPath) {
            try {
                searchUri = await resolveWithFallback(searchPath);
            } catch {
                searchUri = resolveWorkspacePath(searchPath);
            }
        }
        const include = searchUri
            ? new vscode.RelativePattern(searchUri, '**/*')
            : undefined;

        const results: string[] = [];

        await new Promise<void>((resolve) => {
            vscode.workspace.findFiles(
                include || '**/*',
                '**/node_modules/**',
                100,
            ).then(async (files) => {
                for (const file of files) {
                    if (results.length >= MAX_SEARCH_RESULTS) { break; }
                    try {
                        const data = await vscode.workspace.fs.readFile(file);
                        const text = Buffer.from(data).toString('utf8');
                        const lines = text.split('\n');
                        for (let i = 0; i < lines.length; i++) {
                            if (lines[i].toLowerCase().includes(query.toLowerCase())) {
                                const rel = path.relative(root, file.fsPath);
                                results.push(`${rel}:${i + 1}: ${lines[i].trim().slice(0, 200)}`);
                                if (results.length >= MAX_SEARCH_RESULTS) { break; }
                            }
                        }
                    } catch {
                        // skip binary/unreadable files
                    }
                }
                resolve();
            });
        });

        if (results.length === 0) {
            return { name: 'search_files', success: true, result: `No matches found for "${query}"` };
        }
        return {
            name: 'search_files',
            success: true,
            result: `Found ${results.length} match(es):\n${results.join('\n')}`,
        };
    } catch (err: any) {
        return { name: 'search_files', success: false, result: `Error searching: ${err.message}` };
    }
}

/**
 * Read multiple files in a single tool call.
 * Returns concatenated contents with clear file separators.
 */
async function toolBatchReadFiles(args: Record<string, any>): Promise<ToolResult> {
    const paths: string[] = args.paths;
    if (!paths || !Array.isArray(paths) || paths.length === 0) {
        return { name: 'batch_read_files', success: false, result: 'Missing required arg: paths (array of file paths)' };
    }

    const MAX_BATCH = 10;
    const filesToRead = paths.slice(0, MAX_BATCH);
    const results: string[] = [];
    let totalChars = 0;
    const MAX_TOTAL = MAX_FILE_READ_SIZE * 2; // Allow more for batch

    for (const filePath of filesToRead) {
        if (totalChars >= MAX_TOTAL) {
            results.push(`\n━━━ ${filePath} ━━━\n[SKIPPED — total size limit reached]`);
            continue;
        }
        try {
            const uri = await resolveWithFallback(filePath);
            const data = await vscode.workspace.fs.readFile(uri);
            let content = Buffer.from(data).toString('utf8');
            const remaining = MAX_TOTAL - totalChars;
            if (content.length > remaining) {
                content = content.slice(0, remaining) + `\n... [truncated]`;
            }
            totalChars += content.length;
            results.push(`\n━━━ ${filePath} (${content.length} chars) ━━━\n${content}`);
        } catch (err: any) {
            results.push(`\n━━━ ${filePath} ━━━\n[ERROR: ${err.message}]`);
        }
    }

    if (paths.length > MAX_BATCH) {
        results.push(`\n[NOTE: Only first ${MAX_BATCH} files read, ${paths.length - MAX_BATCH} skipped]`);
    }

    return { name: 'batch_read_files', success: true, result: `Read ${filesToRead.length} file(s):${results.join('\n')}` };
}

/**
 * List files in a directory and read them all in one operation.
 * Combines list_files + batch read to save iterations.
 */
async function toolFindAndRead(args: Record<string, any>): Promise<ToolResult> {
    const dir = args.directory || '.';
    const pattern = args.pattern || '*';
    const maxFiles = Math.min(args.max_files || 5, 10);

    try {
        let uri: vscode.Uri;
        try {
            uri = await resolveWithFallback(dir);
        } catch {
            uri = resolveWorkspacePath(dir);
        }

        // Verify directory exists
        try {
            const stat = await vscode.workspace.fs.stat(uri);
            if (stat.type !== vscode.FileType.Directory) {
                uri = vscode.Uri.file(path.dirname(uri.fsPath));
            }
        } catch {
            const root = getWorkspaceRoot() || '(unknown)';
            return {
                name: 'find_and_read',
                success: false,
                result: `Directory "${dir}" not found. Workspace root: ${root}`,
            };
        }

        const globStr = pattern === '*' ? '**/*' : `**/${pattern}`;
        const globPattern = new vscode.RelativePattern(uri, globStr);
        const files = await vscode.workspace.findFiles(globPattern, '**/node_modules/**', maxFiles * 2);
        const root = getWorkspaceRoot() || '';

        if (files.length === 0) {
            return { name: 'find_and_read', success: true, result: `No files found in "${dir}" matching "${pattern}".` };
        }

        // Sort by path and take up to maxFiles
        const sorted = files
            .map(f => ({ uri: f, rel: path.relative(root, f.fsPath) }))
            .sort((a, b) => a.rel.localeCompare(b.rel))
            .slice(0, maxFiles);

        const results: string[] = [];
        let totalChars = 0;
        const MAX_TOTAL = MAX_FILE_READ_SIZE;

        results.push(`Found ${files.length} file(s) matching "${pattern}" in "${dir}", reading ${sorted.length}:\n`);

        for (const { uri: fileUri, rel } of sorted) {
            if (totalChars >= MAX_TOTAL) {
                results.push(`\n━━━ ${rel} ━━━\n[SKIPPED — size limit reached]`);
                continue;
            }
            try {
                const data = await vscode.workspace.fs.readFile(fileUri);
                let content = Buffer.from(data).toString('utf8');
                const remaining = MAX_TOTAL - totalChars;
                if (content.length > remaining) {
                    content = content.slice(0, remaining) + '\n... [truncated]';
                }
                totalChars += content.length;
                results.push(`\n━━━ ${rel} (${content.length} chars) ━━━\n${content}`);
            } catch {
                results.push(`\n━━━ ${rel} ━━━\n[ERROR: could not read file]`);
            }
        }

        if (files.length > sorted.length) {
            results.push(`\n[${files.length - sorted.length} more file(s) not shown — increase max_files to see more]`);
        }

        return { name: 'find_and_read', success: true, result: results.join('\n') };
    } catch (err: any) {
        return { name: 'find_and_read', success: false, result: `Error: ${err.message}` };
    }
}

/**
 * Get an overview of the project structure (directory tree).
 * Helps the LLM understand the codebase layout in one call.
 */
async function toolProjectStructure(args: Record<string, any>): Promise<ToolResult> {
    const dir = args.directory || '.';
    const maxDepth = Math.min(args.max_depth || 3, 5);

    try {
        let rootUri: vscode.Uri;
        try {
            rootUri = await resolveWithFallback(dir);
        } catch {
            rootUri = resolveWorkspacePath(dir);
        }

        const root = getWorkspaceRoot() || '';
        const globPattern = new vscode.RelativePattern(rootUri, '**/*');
        const files = await vscode.workspace.findFiles(globPattern, '{**/node_modules/**,**/.git/**,**/target/**,**/dist/**,**/__pycache__/**}', 500);

        // Build tree structure
        const tree = new Map<string, Set<string>>();
        for (const f of files) {
            const rel = path.relative(rootUri.fsPath, f.fsPath).replace(/\\/g, '/');
            const parts = rel.split('/');
            if (parts.length > maxDepth + 1) { continue; }

            for (let i = 0; i < parts.length; i++) {
                const dirPath = i === 0 ? '.' : parts.slice(0, i).join('/');
                if (!tree.has(dirPath)) { tree.set(dirPath, new Set()); }
                tree.get(dirPath)!.add(parts[i]);
            }
        }

        // Format as tree
        const lines: string[] = [];
        const relRoot = dir === '.' ? path.basename(root || '.') : dir;
        lines.push(`${relRoot}/`);

        function renderDir(dirPath: string, indent: string): void {
            const children = tree.get(dirPath);
            if (!children) { return; }
            const sorted = [...children].sort((a, b) => {
                // Directories first
                const aIsDir = tree.has(dirPath === '.' ? a : `${dirPath}/${a}`);
                const bIsDir = tree.has(dirPath === '.' ? b : `${dirPath}/${b}`);
                if (aIsDir !== bIsDir) { return aIsDir ? -1 : 1; }
                return a.localeCompare(b);
            });
            for (let i = 0; i < sorted.length; i++) {
                const name = sorted[i];
                const childPath = dirPath === '.' ? name : `${dirPath}/${name}`;
                const isLast = i === sorted.length - 1;
                const prefix = isLast ? '└── ' : '├── ';
                const isDir = tree.has(childPath);
                lines.push(`${indent}${prefix}${name}${isDir ? '/' : ''}`);
                if (isDir) {
                    renderDir(childPath, indent + (isLast ? '    ' : '│   '));
                }
            }
        }

        renderDir('.', '');

        return {
            name: 'project_structure',
            success: true,
            result: `Project structure (depth ${maxDepth}):\n${lines.join('\n')}\n\nTotal: ${files.length} files`,
        };
    } catch (err: any) {
        return { name: 'project_structure', success: false, result: `Error: ${err.message}` };
    }
}

/**
 * Rename or move a file/directory.
 */
async function toolMove(args: Record<string, any>): Promise<ToolResult> {
    const src = args.source;
    const dst = args.destination;
    if (!src || !dst) {
        return { name: 'move', success: false, result: 'Missing required args: source, destination' };
    }
    try {
        const srcUri = await resolveWithFallback(src);
        const dstUri = resolveWorkspacePath(dst);
        // Ensure destination parent exists
        const parentDir = vscode.Uri.file(path.dirname(dstUri.fsPath));
        try { await vscode.workspace.fs.stat(parentDir); } catch {
            await vscode.workspace.fs.createDirectory(parentDir);
        }
        await vscode.workspace.fs.rename(srcUri, dstUri, { overwrite: false });
        return { name: 'move', success: true, result: `Moved: ${src} → ${dst}` };
    } catch (err: any) {
        return { name: 'move', success: false, result: `Error moving "${src}" to "${dst}": ${err.message}` };
    }
}

/**
 * Copy a file or directory.
 */
async function toolCopy(args: Record<string, any>): Promise<ToolResult> {
    const src = args.source;
    const dst = args.destination;
    if (!src || !dst) {
        return { name: 'copy', success: false, result: 'Missing required args: source, destination' };
    }
    try {
        const srcUri = await resolveWithFallback(src);
        const dstUri = resolveWorkspacePath(dst);
        const parentDir = vscode.Uri.file(path.dirname(dstUri.fsPath));
        try { await vscode.workspace.fs.stat(parentDir); } catch {
            await vscode.workspace.fs.createDirectory(parentDir);
        }
        await vscode.workspace.fs.copy(srcUri, dstUri, { overwrite: false });
        return { name: 'copy', success: true, result: `Copied: ${src} → ${dst}` };
    } catch (err: any) {
        return { name: 'copy', success: false, result: `Error copying "${src}" to "${dst}": ${err.message}` };
    }
}

/**
 * Delete a file or directory.
 */
async function toolDelete(args: Record<string, any>): Promise<ToolResult> {
    const filePath = args.path;
    if (!filePath) {
        return { name: 'delete', success: false, result: 'Missing required arg: path' };
    }
    try {
        const uri = await resolveWithFallback(filePath);
        const stat = await vscode.workspace.fs.stat(uri);
        const isDir = stat.type === vscode.FileType.Directory;
        await vscode.workspace.fs.delete(uri, { recursive: isDir, useTrash: true });
        return { name: 'delete', success: true, result: `Deleted (moved to trash): ${filePath}${isDir ? ' (directory)' : ''}` };
    } catch (err: any) {
        return { name: 'delete', success: false, result: `Error deleting "${filePath}": ${err.message}` };
    }
}

/**
 * Create a directory (and any missing parents).
 */
async function toolCreateDirectory(args: Record<string, any>): Promise<ToolResult> {
    const dirPath = args.path;
    if (!dirPath) {
        return { name: 'create_directory', success: false, result: 'Missing required arg: path' };
    }
    try {
        const uri = resolveWorkspacePath(dirPath);
        await vscode.workspace.fs.createDirectory(uri);
        return { name: 'create_directory', success: true, result: `Directory created: ${dirPath}` };
    } catch (err: any) {
        return { name: 'create_directory', success: false, result: `Error creating directory "${dirPath}": ${err.message}` };
    }
}

/**
 * Get file/directory info (size, type, timestamps).
 */
async function toolFileInfo(args: Record<string, any>): Promise<ToolResult> {
    const filePath = args.path;
    if (!filePath) {
        return { name: 'file_info', success: false, result: 'Missing required arg: path' };
    }
    try {
        const uri = await resolveWithFallback(filePath);
        const stat = await vscode.workspace.fs.stat(uri);
        const typeStr = stat.type === vscode.FileType.Directory ? 'directory'
            : stat.type === vscode.FileType.SymbolicLink ? 'symlink'
            : 'file';
        const sizeKB = (stat.size / 1024).toFixed(1);
        const modified = new Date(stat.mtime).toISOString();
        const created = new Date(stat.ctime).toISOString();
        return {
            name: 'file_info',
            success: true,
            result: `Path: ${uri.fsPath}\nType: ${typeStr}\nSize: ${stat.size} bytes (${sizeKB} KB)\nModified: ${modified}\nCreated: ${created}`,
        };
    } catch (err: any) {
        return { name: 'file_info', success: false, result: `Error getting info for "${filePath}": ${err.message}` };
    }
}

const TOOL_HANDLERS: Record<string, (args: Record<string, any>) => Promise<ToolResult>> = {
    read_file: toolReadFile,
    write_file: toolWriteFile,
    edit_file: toolEditFile,
    list_files: toolListFiles,
    search_files: toolSearchFiles,
    batch_read_files: toolBatchReadFiles,
    find_and_read: toolFindAndRead,
    project_structure: toolProjectStructure,
    move: toolMove,
    copy: toolCopy,
    delete: toolDelete,
    create_directory: toolCreateDirectory,
    file_info: toolFileInfo,
};

// ── Parsing ──────────────────────────────────────────────────────────────────

function parseToolCalls(text: string): ToolCall[] {
    const calls: ToolCall[] = [];

    // 1. Primary: <tool_call>...</tool_call> blocks
    const tagRegex = /<tool_call>\s*([\s\S]*?)\s*<\/tool_call>/g;
    let match: RegExpExecArray | null;
    while ((match = tagRegex.exec(text)) !== null) {
        try {
            const parsed = JSON.parse(match[1].trim());
            if (parsed.name && typeof parsed.name === 'string') {
                calls.push({
                    name: parsed.name,
                    args: parsed.args || {},
                });
            }
        } catch {
            // Skip malformed tool calls
        }
    }

    // 2. Fallback: JSON in ```json code blocks (some models wrap tool calls this way)
    if (calls.length === 0) {
        const codeBlockRegex = /```(?:json)?\s*\n?\s*(\{[\s\S]*?\})\s*\n?\s*```/g;
        while ((match = codeBlockRegex.exec(text)) !== null) {
            try {
                const parsed = JSON.parse(match[1].trim());
                if (parsed.name && typeof parsed.name === 'string' && TOOL_HANDLERS[parsed.name]) {
                    calls.push({
                        name: parsed.name,
                        args: parsed.args || {},
                    });
                }
            } catch {
                // Not a tool call JSON, skip
            }
        }
    }

    // 3. Fallback: bare JSON objects with "name" that match known tools
    //    (some models output raw JSON without any wrapping)
    if (calls.length === 0) {
        const bareJsonRegex = /\{\s*"name"\s*:\s*"(\w+)"\s*,\s*"args"\s*:\s*\{[^}]*\}\s*\}/g;
        while ((match = bareJsonRegex.exec(text)) !== null) {
            try {
                const parsed = JSON.parse(match[0]);
                if (parsed.name && TOOL_HANDLERS[parsed.name]) {
                    calls.push({
                        name: parsed.name,
                        args: parsed.args || {},
                    });
                }
            } catch {
                // skip
            }
        }
    }

    return calls;
}

function stripToolCalls(text: string): string {
    let stripped = text;
    // Remove <tool_call> blocks
    stripped = stripped.replace(/<tool_call>\s*[\s\S]*?\s*<\/tool_call>/g, '');
    // Remove JSON code blocks that look like tool calls
    stripped = stripped.replace(/```(?:json)?\s*\n?\s*\{\s*"name"\s*:[\s\S]*?\}\s*\n?\s*```/g, '');
    // Remove bare JSON tool call objects (only if they match known tool names)
    stripped = stripped.replace(/\{\s*"name"\s*:\s*"(?:read_file|write_file|edit_file|list_files|search_files|batch_read_files|find_and_read|project_structure|move|copy|delete|create_directory|file_info)"\s*,\s*"args"\s*:\s*\{[^}]*\}\s*\}/g, '');
    return stripped.trim();
}

function formatToolResults(results: ToolResult[]): string {
    return results.map(r =>
        `<tool_result name="${r.name}" success="${r.success}">\n${r.result}\n</tool_result>`
    ).join('\n\n');
}

// ── Legacy diff parsing (kept as fallback) ───────────────────────────────────

function parseDiffBlocks(text: string, originalContent: string): { newContent: string; applied: number; failed: number } {
    let newContent = originalContent;
    let applied = 0;
    let failed = 0;

    const blockRegex = /<<<<\r?\n([\s\S]*?)\r?\n====\r?\n([\s\S]*?)\r?\n>>>>/g;
    let match: RegExpExecArray | null;

    while ((match = blockRegex.exec(text)) !== null) {
        const search = match[1];
        const replace = match[2];

        if (newContent.includes(search)) {
            newContent = newContent.replace(search, replace);
            applied++;
        } else {
            const searchNorm = search.replace(/\r\n/g, '\n');
            const contentNorm = newContent.replace(/\r\n/g, '\n');
            if (contentNorm.includes(searchNorm)) {
                newContent = newContent.replace(/\r\n/g, '\n').replace(searchNorm, replace);
                applied++;
            } else {
                failed++;
            }
        }
    }

    return { newContent, applied, failed };
}

// ── Agent Provider ───────────────────────────────────────────────────────────

/** A single turn in the conversation history (user message + agent response). */
interface ConversationTurn {
    userPrompt: string;
    agentResponse: string;
    timestamp: number;
}

const MAX_HISTORY_TURNS = 10;
const MAX_HISTORY_CHARS = 20_000; // Keep history compact to avoid blowing context

export class AgentProvider {
    private editHistory: EditRecord[] = [];
    private conversationHistory: ConversationTurn[] = [];
    private client: LaRucheClient;
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

    /** Clear conversation history (e.g. when user starts a "new conversation"). */
    clearConversation(): void {
        this.conversationHistory = [];
    }

    // ── Main agent loop (new) ────────────────────────────────────────────────

    /**
     * Run the full agentic loop from the chat panel.
     *
     * The agent sends messages to the LLM, parses tool calls, executes them,
     * feeds results back, and repeats until the LLM is done or we hit
     * MAX_ITERATIONS.
     *
     * @param userPrompt  The user's instruction
     * @param attachments Files attached by the user (content included in context)
     * @param onProgress  Callback for streaming progress to the chat UI
     * @returns The agent's final text answer (after all tool rounds)
     */
    async runAgentLoop(
        userPrompt: string,
        attachments: AgentAttachment[],
        onProgress: AgentProgressCallback,
    ): Promise<{ finalText: string; totalTokens: number; model: string; iterations: number }> {

        const mode = this.getMode();

        // Build the conversation as a growing prompt string
        // (since the /infer endpoint takes a single prompt, not message arrays)
        let conversation = buildSystemPrompt() + '\n';

        // Conversation history (previous turns for context)
        if (this.conversationHistory.length > 0) {
            conversation += '\n## Previous conversation\n\n';
            // Include recent turns, trimming if too long
            let historyChars = 0;
            const recentTurns = [...this.conversationHistory].reverse();
            const includedTurns: ConversationTurn[] = [];
            for (const turn of recentTurns) {
                const turnSize = turn.userPrompt.length + turn.agentResponse.length;
                if (historyChars + turnSize > MAX_HISTORY_CHARS) { break; }
                historyChars += turnSize;
                includedTurns.unshift(turn);
            }
            for (const turn of includedTurns) {
                conversation += `**User:** ${turn.userPrompt}\n\n**Assistant:** ${turn.agentResponse}\n\n---\n\n`;
            }
        }

        // Attachments
        if (attachments.length > 0) {
            conversation += '\n## Attached files\n\n';
            for (const att of attachments) {
                const langHint = att.language ? ` (${att.language})` : '';
                conversation += `### ${att.name}${langHint}\n\`\`\`\n${att.content}\n\`\`\`\n\n`;
            }
        }

        // Active editor context (include if open)
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            const fileName = vscode.workspace.asRelativePath(editor.document.uri);
            const lang = editor.document.languageId;
            const content = editor.document.getText();
            if (content.length < MAX_FILE_READ_SIZE) {
                conversation += `\n## Currently open file: ${fileName} (${lang})\n\`\`\`${lang}\n${content}\n\`\`\`\n\n`;
            } else {
                conversation += `\n## Currently open file: ${fileName} (${lang}) — too large to include, use read_file to access it.\n\n`;
            }
        }

        conversation += `\n## User request\n${userPrompt}\n\n## Your response\n`;

        let totalTokens = 0;
        let lastModel = '';
        let iteration = 0;

        while (iteration < MAX_ITERATIONS) {
            iteration++;

            onProgress({ type: 'agentProgress', text: `Iteration ${iteration}/${MAX_ITERATIONS} — thinking...` });

            // Call the LLM
            let resp;
            try {
                resp = await this.client.infer(conversation, 'code', this.model);
            } catch (err: any) {
                onProgress({ type: 'agentProgress', text: `LLM error: ${err.message}` });
                throw err;
            }

            totalTokens += resp.tokens_generated;
            lastModel = resp.model;

            const rawResponse = resp.response.trim();

            // Handle empty responses — retry once with a nudge
            if (!rawResponse) {
                onProgress({ type: 'agentProgress', text: 'Empty response from model, retrying...' });
                conversation += '[The model returned an empty response. Please try again and provide a useful answer. If you need to use a tool, emit a <tool_call> block.]\n\n';
                continue;
            }

            // Parse tool calls (try <tool_call> tags first, then code blocks as fallback)
            const toolCalls = parseToolCalls(rawResponse);
            const textPart = stripToolCalls(rawResponse);

            // If no tool calls, the agent is done
            if (toolCalls.length === 0) {
                // Check for legacy diff blocks as a fallback
                if (rawResponse.includes('<<<<') && rawResponse.includes('>>>>') && editor) {
                    await this.handleLegacyDiff(editor, rawResponse, userPrompt, onProgress);
                }
                const finalText = textPart || rawResponse;
                // Save turn to conversation history
                this.saveTurn(userPrompt, finalText);
                return { finalText, totalTokens, model: lastModel, iterations: iteration };
            }

            // Append the LLM's response to the conversation
            conversation += rawResponse + '\n\n';

            // Show what tools are being called
            const toolNames = toolCalls.map(tc => tc.name).join(', ');
            onProgress({ type: 'agentProgress', text: `Calling tools: ${toolNames}` });

            // Deduplicate: if model emits both edit_file and write_file for the same path,
            // keep only the edit_file (more surgical). Same for other redundant combos.
            const seenWritePaths = new Set<string>();
            const deduped = toolCalls.filter(tc => {
                const p = tc.args.path || tc.args.source || '';
                if (tc.name === 'write_file' && seenWritePaths.has(p)) {
                    return false; // Skip write_file if we already have an edit_file for this path
                }
                if (tc.name === 'edit_file' || tc.name === 'write_file') {
                    seenWritePaths.add(p);
                }
                return true;
            });

            // Execute all tool calls
            const results: ToolResult[] = [];
            for (const tc of deduped) {
                const handler = TOOL_HANDLERS[tc.name];
                if (!handler) {
                    results.push({ name: tc.name, success: false, result: `Unknown tool: ${tc.name}` });
                    continue;
                }

                // In 'ask' mode, confirm destructive operations with the user
                const WRITE_TOOLS = new Set(['write_file', 'edit_file']);
                const DESTRUCTIVE_TOOLS = new Set(['move', 'copy', 'delete', 'create_directory']);

                if (mode === 'ask' && WRITE_TOOLS.has(tc.name)) {
                    const approved = await this.previewAndConfirmChange(tc, onProgress);
                    if (!approved) {
                        results.push({ name: tc.name, success: false, result: 'User rejected the change.' });
                        continue;
                    }
                }

                if (mode === 'ask' && DESTRUCTIVE_TOOLS.has(tc.name)) {
                    const desc = tc.name === 'move' ? `Move ${tc.args.source} → ${tc.args.destination}`
                        : tc.name === 'copy' ? `Copy ${tc.args.source} → ${tc.args.destination}`
                        : tc.name === 'delete' ? `Delete ${tc.args.path}`
                        : `Create directory ${tc.args.path}`;
                    const choice = await vscode.window.showInformationMessage(
                        `LaRuche Agent: ${desc}`,
                        { modal: false },
                        'Accept', 'Reject',
                    );
                    if (choice !== 'Accept') {
                        results.push({ name: tc.name, success: false, result: 'User rejected the operation.' });
                        continue;
                    }
                }

                try {
                    const result = await handler(tc.args);
                    results.push(result);

                    // Track edit history for undo
                    if (result.success && (tc.name === 'write_file' || tc.name === 'edit_file')) {
                        this.trackEdit(tc);
                    }

                    // Report each tool result
                    const preview = result.result.length > 200
                        ? result.result.slice(0, 200) + '...'
                        : result.result;
                    onProgress({
                        type: 'agentToolResult',
                        tool: tc.name,
                        success: result.success,
                        preview,
                    });
                } catch (err: any) {
                    results.push({ name: tc.name, success: false, result: `Execution error: ${err.message}` });
                }
            }

            // Append tool results to conversation
            conversation += formatToolResults(results) + '\n\n';

            // If we have text alongside tool calls, show it as intermediate reasoning
            if (textPart) {
                onProgress({ type: 'agentThinking', text: textPart });
            }
        }

        // Hit max iterations
        onProgress({ type: 'agentProgress', text: `Reached max iterations (${MAX_ITERATIONS}). Stopping.` });
        const maxIterText = 'Agent reached the maximum number of iterations. The task may be incomplete.';
        this.saveTurn(userPrompt, maxIterText);
        return {
            finalText: maxIterText,
            totalTokens,
            model: lastModel,
            iterations: MAX_ITERATIONS,
        };
    }

    /** Save a conversation turn for future context. */
    private saveTurn(userPrompt: string, agentResponse: string): void {
        this.conversationHistory.push({
            userPrompt,
            agentResponse: agentResponse.slice(0, 2000), // Keep responses compact
            timestamp: Date.now(),
        });
        // Trim old turns
        while (this.conversationHistory.length > MAX_HISTORY_TURNS) {
            this.conversationHistory.shift();
        }
    }

    // ── Preview and confirm (ask mode) ───────────────────────────────────────

    private async previewAndConfirmChange(tc: ToolCall, onProgress: AgentProgressCallback): Promise<boolean> {
        const filePath = tc.args.path;
        const uri = resolveWorkspacePath(filePath);

        let originalContent = '';
        try {
            const data = await vscode.workspace.fs.readFile(uri);
            originalContent = Buffer.from(data).toString('utf8');
        } catch {
            // New file — no preview needed for write_file
            if (tc.name === 'write_file') { return true; }
        }

        let proposedContent = '';
        if (tc.name === 'write_file') {
            proposedContent = tc.args.content;
        } else if (tc.name === 'edit_file') {
            const oldText = tc.args.old_text;
            const newText = tc.args.new_text;
            if (originalContent.includes(oldText)) {
                proposedContent = originalContent.replace(oldText, newText);
            } else {
                const oldNorm = oldText.replace(/\r\n/g, '\n');
                const contentNorm = originalContent.replace(/\r\n/g, '\n');
                if (contentNorm.includes(oldNorm)) {
                    proposedContent = contentNorm.replace(oldNorm, newText);
                } else {
                    // Can't preview — let the tool handler report the error
                    return true;
                }
            }
        }

        // Show VS Code diff
        try {
            const origDoc = await vscode.workspace.openTextDocument({ content: originalContent });
            const proposedDoc = await vscode.workspace.openTextDocument({ content: proposedContent });
            await vscode.commands.executeCommand(
                'vscode.diff',
                origDoc.uri,
                proposedDoc.uri,
                `LaRuche Agent: ${tc.name} — ${filePath}`,
                { preview: true },
            );
        } catch {
            // Fall back to simple confirmation if diff fails
        }

        const choice = await vscode.window.showInformationMessage(
            `LaRuche Agent wants to ${tc.name === 'write_file' ? 'write' : 'edit'}: ${filePath}`,
            { modal: false },
            'Accept', 'Reject',
        );

        return choice === 'Accept';
    }

    // ── Edit tracking ────────────────────────────────────────────────────────

    private async trackEdit(tc: ToolCall): Promise<void> {
        const filePath = tc.args.path;
        const uri = resolveWorkspacePath(filePath);
        try {
            // Read back the file to capture the new state
            const data = await vscode.workspace.fs.readFile(uri);
            const newContent = Buffer.from(data).toString('utf8');
            this.editHistory.push({
                uri,
                originalContent: tc.name === 'write_file' ? '' : (tc.args.old_text || ''),
                newContent,
                timestamp: Date.now(),
                prompt: `${tc.name}: ${filePath}`,
            });
        } catch {
            // Ignore tracking errors
        }
    }

    // ── Legacy single-file agent (kept for the command-palette flow) ─────────

    /**
     * Run the legacy agent on the active file (command palette).
     * This is a simpler flow: send file + instructions, get diff back.
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
            title: `LaRuche Agent (${mode})${this.model ? ` \u00B7 ${this.model}` : ''}`,
            cancellable: true,
        }, async (progress, token) => {
            progress.report({ message: 'Analyzing\u2026' });

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

                const blockRegex = /<<<<\r?\n([\s\S]*?)\r?\n====\r?\n([\s\S]*?)\r?\n>>>>/g;
                let match: RegExpExecArray | null;

                while ((match = blockRegex.exec(rawResponse)) !== null) {
                    const search = match[1];
                    const replace = match[2];

                    if (newContent.includes(search)) {
                        newContent = newContent.replace(search, replace);
                        blocksApplied++;
                    } else {
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
                        `LaRuche Agent: ${blocksFailed} diff block(s) failed to match.`,
                    );
                }

                if (blocksApplied === 0) {
                    const cleaned = rawResponse
                        .replace(/^```[\w-]*\r?\n?/, '')
                        .replace(/\r?\n?```\s*$/, '')
                        .trim();

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
                            `LaRuche Agent: ${blocksApplied || 1} change(s) applied (${resp.tokens_generated} tokens \u00B7 ${resp.model})`,
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

    // ── Handle legacy diff blocks coming from the agent loop ─────────────────

    private async handleLegacyDiff(
        editor: vscode.TextEditor,
        rawResponse: string,
        instructions: string,
        onProgress: AgentProgressCallback,
    ): Promise<void> {
        const doc = editor.document;
        const originalContent = doc.getText();
        const { newContent, applied, failed } = parseDiffBlocks(rawResponse, originalContent);

        if (applied === 0) { return; }

        const mode = this.getMode();
        if (failed > 0) {
            onProgress({ type: 'agentProgress', text: `${failed} diff block(s) failed to match.` });
        }

        if (newContent !== originalContent) {
            if (mode === 'auto') {
                await this.applyEdit(doc, originalContent, newContent, instructions);
            } else if (mode === 'ask') {
                await this.showDiffAndAsk(doc, originalContent, newContent, instructions, 0);
            }
        }
    }

    // ── Shared helpers ───────────────────────────────────────────────────────

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
            `LaRuche Agent: ${instructions.slice(0, 50)}\u2026 (${tokens} tokens)`,
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
            `LaRuche Suggestion (readonly \u00B7 ${tokens} tokens)`,
            { preview: true },
        );
        vscode.window.showInformationMessage('LaRuche Agent (readonly): Diff shown \u2014 no changes applied.');
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
            vscode.window.showErrorMessage(`LaRuche Agent: Failed to undo \u2014 ${err.message}`);
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
            title: 'LaRuche Agent \u2014 Edit History',
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
