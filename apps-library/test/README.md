# Real-host App integration tests

These tests create isolated temporary LaRuche data homes and launch their own node. They do
not enroll users on a running hive or use its provider credentials. Temporary test homes and
screenshots are retained for inspection. Close the tests before rebuilding their executable
on Windows.

Requirements: Node, Playwright, Chromium and a built LaRuche node. Set `NODE_PATH` if Playwright
is supplied by a separate runtime. Optional `CHROME_PATH` selects a browser;
`LARUCHE_TEST_BINARY` selects the node executable (otherwise the debug binary is used).

## Bidirectional bridge

```powershell
python apps-library/2048/build.py
python apps-library/checkers/build.py
node apps-library/test/agent-bridge.test.cjs
```

Uses a controlled local streaming provider, not a real LLM. Covers installation consent,
permissions UI, agent library, opening/Ready, valid and stale game moves, two independent
agent contexts, user isolation and live revocation during a model request.
Also checks that the checkers agent can play white when the human chooses black,
and that it cannot play the subsequent human turn.

## DS Studio persistence

Supply an independently built DS Studio package using the `dev.laruche.ds-studio` action
contract. The test consumes the archive as-is and does not modify the App source:

```powershell
node apps-library/test/ds-persistence.test.cjs C:/Packages/dev.laruche.ds-studio-1.3.0.laruche-app
```

Checks actual manifest validation, the opaque iframe (direct localStorage is unavailable),
Ready, dataset import, cell execution/job polling, a yearly sales aggregation and its chart.
It waits until the cell/chart reach disk-backed storage, closes the browser context, stops
and restarts its node, then opens a fresh browser context with only the test login cookie.
The notebook id, source, calculated table/chart and small dataset must all be restored.
Requires DS Studio 1.3.0 or later for `storage.status`/`storage.flush`; checks dataset
revision changes and paginated CSV export too.

This verifies the built-in notebook engine and the existing small JSON storage contract. It
does not validate Pyodide, large binary files, a real LLM, sudden power loss or backups. It
does not prove persistence for datasets above DS Studio's current save quota. Large-dataset
storage remains a separate host capability to implement.
