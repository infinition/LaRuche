# Watchers

A watcher is a standing condition. It observes something, decides whether that means
anything, and reacts. The design goal is blunt: **watching must cost nothing while
nothing is happening.**

That single constraint explains every decision below. A cron wakes a model on every
tick to look at something that did not change. A watcher evaluates a compiled rule tree
instead, and never involves a model unless the tree explicitly asks for one.

## Three questions

Every watcher answers three separate questions, and they are worth keeping apart.

| Question | Field | Answers |
|---|---|---|
| What do I observe? | `watcher_type` + `target` | a file, a page, a log, a command |
| Does it mean anything? | `regles` | a rule tree, evaluated for free |
| What do I do about it? | `action` | reason, notify, or act |

You describe it in plain language:

> "If it is Tuesday or Thursday and my local site has been down for 10 minutes, send me
> a Telegram message every 20 minutes until it comes back."

The agent compiles that once, through its `watcher-design` skill, into a deterministic
predicate tree. At runtime the scheduler evaluates the tree. No model is involved.

## What you can observe

| `watcher_type` | Observes | Rules it can satisfy |
|---|---|---|
| `file` | the lifecycle AND the presence of a path. Carries NO text | `apparu`, `supprime`, `modifie`, `existe`, `absent`, `taille_depasse_mo` |
| `log` | new lines appended to a growing file | `contient`, `contenu_change`, `nouvelle_ligne` |
| `url` | reachability and page text | `est_down`, `down_depuis_min`, `retour_en_ligne`, `status_http`, `contient`, `contenu_change` |
| `command` | the output and exit code of a shell command | `contient`, `contenu_change`, `nouvelle_ligne`, `code_retour` |

The `command` type is what makes everything else reachable. A lamp, a service, a
container, free disk space: none of those is a file or a page, and all of them answer
to a CLI. The command is re-run at every poll, through PowerShell on Windows and `sh`
on macOS and Linux.

A rule that does not match its type is false at every poll. `contient` on a `file`
watcher reads an observation that carries no text, so the watcher reports itself active
and never fires. Creation refuses that combination rather than letting it ship.

## Rules

One JSON object tagged by `op`, nesting through `regles` arrays. Combinators are `et`,
`ou`, `non`.

| Rule | True when |
|---|---|
| `jour_semaine` | today is in the given set |
| `heure_entre` | local time is inside a window, end excluded |
| `plage_date` | the date is inside a range |
| `apparu` / `supprime` / `modifie` | the watched file appeared, was deleted, changed |
| `existe` / `absent` | the watched path is there right now, or is not |
| `contenu_change` | the observed content differs from last time |
| `nouvelle_ligne` | a line appeared that was NOT in the previous poll |
| `contient` | fresh content contains a pattern |
| `taille_depasse_mo` | the file passed a size |
| `est_down` / `down_depuis_min` / `retour_en_ligne` | a target is down, has been for N minutes, just came back |
| `status_http` | the last status is in the list |
| `code_retour` | the command exited with one of those codes |
| `watcher` | ANOTHER watcher currently holds a true verdict |
| `llm_check` | a model answers yes |

`llm_check` is the only leaf that costs a model call, and it runs only after the
deterministic part already passed. A watcher guarded by a weekday costs zero tokens six
days a week, by construction.

`nouvelle_ligne` deserves a note. `contenu_change` fires when the output differs, which
includes something LEAVING. For anything list shaped, a device list, running containers,
changed files, what you want is the arrival. The first poll only records a baseline and
announces nothing, otherwise everything already present is reported as new the moment
you create the watcher.

The down and up leaves carry memory between polls: `down_depuis_min` tracks how long a
target has been failing, `retour_en_ligne` fires exactly once on recovery. With a
cooldown, "nag me every 20 minutes while it is down, tell me once when it is back" is a
single watcher.

## What it does when it fires

Three behaviours, and the default is the expensive one. Choose deliberately.

| `action` | Cost | Use when |
|---|---|---|
| `{"type":"agent"}` (default) | a full model turn | the answer has to be worked out |
| `{"type":"notifier"}` | nothing | the job is just to tell you. The observation IS the message |
| `{"type":"commande","commande":"..."}` | nothing | the watcher must ACT |
| `{"type":"aucune"}` | nothing | it is a pure sensor, feeding a correlation |

`notifier` is right far more often than the default. "Tell me when the file is gone"
needs no thinking: the sentence is known in advance, and a model asked to write it can
also get it wrong.

`commande` is what turns monitoring into automation.

### Transitions or states

`apparu`, `supprime`, `modifie` are TRANSITIONS: true only on the poll where the thing
happened, false again immediately after. `existe` and `absent` are STATES: true for as
long as the situation lasts.

The distinction decides what you can combine. "test.txt is present AND the light is on"
needs a state on the file side, because a transition is false again at the very next
poll and the two conditions would essentially never be true together.

`absent` is also the one that catches what did NOT happen. Combined with `heure_entre`,
"no backup file this morning" becomes a rule, where no transition can express it:
nothing happened, so there was nothing to observe.

## Correlation

A watcher publishes its verdict, so another one can read it. This is the difference
between alerting and diagnosing: one signal rarely means anything, two together do.

```json
{"op":"et","regles":[
  {"op":"watcher","nom":"site-down","depuis_min":5},
  {"op":"watcher","nom":"hote-repond"}]}
```

