---
type: skill
name: node-inspect-debugger
description: "Debug Node.js via --inspect + Chrome DevTools Protocol CLI."
version: 1.0.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, file_write]
metadata:
  laruche:
    tags: [debugging, nodejs, node-inspect, cdp, breakpoints]
    related_skills: [systematic-debugging, python-debugpy]
---

# Node.js Inspect Debugger

Drive Node's built-in V8 inspector from the terminal: real breakpoints, step in/over/out, call-stack walking, scope dumps, and expression evaluation in the paused frame.

Two tools — pick one:

- **`node inspect`** — built-in, zero install, interactive REPL. Best for quick poking.
- **CDP via `chrome-remote-interface`** — scriptable; use when automating many breakpoints or collecting state across runs.

**Prefer `node inspect` first.** Always available, no install.

**Skip entirely** when `console.log` solves the problem in under a minute.

## Quick Reference: `node inspect` REPL

Launch paused on first line:

```bash
node inspect path/to/script.js
# TypeScript via tsx:
node --inspect-brk --import tsx script.ts
```

The `debug>` prompt:

| Command | Action |
|---|---|
| `c` / `cont` | continue |
| `n` / `next` | step over |
| `s` / `step` | step into |
| `o` / `out` | step out |
| `pause` | pause running code |
| `sb('file.js', 42)` | set breakpoint at file.js:42 |
| `sb(42)` | set breakpoint at current file:42 |
| `sb('fnName')` | break on function entry |
| `cb('file.js', 42)` | clear breakpoint |
| `breakpoints` | list all breakpoints |
| `bt` | backtrace (call stack) |
| `list(5)` | show 5 source lines around current position |
| `watch('expr')` | evaluate expr on every pause |
| `watchers` | show watched expressions |
| `repl` | drop into REPL in current scope (Ctrl+C to exit) |
| `exec expr` | evaluate expression once |
| `restart` | restart script |
| `kill` | kill the script |
| `.exit` | quit debugger |

In `repl` sub-mode: type any JS expression to access locals/closures. `Ctrl+C` exits back to `debug>`.

## Attaching to a Running Process

```bash
# Enable inspector on an already-running process
kill -SIGUSR1 <pid>
# Node prints: Debugger listening on ws://127.0.0.1:9229/<uuid>

# Attach by PID or URL
node inspect -p <pid>
node inspect ws://127.0.0.1:9229/<uuid>
```

Start a process with the inspector from the beginning:

```bash
node --inspect script.js           # listen on 127.0.0.1:9229, keep running
node --inspect-brk script.js       # listen AND pause on first line
node --inspect=0.0.0.0:9230 script.js   # custom host:port
```

## Programmatic CDP (automation)

Use `chrome-remote-interface` to automate breakpoints, capture scope state, or script a repro.

```bash
# Install to a throwaway location to avoid dirtying your project
mkdir -p /tmp/cdp-tools && cd /tmp/cdp-tools && npm i chrome-remote-interface

# Start target
node --inspect-brk=9229 target.js &
```

Write the driver script via `file_write` or `shell_exec`:

```javascript
// /tmp/cdp-tools/debug.js
const CDP = require('chrome-remote-interface');

(async () => {
  const client = await CDP({ port: 9229 });
  const { Debugger, Runtime } = client;

  Debugger.paused(async ({ callFrames, reason }) => {
    const top = callFrames[0];
    console.log(`PAUSED: ${reason} @ ${top.url}:${top.location.lineNumber + 1}`);

    // Walk local/closure scopes
    for (const scope of top.scopeChain) {
      if (scope.type === 'local' || scope.type === 'closure') {
        const { result } = await Runtime.getProperties({
          objectId: scope.object.objectId,
          ownProperties: true,
        });
        for (const p of result) {
          console.log(`  ${scope.type}.${p.name} =`, p.value?.value ?? p.value?.description);
        }
      }
    }

    // Evaluate in paused frame
    const { result } = await Debugger.evaluateOnCallFrame({
      callFrameId: top.callFrameId,
      expression: 'typeof state !== "undefined" ? JSON.stringify(state) : "n/a"',
    });
    console.log('state =', result.value ?? result.description);

    await Debugger.resume();
  });

  await Runtime.enable();
  await Debugger.enable();

  // Set breakpoint by URL regex + line (0-indexed)
  await Debugger.setBreakpointByUrl({ urlRegex: '.*app\\.js$', lineNumber: 119, columnNumber: 0 });

  await Runtime.runIfWaitingForDebugger();
})();
```

