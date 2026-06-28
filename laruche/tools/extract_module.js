// Dev tool: extract a contiguous block of laruche-node/src/main.rs into its own module.
// Pure code move to keep main.rs from becoming a monolith (see ROADMAP #42).
//
// Usage (run from the `laruche/` workspace dir):
//   node tools/extract_module.js <startLine> <endLine> <moduleName> "<doc string>"
//
// Lines are 1-indexed inclusive. The script:
//   - cuts main.rs[start..end] into laruche-node/src/<moduleName>.rs
//   - prepends `use crate::*;` + common axum imports
//   - makes every `async fn` in the block `pub(crate)`
//   - in the remaining main.rs, prefixes route registrations `get(NAME)` / `post(NAME)`
//     with `<moduleName>::NAME`
// Then YOU must: add `mod <moduleName>;` to main.rs, run `cargo check -p laruche-node`,
// and fix any "cannot find function" errors (internal call sites of moved helpers) by
// prefixing them with `<moduleName>::`.
const fs = require('fs');
const MAIN = 'laruche-node/src/main.rs';
const start = parseInt(process.argv[2], 10);
const end = parseInt(process.argv[3], 10);
const mod = process.argv[4];
const doc = process.argv[5] || mod;
if (!start || !end || !mod) { console.error('usage: node tools/extract_module.js <start> <end> <module> "<doc>"'); process.exit(1); }

let lines = fs.readFileSync(MAIN, 'utf8').split('\n');
const block = lines.slice(start - 1, end).join('\n');
const fnNames = [...block.matchAll(/^\s*(?:pub(?:\(crate\))?\s+)?async fn (\w+)/gm)].map(m => m[1]);
let modBody = block.replace(/^(\s*)async fn /gm, '$1pub(crate) async fn ');

let rest = lines.slice(0, start - 1)
  .concat([`// ${doc} -> moved to ${mod}.rs`])
  .concat(lines.slice(end))
  .join('\n');
let prefixed = 0;
for (const f of fnNames) {
  const a = '(' + f + ')';
  if (rest.includes(a)) { rest = rest.split(a).join('(' + mod + '::' + f + ')'); prefixed++; }
}
fs.writeFileSync(MAIN, rest);

const header = `//! ${doc} - split out of main.rs.\n\nuse crate::*;\nuse axum::extract::{Path, Query, State};\nuse axum::response::{IntoResponse, Json};\nuse axum::http::StatusCode;\nuse std::sync::Arc;\n\n`;
fs.writeFileSync(`laruche-node/src/${mod}.rs`, header + modBody + '\n');

console.log(`${mod}.rs: ${block.split('\n').length} lines | fns: ${fnNames.length} | routes prefixed: ${prefixed}`);
console.log('fns:', fnNames.join(', '));
console.log(`NEXT: add \`mod ${mod};\` to main.rs, then \`cargo check -p laruche-node\` and prefix any internal call sites with ${mod}::`);