The site has been down for five minutes and the host still answers ping. That is the
application, not the network. Alerting on the first leaf alone wakes someone for a cut
cable.

Verdicts are read from a snapshot taken before each round, never live, so the result
never depends on the order watchers happen to be polled in. The cost is one tick of
latency on a correlation, which beats an alert that fires differently between two runs.

A watcher referenced but deleted reads as false, never as an error. A correlation whose
peer is gone degrades quietly instead of breaking an otherwise valid tree forever.

Give the upstream watchers `{"type":"aucune"}`. They exist to publish a verdict, and
without it each of them also alerts on its own: "the site is down" and "the host
answers" would both wake you alongside the single conclusion you actually asked for.

The Watchers page draws the dependency graph whenever at least one correlation exists:
sources on the left, conclusions on the right, a green border on a verdict currently
true.

## Use cases

**A file lands on the desktop**

```json
{"watcher_type":"file","target":"C:\\Users\\me\\Desktop\\rapport.pdf",
 "regles":{"op":"apparu"},
 "action":{"type":"notifier"}}
```

**An error in a log, but not at night**

```json
{"watcher_type":"log","target":"/var/log/app.log",
 "regles":{"op":"et","regles":[
   {"op":"contient","motif":"ERROR"},
   {"op":"heure_entre","de":"08:00","a":"23:00"}]}}
```

**Someone arrives on the home network**

```json
{"watcher_type":"command","target":"arp -a",
 "regles":{"op":"nouvelle_ligne"},
 "action":{"type":"notifier"}}
```

Fires on an arrival, stays silent on a departure. `arp -a` behaves the same on Windows,
macOS and Linux. A full sweep such as `nmap -sn 192.168.1.0/24` is more reliable,
because a sleeping device drops out of the ARP table on its own. Note that recent phones
randomise their MAC address per network, so detecting that someone arrived is easy,
identifying who is much harder.

**A light left on after midnight, turned off automatically**

```json
{"watcher_type":"command","target":"openhue get light \"Bureau\"",
 "regles":{"op":"et","regles":[
   {"op":"contient","motif":"[on]"},
   {"op":"heure_entre","de":"00:30","a":"06:00"}]},
 "action":{"type":"commande","commande":"openhue set light \"Bureau\" --off"}}
```

**Disk filling up**

```json
{"watcher_type":"command",
 "target":"df -h / | tail -1 | awk '{print $5}'",
 "regles":{"op":"contient","motif":"9"},
 "action":{"type":"notifier"}}
```

On Windows, the same idea with the command that fits:
`Get-PSDrive C | Select-Object -ExpandProperty Free`.

**A container that stopped**

```json
{"watcher_type":"command","target":"docker ps --filter name=api --format '{{.Status}}'",
 "regles":{"op":"non","regle":{"op":"contient","motif":"Up"}},
 "action":{"type":"commande","commande":"docker start api"}}
```

**A light on AND a file present**

Two watchers. The sensor publishes, the alert correlates.

```json
{"name":"lumiere-bureau","watcher_type":"command",
 "target":"openhue get light \"Bureau\"",
 "regles":{"op":"contient","motif":"[on]"},
 "action":{"type":"aucune"}}
```

```json
{"name":"alerte-bureau","watcher_type":"file",
 "target":"C:\Users\me\Desktop\test.txt",
 "regles":{"op":"et","regles":[
   {"op":"existe"},
   {"op":"watcher","nom":"lumiere-bureau"}]},
 "action":{"type":"notifier"}}
```

Note `existe` rather than `apparu`: the file has to be a standing condition, not an
event, or the two conditions would essentially never be true on the same poll.

**Diagnosing rather than alerting**

Three watchers. `site-down` on the URL, `hote-repond` on a ping command, and a third
correlating both, which is the only one that notifies you. You get "the application is
broken" instead of two contradictory alarms at three in the morning.

**Something that did NOT happen**

The most useful watcher in operations is often the one that detects an absence: the
nightly backup that wrote no file, the sensor that stopped reporting. Watch the target
folder and combine `heure_entre` on the morning after with the absence of a fresh file.

## Guardrails

Learned the hard way, enforced server side.

- **Targets are validated.** A file target must be an absolute path, a URL must be well
  formed. A relative path resolves against the server working directory, not yours, so
  it would watch nothing forever while reporting itself active.
- **Duplicates are refused.** Same type, same target, still active: rejected, so a
  confused model cannot pile up six copies.
- **Destructive commands are refused.** A watcher runs unattended, every minute,
  forever. What is merely risky by hand is a standing hazard here, and the same refusal
  list covers observed commands and action commands.
- **Correlation cycles are refused at creation**, with the path that closes the loop.
  A diamond is legitimate and stays allowed: two watchers can share a source.
- **Intervals have a floor**, so a typo cannot hammer your own services.
- **A failing target backs off.** The interval doubles per consecutive failure, capped
  at one hour, and a single success clears it. A site unreachable for three days used to
  be probed every minute forever.

## Cost

Polling is parallel, so one slow URL no longer holds every other watcher for the length
of its timeout. A rule tree with no `llm_check`, and an action of `notifier` or
`commande`, costs zero tokens whatever its complexity and however often it fires.

## The UI

Each watcher card renders the compiled rule tree as a pipeline: condition bubbles,
and/or branches, the action at the end. Collapsed, a card is a one line summary.
Expanded, every value is editable in place. What you see is exactly what runs.
