# Providers and profiles

Most confusion about "which model am I actually talking to" comes from one idea that is
never stated: **the active model is a pair, not a name**. A model name alone means
nothing, because the same name can be served by your machine, by a hosted API and by
another hive on your network, with different keys and different limits.

That pair is a **profile** plus a **model**.

## What a profile is

A profile is one endpoint you can reach, described once. It lives in
`provider-profiles.json` next to the rest of your configuration, and it holds:

| Field | Meaning |
|---|---|
| `provider` | The dialect to speak: `openai`, `anthropic`, `ollama`, `codex`, `miel` |
| `base_url` | Where to send the request |
| `api_key` | The key, or a vault reference like `${DEEPSEEK}` |
| `models` | The model names this endpoint serves |
| `max_context_length` | The window, used for the context gauge |
| `visibility` | Whether other hives may use this profile through the mesh |

Two profiles can serve a model with the same name. That is not a conflict, it is the
normal case: `llama3.2` on your machine and `llama3.2` on the workstation in the other
room are two different things, and the pair is what tells them apart.

The key deserves its own note. Store it as a vault reference rather than a literal, and
the value never sits in plain text in the configuration file. Substitution happens at
the moment the request is built, at one single point that every provider call goes
through. See [Secrets](Secrets).

## Where the active pair is used

The pair chosen in the status bar is the default for the chat and for anything that does
not override it. Several things override it, deliberately:

- **A kanban task** can carry its own profile and model, set when the task is created.
- **A cron or a mission** can pin a model, so a scheduled job does not silently change
  behaviour when you switch the model you are chatting with.
- **A specialist at the round table** can be given its own profile. This is the most
  important override in the whole system, and it has its own reason: see
  [Table-Ronde](Table-Ronde).
- **Auxiliary runs** (the curator, the scouts) use the lighter model configured for
  them, not the main one.

## A hive as a provider

Every LaRuche node exposes an OpenAI-compatible endpoint, so another hive on the network
is reachable exactly like any hosted API. A profile of provider type `miel`, or a model
addressed as `peer:<host>:<port>`, points at that hive.

One behaviour is worth knowing before you rely on it. If the peer does not answer, the
call falls back to the local provider rather than failing: a hive is a machine that can
be asleep, unplugged or rebooting, and a scheduled task must not die because the box
next door went off. **The fallback is announced in the response.** Silently substituting
a model is worse than a clean failure, because the answer then gets attributed to the
wrong model.

## Vision is a property of the endpoint, not of the model

A model's datasheet can say it accepts images while the endpoint you actually call
refuses them, or accepts them only on a different model id. Observed on a provider that
documents 32 MiB per image and rejects a 200 KB screenshot on its non-vision id.

So LaRuche does not guess from the name. Images are sent, and if the provider refuses,
the refusal is read: an explicit "this model does not support image" drops images for
that model, while a vaguer complaint about `image_url` first retries with a smaller
image, because an undocumented size limit explains the same error. The model gets its
chance back after ten minutes rather than being written off for the session. Set
`LARUCHE_VISION=0` to stop sending images entirely, or `=1` to force them.

Images are resized before sending, to 1280 px on the long side and about 150 KB. Very
little is lost by this: vision models tile an image into patches of a few hundred pixels
and never see the full resolution, and some providers bill a flat token count per image
whatever its size. The number 150 KB is not derived from any published limit, because
published limits turned out not to predict what endpoints accept. It sits below the
largest image observed to go through.

## Practical notes

- **An empty key field means unchanged.** The API never returns a key in clear, so the
  Settings form cannot show it back and posts it empty when you have not touched it.
  Saving a profile to add a model therefore keeps the key it already had.
- **A model that vanishes from the list** usually means its endpoint stopped answering.
  The status bar rescans every few seconds, and the entry at the bottom of the model
  menu forces a scan immediately.
- **Changing the active pair is live.** No restart, and the running engine follows.
