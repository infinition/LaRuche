const fs = require('fs');
const dist = fs.readFileSync('dist/extension.js', 'utf8');

// Find getChatHtml and simulate calling it with a mock webview
// to get the actual HTML output
const mockWebview = {
    cspSource: 'mock-csp'
};

// Extract the getChatHtml function body
// It uses template literal with ${nonce} and ${webview.cspSource}
// Let's just find the script tag content in the compiled output

// The template literal in dist starts with `<!DOCTYPE
const bt = String.fromCharCode(96); // backtick
const tplStart = dist.indexOf(bt + '<!DOCTYPE');
if (tplStart === -1) { console.log('Template not found'); process.exit(1); }

// Find matching closing backtick - need to handle escaped backticks
let end = -1;
for (let i = tplStart + 1; i < dist.length; i++) {
    if (dist[i] === bt[0]) {
        // Check if it's escaped
        let backslashes = 0;
        let j = i - 1;
        while (j >= 0 && dist[j] === '\\') { backslashes++; j--; }
        if (backslashes % 2 === 0) {
            end = i;
            break;
        }
    }
}

if (end === -1) { console.log('End of template not found'); process.exit(1); }

let tpl = dist.substring(tplStart + 1, end);
console.log('Template length:', tpl.length);

// Resolve template literal escapes: \` -> ` and \\ -> \ and \n -> newline etc.
// But we need to be careful about ${} interpolations
let resolved = '';
for (let i = 0; i < tpl.length; i++) {
    if (tpl[i] === '\\') {
        i++;
        if (tpl[i] === 'n') resolved += '\n';
        else if (tpl[i] === 't') resolved += '\t';
        else if (tpl[i] === '\\') resolved += '\\';
        else if (tpl[i] === bt[0]) resolved += bt[0];
        else if (tpl[i] === '$') resolved += '$';
        else resolved += tpl[i]; // just keep the char
    } else if (tpl[i] === '$' && tpl[i+1] === '{') {
        // Skip template interpolation - replace with placeholder
        let depth = 0;
        let j = i + 1;
        while (j < tpl.length) {
            if (tpl[j] === '{') depth++;
            else if (tpl[j] === '}') { depth--; if (depth === 0) break; }
            j++;
        }
        resolved += 'PLACEHOLDER';
        i = j;
    } else {
        resolved += tpl[i];
    }
}

// Extract script content
const scriptStart = resolved.indexOf('<script');
const scriptEnd = resolved.lastIndexOf('</script>');
if (scriptStart === -1 || scriptEnd === -1) {
    console.log('Script tags not found');
    process.exit(1);
}
const scriptTagEnd = resolved.indexOf('>', scriptStart) + 1;
const script = resolved.substring(scriptTagEnd, scriptEnd);

console.log('Script length:', script.length);
console.log('First 200 chars:', script.substring(0, 200));

// Try syntax check
try {
    new Function(script);
    console.log('\nSyntax: OK');
} catch(e) {
    console.log('\nSyntax ERROR:', e.message);
    // Try to find the problematic line
    const lines = script.split('\n');
    for (let i = 0; i < lines.length; i++) {
        try {
            new Function(lines.slice(0, i + 1).join('\n') + '\n}');
        } catch(e2) {
            if (e2.message.includes('Invalid regular expression') ||
                e2.message.includes('Unexpected token') ||
                e2.message.includes('Invalid or unexpected')) {
                console.log('Problem around line ' + (i + 1) + ':');
                console.log('  ' + lines[i].trim().substring(0, 150));
                console.log('  Error: ' + e2.message);
                // Show context
                for (let k = Math.max(0, i - 2); k <= Math.min(lines.length - 1, i + 2); k++) {
                    console.log('  ' + (k === i ? '>>>' : '   ') + ' L' + (k+1) + ': ' + lines[k].trim().substring(0, 120));
                }
                break;
            }
        }
    }
}
