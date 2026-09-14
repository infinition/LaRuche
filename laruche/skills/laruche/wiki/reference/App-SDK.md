# The App SDK

Every capability an [App](Apps) has reaches it through this one object. The page receives
no cookies and no general HTTP client: the host transfers a private `MessagePort` to that
exact frame, and `LaRucheApp` is the only thing on the other end.

Load it before your own script, from the host rather than from the package:

```html
<script src="/apps-runtime/v1.js" defer></script>
<script src="./app.js" defer></script>
```

`LaRucheAddon` is kept as an alias of the same object, for packages written before the
format was renamed.

## Waiting for the bridge

```javascript
const context = await LaRucheApp.ready();
```

Nothing else works before that promise resolves. It rejects if the bridge has not
connected within five seconds, which is the case to handle rather than ignore: it means
the frame was opened outside LaRuche, or the App was disabled while loading.

The resolved context is frozen and carries:

| Field | Contents |
|---|---|
| `sessionId` | This instance's session, which the bridge checks on every message |
| `appId` | The App identifier |
| `appVersion` | The version actually running |
| `viewId` | Which of the manifest's views this frame is |
| `grantedCapabilities` | The permissions the user granted, as an array |
| `locale` | The interface language, or `null` without `ui.locale.read` |
| `theme` | The interface theme, or `null` without `ui.theme.read` |
| `limits` | `messageBytes`, `concurrentRequests`, `requestsPerMinute`, `storageBytes` |

Read `grantedCapabilities` rather than assuming: a permission declared `optional` in the
manifest may well have been refused, and the App is expected to keep working without it.

`LaRucheApp.version` is the SDK's own version string.

## Errors

Every method returns a promise that rejects with an `Error` carrying two extra fields:
`code`, and `retryable` saying whether the same call is worth attempting again.

| Code | Meaning |
|---|---|
| `permission_denied` | The permission was not granted, or was revoked while the App was running |
| `authentication_required` | No signed-in user behind the request |
| `validation_failed` | The arguments did not pass the host's checks |
| `not_found` | The key, path or node does not exist |
| `app_not_found` | The App is no longer registered |
| `app_disabled` | The App was disabled, or updated to another version |
| `quota_exceeded` | A storage or folder limit was reached |
| `conflict` | The request id is already in flight |
| `rate_limited` | Too many calls, see the limits below |
| `storage_unavailable` | Private storage could not answer |
| `file_unavailable` | The App folder could not answer |
| `memory_unavailable` | Cognitive memory could not answer |
| `timeout` | The host did not answer in time |
| `service_unavailable` | The bridge is not connected |
| `internal_error` | Anything the host could not classify |

A revoked permission surfaces as `permission_denied` on the next call rather than at some
later restart, because the grant is checked on the server for every request. The same is
true of `app_disabled`: disabling an App, or installing a new version of it, invalidates
the bridge a running frame is holding.

## Limits

| Limit | Value |
|---|---|
| Message size, request or response | 64 KiB |
| Concurrent requests | 8 |
| Requests per minute | 120 |
| Call timeout | 10 seconds |
| Call timeout for `agents.run` | 175 seconds |
| Result of an action handler | 60 KiB |

Exceeding the action result cap rejects the handler's own promise, so the agent receives
an error rather than a truncated answer.

## `actions`

```javascript
const unregister = LaRucheApp.actions.register('cell.run', async args => {
    return { output: await runCell(args.cellId) };
});
```

`register(name, handler)` attaches a handler to one of the actions declared in
[the manifest](App-Manifest) and returns a function that detaches it. The name must be
one to eighty characters of letters, digits, dot, underscore or hyphen. The handler
receives the arguments the agent sent, already checked against the action's
`inputSchema`, and returns the result as a plain object.

An action the manifest declares but no handler registers fails when it is called. Register
every one of them before reporting ready.

## `storage`

Private key-value storage, isolated by App **and** by user. Requires `storage.private`.