Run it:

```bash
NODE_PATH=/tmp/cdp-tools/node_modules node /tmp/cdp-tools/debug.js
```

## Running Tests Under the Debugger

```bash
# Vitest — single file, paused on entry
node --inspect-brk ./node_modules/vitest/vitest.mjs run --no-file-parallelism src/foo.test.ts

# Jest
node --inspect-brk ./node_modules/jest/bin/jest.js --runInBand src/foo.test.ts
```

Attach in another terminal: `node inspect -p <pid>`, set breakpoints, `cont`.

Use `--no-file-parallelism` (vitest) or `--runInBand` (jest) — debugging a worker pool is painful.

## Heap Snapshots & CPU Profiles

Swap `Debugger` for `HeapProfiler` / `Profiler` in the CDP driver:

```javascript
// CPU profile for 5 seconds
await client.Profiler.enable();
await client.Profiler.start();
await new Promise(r => setTimeout(r, 5000));
const { profile } = await client.Profiler.stop();
require('fs').writeFileSync('/tmp/cpu.cpuprofile', JSON.stringify(profile));
// Open in Chrome DevTools → Performance tab

// Heap snapshot
await client.HeapProfiler.enable();
const chunks = [];
client.HeapProfiler.addHeapSnapshotChunk(({ chunk }) => chunks.push(chunk));
await client.HeapProfiler.takeHeapSnapshot({ reportProgress: false });
require('fs').writeFileSync('/tmp/heap.heapsnapshot', chunks.join(''));
// Open in Chrome DevTools → Memory tab
```

## Common Pitfalls

1. **Wrong line numbers in TS.** Breakpoints hit the emitted JS. Either break in `dist/*.js`, or enable sourcemaps (`node --enable-source-maps`) and use CDP (`node inspect` does not follow sourcemaps).

2. **`--inspect` vs `--inspect-brk`.** `--inspect` doesn't pause; your script races past the first breakpoint if you attach too late. Use `--inspect-brk` to pause before any code runs.

3. **Port collisions.** Default is `9229`. Pass `--inspect=0` for a random port; read the actual URL:
   ```bash
   curl -s http://127.0.0.1:9229/json/list
   ```

4. **Child processes.** `--inspect` on a parent does NOT inspect its children. Use `NODE_OPTIONS='--inspect-brk' node parent.js` to propagate; children get auto-incremented ports.

5. **Stuck target on Ctrl+C.** If you exit `node inspect` while the target is paused, the target stays paused. Run `cont` before exiting, or `kill` the target explicitly.

6. **Security.** `--inspect=0.0.0.0:9229` exposes arbitrary code execution. Always bind to `127.0.0.1` (the default) unless on an isolated network.

## Verification Checklist

- `curl -s http://127.0.0.1:9229/json/list` returns the expected target
- First breakpoint actually hits (if not: missing `--inspect-brk`, or attached after execution completed)
- Source listing at pause shows the right file (mismatch = sourcemap issue, see pitfall 1)
- `exec process.pid` in `repl` returns the PID you intended to attach to

## One-Shot Recipes

**"Why is this variable undefined at line X?"**
```bash
node --inspect-brk script.js &
node inspect -p $!
# debug> sb('script.js', X)
# debug> cont
# paused. Then:
# debug> repl
# > myVariable
```

**"What's the call path into this function?"**
```
debug> sb('suspectFn')
debug> cont
# paused on function entry
debug> bt
```

**"This async chain hangs — where?"**
```bash
node --inspect script.js   # no -brk, let it run to the hang
# In another terminal:
node inspect -p <pid>
# debug> pause
# debug> bt
```
