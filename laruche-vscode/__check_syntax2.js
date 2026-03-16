const fs = require('fs');
const dist = fs.readFileSync('dist/extension.js', 'utf8');

const bt = String.fromCharCode(96);
const tplStart = dist.indexOf(bt + '<!DOCTYPE');
let end = -1;
for (let i = tplStart + 1; i < dist.length; i++) {
    if (dist[i] === bt[0]) {
        let backslashes = 0;
        let j = i - 1;
        while (j >= 0 && dist[j] === '\\') { backslashes++; j--; }
        if (backslashes % 2 === 0) { end = i; break; }
    }
}

let tpl = dist.substring(tplStart + 1, end);

// Resolve template literal escapes
let resolved = '';
for (let i = 0; i < tpl.length; i++) {
    if (tpl[i] === '\\') {
        i++;
        if (tpl[i] === 'n') resolved += '\n';
        else if (tpl[i] === 't') resolved += '\t';
        else if (tpl[i] === '\\') resolved += '\\';
        else if (tpl[i] === bt[0]) resolved += bt[0];
        else if (tpl[i] === '$') resolved += '$';
        else resolved += tpl[i];
    } else if (tpl[i] === '$' && tpl[i+1] === '{') {
        let depth = 0, j = i + 1;
        while (j < tpl.length) {
            if (tpl[j] === '{') depth++;
            else if (tpl[j] === '}') { depth--; if (depth === 0) break; }
            j++;
        }
        resolved += '"PLACEHOLDER"';
        i = j;
    } else {
        resolved += tpl[i];
    }
}

const scriptStart = resolved.indexOf('<script');
const scriptEnd = resolved.lastIndexOf('</script>');
const scriptTagEnd = resolved.indexOf('>', scriptStart) + 1;
const script = resolved.substring(scriptTagEnd, scriptEnd);

// Write the resolved script to a file for inspection
fs.writeFileSync('__resolved_script.js', script);
console.log('Resolved script written to __resolved_script.js');
console.log('Length:', script.length);

// Check with acorn or just try eval
try {
    new Function(script);
    console.log('Syntax: OK');
} catch(e) {
    console.log('Syntax ERROR:', e.message);

    // Find all regex literals and check them
    const regexPattern = /\/(?:[^\/\\]|\\.)*\/[gimsuy]*/g;
    let match;
    const lines = script.split('\n');

    // Find the line with the error by looking for problematic regex patterns
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        // Check for regex patterns in this line
        const regexes = line.match(/\/(?:[^\/\\]|\\.)*\/[a-z]*/g);
        if (regexes) {
            for (const r of regexes) {
                try {
                    eval(r);
                } catch(e2) {
                    console.log('Bad regex at line ' + (i+1) + ': ' + r);
                    console.log('  Error: ' + e2.message);
                    console.log('  Full line: ' + line.trim().substring(0, 150));
                }
            }
        }
    }
}