| Method | Returns |
|---|---|
| `storage.get(key)` | The stored value, `null` when the key is absent |
| `storage.set(key, value)` | Nothing |
| `storage.delete(key)` | Nothing |
| `storage.list(prefix)` | The matching keys |

Values are JSON. At most 256 keys per App and user, 128 bytes per key, 64 KiB per value,
1 MiB in total, and 20 levels of nesting.

Storage is keyed by the App identifier, so give saved data its own versioned keys and
migrate it when its shape changes.

## `files`

A folder of its own on disk, which the App can read and write. Requires `laruche.files`.

| Method | Returns |
|---|---|
| `files.read(path)` | The file's content |
| `files.write(path, content, append)` | The path and the number of bytes written |
| `files.append(path, content)` | The same as `write` with `append` set |
| `files.delete(path)` | Whether something was actually deleted |
| `files.exists(path)` | A boolean |
| `files.list(path)` | The entries at that path |
| `files.mkdir(path)` | The path |

Paths are relative and stay inside the App's folder. An absolute path, a parent hop, a
drive letter or a control character is refused, and the resolved path is checked again
against the root before anything touches the disk.

At most 2 MiB per file, 32 MiB across the folder, 2 000 entries, 12 levels of nesting and
512 bytes per path.

## `memory`

Read [cognitive memory](Cognitive-Memory), and add to it. Requires `laruche.memory`.

| Method | Returns |
|---|---|
| `memory.search(query, limit)` | The same recall payload the agent receives |
| `memory.read(nodeId)` | One node and its items |
| `memory.list()` | The node tree |
| `memory.propose(nodeId, content, tags)` | `{ written: false, queued: true }` |
| `memory.write(nodeId, content, tags)` | `{ written: true, queued: false }` and the stored item |

`search` returns eight items by default, and clamps any requested limit between one and
thirty. What comes back is the recall payload unchanged, so an App sees exactly what an
agent would see for the same query.

`propose` and `write` differ only in whether the user has to agree first. Prefer
`propose`: an App writing into the user's memory without asking is the behaviour the
review queue exists to prevent. Either way the item records its origin as the App, so a
fact can be traced back to what put it there, and both are written to the activity log.

At most 32 KiB of content, 200 bytes of node identifier, 500 bytes of query and 12 tags.

## `agents`

Ask one of the agents the user authorized to do some work. Requires `agents.invoke`, and
spends model tokens.

| Method | Returns |
|---|---|
| `agents.list()` | The agents this App may invoke |
| `agents.run(agentId, sessionId, prompt)` | The agent's answer |
| `agents.act(agentId, sessionId, stateAction, prompt)` | The answer, the agent having first read the App's state through `stateAction` |
| `agents.reset(agentId, sessionId)` | Nothing, the session's history is cleared |

`sessionId` separates conversations: two views, two users or two games each keep their
own history. Contexts are held in memory, bounded, and not restored across a node
restart; the agents themselves and their permissions are on disk.

`act` is the one to reach for in a game or an editor. Passing the state action means the
agent reads the current position from the App rather than from whatever it remembers,
and `stateAction` needs its own agent permission like any other action.

These calls are slow by nature, which is why their timeout is 175 seconds rather than 10.

## `ui`

Control of the frame around the App. No permission required.

| Method | Effect |
|---|---|
| `ui.setTitle(title)` | Sets the tab title |
| `ui.setDirty(dirty)` | Marks unsaved work, so closing can warn |
| `ui.setStatus(state, message, progress)` | Reports loading state to the host |
| `ui.requestDetach()` | Asks for the view to be detached |
| `ui.close()` | Closes the view |

`setStatus` is what a view with `waitForReady` uses to report progress while a runtime
loads, and what an agent sees when it calls `app_wait`.

`requestDetach` opens a popup and may be blocked by the browser's popup rules, since it
does not come from a user gesture. The native detach control does, so prefer it.

## `call`

```javascript
await LaRucheApp.call('storage.get', { key: 'state.v1' });
```

The raw method the named helpers are built on. It exists so an App can reach a capability
that the typed helpers do not cover yet. Prefer the helpers: they are what stays stable.
