# Security

LaRuche's security model starts from an honest premise: the model will eventually do
something wrong. A confused local model, a prompt injection in a fetched page, a
hallucinated command. The defenses are structural, not prompt-based.

## Network posture

- **Loopback by default.** The node binds `127.0.0.1`; other machines cannot reach it
  unless you set `LARUCHE_BIND_LAN` explicitly.
- **Strict CORS** on the API.
- **Auth guard** on every mutating route.
- **Mesh is opt-in.** The Miel protocol announces nothing until you enable it, and
  federation carries provenance tags.
- **HTTPS available** (`LARUCHE_HTTPS` self-signed, or bring your own certificate) for
  LAN access, required by browsers for the microphone.

## Execution posture

- **Approval gates.** Shell, Python, destructive file operations, and other sensitive
  tools require a human click in a popup before running. The permission engine
  (`laruche-permissions`) decides what is gated.
- **Server-side validation.** Tool arguments are validated against JSON Schemas before
  execution; watcher targets are validated for existence and well-formedness on the
  server, not trusted from the model.
- **Timeouts and budgets** bound what any single run can consume.
- **Computer control stays visible.** A halo marks automated actions, held inputs are
  released after a timeout, and `Ctrl+Alt+Shift+H` stops control immediately.
- **Browser consent stays human.** The extension reports consent overlays but does not
  accept them. Cookie names and sizes can be inspected; values are not returned.

## How the gate actually decides

The order in `approbation.rs` is fixed, and worth knowing because a decision made early
is never revisited:

1. **A user deny rule blocks, always.** This is the hard floor. Autonomous mode does not
   lift it, and the reason travels back to the model so it changes approach instead of
   rephrasing the same call.
2. **An already approved pattern runs.** That is what approving a pattern means.
3. **The security reviewer rules**, when a verdict is available. It can allow or refuse.
4. **Otherwise a human is asked**, whenever one is reachable. The popup refuses by
   default after 180 seconds.

`pre_tool` hooks can block before any of this. See [Configuration](Configuration).

## Limits

These are the places where the model above stops holding. They are written down because
you need them to decide what to turn on, not because they are theoretical.

- **`Auto` mode allows everything that is not explicitly denied.** Deny rules and the
  approval popup are what the other modes rely on; `Auto` keeps only the deny rules.
- **In an autonomous context, an unresolved call runs by default.** When no human is
  reachable (cron, watcher, scout) and the reviewer returned no verdict, LaRuche allows
  the call rather than stalling the run. Set `approbation_stricte` to reverse it and fail
  closed instead.
- **The blocked command list is a substring match, not a boundary.** Eleven patterns are
  refused outright, `rm -rf /` and `mkfs` among them. They are matched with a lowercase
  `contains`, so an equivalent command written differently goes straight through:
  `Remove-Item -Recurse -Force C:\` matches none of them. The real barrier is the
  reviewer plus the popup. Treat the list as a guard against the obvious.
- **Deferred execution is still execution.** `plugin_create` writes a manifest whose
  `command` field is a shell template run later, and `mcp_add` registers a
  `{command, args}` launched at the next start. Neither runs anything at the moment of
  the call, which makes them look like file writes. Judge them like `shell_exec`.
- **Browser control inherits your logged-in sessions.** That is the point of driving your
  own Chrome, and it is also the widest surface: `eval` runs arbitrary JavaScript in the
  page, and keystrokes go through the DevTools Input domain, so what reaches the site is
  indistinguishable from a human typing. See [Computer and Browser](Computer-and-Browser).
- **The sandbox is opt-in.** Tools run in the node's own process with your user's rights.
  `ESSAIM_SANDBOX_DOCKER=1` puts shell execution in a container when Docker is present;
  without it, execution is bare on the host.

## Data posture

- **Secrets never enter the context.** Names in, values substituted at execution time,
  and exact values masked to `[SECRET:NAME]` in every tool output before it returns to
  the loop. See [Secrets](Secrets).
- **Sanitized rendering.** Chat output is sanitized with a vendored DOMPurify before it
  touches the DOM, so a malicious page quoted by the agent cannot script your dashboard.
- **No CDN.** All web assets ship inside the binary; the UI makes no third-party
  requests.
- **Local by default.** With a local model and local speech backends, conversation
  content stays on the machine. Cloud model, search, speech and messaging providers
  receive only the requests sent to those services, and keys stay in the vault.

## Self-modification posture

The agent can improve itself, under review: skill proposals and memory changes can
route through the [LaReine](LaReine) proposal queue, pull-request style, and the
memory's git time travel ([Cognitive Memory](Cognitive-Memory)) makes any accepted
change auditable and reversible after the fact.

## Reporting

Found something? Please open a private security advisory on GitHub rather than a
public issue.
