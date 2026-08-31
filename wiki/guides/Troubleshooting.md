# When something does not work

Symptoms first. Each one below has been seen, and the cause is rarely where the message
points.

## The agent says it cannot see an image

The picture is displayed next to the message, so it clearly arrived. What failed is the
leg between LaRuche and the provider.

Three causes, in order of likelihood:

- **The model id has no vision, even though the family does.** Providers often ship
  vision as a separate id. Check the exact id in the status bar against the provider's
  own list.
- **The provider refused the image and LaRuche stopped sending them.** A status line in
  the chat says so, with the refusal text. The model gets another chance after ten
  minutes, or immediately after a restart.
- **The image was too large for the endpoint.** Documented limits and enforced limits
  differ. LaRuche already resizes to 1280 px and about 150 KB, and shrinks further after
  a first refusal.

`LARUCHE_VISION=1` forces images through if you believe a refusal was misread.

## A tool screenshot is described wrongly, or not at all

Tool captures (`browser`, `computer`, `camera`) travel to the model as a user message
placed after the tool observation. Only the last three images of a conversation are
re-sent: a browser run takes one capture per step, and re-sending twenty of them at
every iteration costs a fortune for images the model already used. Older ones leave a
line saying they existed, so the model does not describe what it no longer has.

If the model is describing the page correctly without seeing it, that is expected: the
accessibility tree carries the text, and it is usually the better source.

## The build fails with "access denied" on the executable

The application is running and holds its own binary. Close LaRuche and build again. On
Windows an open Explorer window on `target/release` can hold it too.

## A provider profile lost its key

Fixed, but if you are on an older build: saving a profile posted an empty key field and
the empty value overwrote the stored key. Re-enter the key once. Current builds treat an
empty field as unchanged.

## A kanban task never runs

It is waiting in Triage or Todo, and that is by design. **Only the Ready column is
picked up.** Drag the task into Ready and it starts within the poll interval, five
seconds by default, adjustable in the Kanban tab. Without that gate, the dispatcher
would empty the whole board a few seconds after the agent filled it.

## Nothing happens when I click a link in the desktop application

Fixed in current builds. `target="_blank"` does nothing in a webview: there is no tab to
open and no browser around it. Links are now handed to the system browser through the
node. If you are on an older build, copy the address instead.

## The context gauge shows numbers that do not match what I am looking at

The gauge is shared between the chat and the round table because they live in the same
page. On the round table it now shows the debate's own token consumption against the
engine budget, which is a per-debate ceiling and not a context window. If the two ever
disagree again, trust the tab you are on.

## The microphone transcribes the assistant's own voice

Speech recognition opens its own microphone and cannot apply echo cancellation to what
the speakers are playing. LaRuche only opens it once the voice has really finished,
counting queued sentences and in-flight requests, not just audible sound, and after a
short silence. Interruption is detected on a separate echo-cancelled stream.

A headset removes the problem entirely and costs nothing.

## The agent stops after a few passes

Look at the terminal reason rather than the last message. A sterile loop, an exhausted
token budget and a clean answer all end the run, and they mean different things. The
pass ceiling and the budgets are in Settings, under generation.

## A model disappeared from the list

Its endpoint stopped answering. The list rescans every few seconds; the entry at the
bottom of the model menu forces a scan immediately. A local server that has just started
appears within one cycle.

## A peer hive answers with the wrong model

It did not answer at all. An unreachable peer falls back to the local provider so a
scheduled task survives a machine being asleep, and the fallback is stated in the
response. If it happens constantly, the usual cause is a hive listening only on
`127.0.0.1`: it must be started with `LARUCHE_BIND_LAN=1` to accept connections from the
network.

## Where to look next

- The **Console** panel in the dashboard carries client-side errors and the reason a
  fallback was taken.
- The **Feed** carries the structured event stream: tool calls, approvals, memory
  writes.
- A refused provider request is dumped to a temporary file whose path appears in the
  error message. That file is the exact body that was sent, which settles most
  arguments about who is at fault.
