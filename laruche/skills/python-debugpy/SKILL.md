---
type: skill
name: python-debugpy
description: "Debug Python via pdb REPL and debugpy remote (DAP)."
version: 1.0.0
author: laruche
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec, execute_code, file_edit]
metadata:
  laruche:
    tags: [debugging, python, pdb, debugpy, breakpoints, dap, post-mortem]
    related_skills: [systematic-debugging, node-inspect-debugger]
---

# Python Debugger (pdb + debugpy)

## Tool Selection

| Tool | When |
|---|---|
| **`breakpoint()` + pdb** | Local, interactive. Add `breakpoint()` in source, run normally, get a REPL at that line. |
| **`python -m pdb`** | Launch a script under pdb with no source edits. |
| **`debugpy`** | Remote / headless / attach to already-running process. Talks DAP. |
| **`remote-pdb`** | Terminal-friendly remote debugging — simpler than debugpy when IDE integration is not needed. |

**Default choice: `breakpoint()`.** Use `remote-pdb` for daemons; `debugpy` only when you need IDE (DAP) integration.

**Skip debuggers when:** `print()` / `logging.debug` or `pytest -vv --tb=long --showlocals` would answer the question in under a minute.

## pdb Command Reference

Inside `(Pdb)`:

| Command | Action |
|---|---|
| `n` / `s` / `r` / `c` | next / step-into / return / continue |
| `unt N` | continue until line N |
| `j N` | jump to line N (same function only) |
| `l` / `ll` | list source / full function |
| `w` | stack trace |
| `u` / `d` | up / down in stack |
| `a` | print current function args |
| `p expr` / `pp expr` | print / pretty-print |
| `display expr` | auto-print expr on every stop |
| `b file:line` / `b func` | set breakpoint |
| `b file:line, cond` | conditional breakpoint |
| `cl N` | clear breakpoint N |
| `tbreak file:line` | one-shot breakpoint |
| `!stmt` | execute arbitrary Python (use `!x = 42` to mutate locals) |
| `interact` | full Python REPL in current scope (Ctrl+D to exit) |
| `q` | quit |

`interact` is the most powerful: import anything, inspect complex objects, call methods that mutate state.

## Recipe 1: Local breakpoint

```python
def compute(x, y):
    result = some_helper(x)
    breakpoint()   # drops into pdb here
    return result + y
```

Run normally. Remove before committing:
```bash
rg -n 'breakpoint\(\)' --type py
```

## Recipe 2: Launch under pdb (no source edits)

```bash
python -m pdb path/to/script.py arg1 arg2
(Pdb) b path/to/script.py:42
(Pdb) c
```

## Recipe 3: Debug a pytest test

```bash
# Drop to pdb on failure:
pytest tests/path/to/test_file.py::test_name --pdb

# Drop to pdb at test start:
pytest tests/path/to/test_file.py::test_name --trace

# xdist disables pdb — always disable it:
pytest tests/foo_test.py::test_bar --pdb -p no:xdist
```

If using a test-runner wrapper that enables xdist by default, always pass `-p no:xdist` or `-n 0` when using pdb.

## Recipe 4: Post-mortem on any exception

```python
import pdb, sys
try:
    run_the_thing()
except Exception:
    pdb.post_mortem(sys.exc_info()[2])
```

Or wrap a whole script — pdb catches any uncaught exception and lands in the frame:
```bash
python -m pdb -c continue script.py
```

Or install a global hook:
```python
import sys
def excepthook(etype, value, tb):
    import pdb; pdb.post_mortem(tb)
sys.excepthook = excepthook
```

## Recipe 5: Remote debug with debugpy

Install: `pip install debugpy`

### Pattern A: Source-edit — process waits for client at launch

```python
import debugpy
debugpy.listen(("127.0.0.1", 5678))
print("waiting for debugger...", flush=True)
debugpy.wait_for_client()
debugpy.breakpoint()   # optional: pause immediately on attach
```

### Pattern B: No source edit — launch with `-m debugpy`

```bash
python -m debugpy --listen 127.0.0.1:5678 --wait-for-client your_script.py arg1
python -m debugpy --listen 127.0.0.1:5678 --wait-for-client -m your.module
```

### Pattern C: Attach to already-running process by PID

```bash
python -m debugpy --listen 127.0.0.1:5678 --pid <pid>
```

Fails on hardened kernels (`ptrace_scope=1`). Fix:
```bash
echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope
```

### Connecting a client

**VS Code / Cursor / Zed** — add to `launch.json`:
```json
{
  "name": "Attach",
  "type": "debugpy",
  "request": "attach",
  "connect": { "host": "127.0.0.1", "port": 5678 },
  "justMyCode": false
}
```

**Terminal (no IDE)** — use `remote-pdb` instead (see Recipe 6).

## Recipe 6: remote-pdb (terminal-friendly remote debug)

Preferred over raw debugpy when terminal-only or inside a daemon/subprocess.

```bash
pip install remote-pdb
```

```python
from remote_pdb import set_trace
set_trace(host="127.0.0.1", port=4444)   # blocks until connection
```

Connect:
```bash
nc 127.0.0.1 4444
# Full (Pdb) prompt — identical to local pdb
```

Use this for: daemons, subprocesses, async handlers, any context where you can't attach an IDE.

**Async deadlock pattern:**
```python
import remote_pdb; remote_pdb.set_trace(host="127.0.0.1", port=4444)
# Trigger the handler, then: nc 127.0.0.1 4444
# (Pdb) w   → see suspended frame
# (Pdb) !import asyncio; asyncio.all_tasks()   → see pending tasks
```

## Common Pitfalls

1. **pdb under pytest-xdist hangs silently.** The pdb prompt never appears; the test just blocks. Always add `-p no:xdist` or `-n 0`.

2. **`breakpoint()` in non-TTY / CI hangs the process.** Never commit it. Use a pre-commit grep.

3. **`PYTHONBREAKPOINT=0` silently disables all `breakpoint()` calls.** Check: `echo $PYTHONBREAKPOINT`.

4. **`debugpy.listen` without `wait_for_client()` lets execution continue.** Your first breakpoint may fire before the client attaches.

5. **Attach to PID fails on hardened kernels.** `ptrace_scope=1` (Ubuntu default) only allows ptrace of child processes. Either launch under `debugpy` from the start or temporarily lower `ptrace_scope`.

6. **Threads.** `pdb` only debugs the current thread. For multithreaded code, use `debugpy` (thread-aware DAP) or set `threading.settrace()` per thread.

7. **asyncio + pdb.** `await` inside the pdb prompt requires Python 3.13+. On 3.11/3.12, use `interact` mode or `!asyncio.ensure_future(coro)` workarounds.

8. **Forking / multiprocessing.** pdb does not follow forks. Each child process needs its own `breakpoint()` or `set_trace()`. Debug one process at a time.

9. **Test-runner wrappers may strip credentials or reset `HOME`.** If your bug depends on real API keys or user config, repro with raw `pytest` first, then re-confirm under the wrapper.

## Verification Checklist

- [ ] `python -c "import debugpy; print(debugpy.__version__)"` — confirms debugpy is installed
- [ ] `ss -tlnp | grep 5678` — confirms port is listening before attaching
- [ ] First breakpoint hits (if not: check `PYTHONBREAKPOINT`, xdist, and whether execution finished before attach)
- [ ] `w` shows the expected call stack
- [ ] Cleanup — no stray debug calls in committed code:
  ```bash
  rg -n 'breakpoint\(\)|set_trace\(|debugpy\.listen' --type py
  ```
