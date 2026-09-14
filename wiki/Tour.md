# A tour of LaRuche

The rest of this wiki explains how LaRuche works. This page shows what it does. Every
recording and screenshot below comes from a running hive. The model named in the corner
of a capture is whichever one that hive was configured with at the time; nothing here
depends on it.

## Three things a chat window does not do

**It acts.** On the machine, and inside the browser you already use. Native desktop
control, and an extension that drives your own Chrome with the sessions already signed
in, rather than a blank browser standing next to it.

**It continues.** After the conversation ends. A memory you can open, reread and correct.
Scheduled tasks, long-running missions, and watchers that wake it when something moves.

**It gets supervised.** [LaReine](LaReine) reads the work and can send it back. The
[Table Ronde](Table-Ronde) makes specialists argue, which is not a vote: agreement
between models measures how alike they are, not whether they are right.

## Finding a bug in a project

![LaRuche inspects the project, recalls a similar bug it has seen before, finds the typo (openHvieModal instead of openHiveModal) and edits the file.](media/coding-bug.mp4)

Two things are worth watching here. The recall arrives before the search: the hive
remembers meeting this mistake in another folder, and says so. And the fix is an edit to
a real file on disk, not a code block to copy.

## Taking over the browser

![Navigating Wikipedia, reading the article and scrolling, driven inside the Chrome session that was already open.](media/demo-browser.mp4)

The page is not fetched and parsed somewhere else. It is the browser the user is looking
at, with its cookies, its logins and its tabs. See
[Computer and Browser](Computer-and-Browser) for what that reaches and what it does not.

## Memory that answers in your place

![The decisions/projet_alpha node, holding the fact before anyone asks for it.](media/memory-alpha-fact.webp)

![Asked why the alpha project slipped, LaRuche finds the cause in cognitive memory, roadworks in the datacenter, and answers from it.](media/memory-alpha.mp4)

The fact was stored during an earlier conversation. Nothing was searched for on the web,
and nothing was re-read: the answer comes out of the graph.
[Cognitive Memory](Cognitive-Memory) explains recall, decay and how a fact is superseded
rather than overwritten.

![A proposed fact, waiting for approval before it enters memory.](media/memory-identite.webp)
![One node of the graph, built up across conversations.](media/memory-skills.webp)

What the hive learns is visible and reversible. A proposal waits, a stored item can be
edited, moved or deleted, and the whole memory is under git so a change can be read as a
diff and rolled back.

## The supervisor reads the answer before it leaves

![LaRuche researches an acquisition, writes its synthesis, and LaReine reviews then approves the answer before it is sent.](media/lareine.mp4)

LaReine judges against a charter, can force a redo, and routes the hive's own
self-modifications through a proposal queue.

## A Table Ronde in session

![Orchestrator, scientist, systems engineer, attacker, contradictor and arbiter, each with its role, deliberating on one question.](media/table-ronde.webp)

## What it is doing, and what it is keeping

![The activity feed on the right, memory proposals on the left.](media/feed.webp)

## Reflexes, tasks and long goals

![A watcher on a file, with a Telegram alert.](media/watchers.webp)
![The board, from To do to Done.](media/kanban.webp)
![Scheduled research missions.](media/missions-recherches.webp)

A watcher costs nothing until its compiled rule is true, so watching a file all day is
not a model call all day. [Watchers](Watchers) covers the rule language, and
[Automation](Automation) covers crons, missions and the board.

## The dashboard

![Live CPU, RAM, GPU and VRAM load, the models available across the mesh, and the hive's audit log.](media/tableau-de-bord.webp)

## Under the surface

**The loop, and what keeps it from drifting.** Butinage assembles the context, picks
tools, executes, observes and starts again. Around it: budgets, anti-loop counters,
compaction and resumable runs. See [the Butinage Engine](Butinage-Engine).

**Rust, and your data where you left it.** A system program that performs actions and
runs for a long time. Memory, sessions and configuration stay on the machine, which
listens only on the loopback interface until you decide otherwise.

**The model is not the product.** llama.cpp, Ollama, LM Studio, or a remote API. Changing
one rewrites nothing, and a different model can serve a different channel. See
[Providers and profiles](Providers-and-Profiles).

Acting on a machine is what makes the guard rails necessary rather than decorative. The
defaults, the approval policy and the limits of that gate are set out in
[Security](Security).

## Where to go next

- [Quick Start](Quick-Start) for a first boot and a first conversation
- [Architecture](Architecture) for the crates and how a request flows through them
- [FAQ](FAQ) for what is ready today and what is still experimental
