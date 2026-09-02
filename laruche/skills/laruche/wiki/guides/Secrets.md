# Secrets

The secrets vault exists for one reason: your API keys, tokens, and passwords should
never enter a model's context. Not the local model's, not a cloud model's, not the
session file's. LaRuche enforces this in both directions.

## Storing a secret

**Settings > Secrets**: give it a name (`TAVILY_KEY`, `HOMELAB_SSH_PASS`), paste the
value once. Values are encrypted at rest and never displayed again.

## Using a secret: `@@NAME`

Reference a secret by name anywhere: in chat, in a cron payload, in a watcher action,
in a tool argument. Autocomplete for `@@` works across the UI.

> "Call the staging API with the header `Authorization: Bearer @@STAGING_TOKEN` and
> tell me the status."

The model sees the literal text `@@STAGING_TOKEN` and passes it along in its tool call.
Substitution happens at execution time, inside the tool runner, after the model has
already committed its arguments. The value exists only in the actual HTTP request or
shell command.

## The way back is guarded too

Substitution alone is not enough: a tool's output can echo the secret (a debug page
printing headers, `env` in a shell, an error message quoting the request). LaRuche
masks tool outputs before they return to the loop: any exact occurrence of a stored
secret value becomes `[SECRET:NAME]`.

So even in the worst case, the model's context and the session file contain the
placeholder, never the value.

## What this enables

- Give a **local** model the ability to use your paid cloud APIs without ever holding
  the keys.
- Let the agent run authenticated shell commands while keeping credentials out of chat
  history.
- Share a screen or a session export without scrubbing it first.

## Limits worth knowing

Masking matches exact values. A tool that re-encodes a secret (base64, URL-encoding)
before printing it would slip past the filter. Treat the vault as a strong seatbelt,
not diplomatic immunity: prefer scoped, revocable tokens where the service offers them.
