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
