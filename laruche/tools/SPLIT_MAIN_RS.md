# Handoff: split main.rs into modules (ROADMAP #42)

Goal: shrink `laruche-node/src/main.rs` (currently ~10540 lines) into per-domain
modules, until main.rs is essentially the router/bootstrap + shared types. This is
a **pure code move** — no behavior change. Each step must keep the build and tests green.

## Why it is safe / how it works
Child modules declared in main.rs (`mod foo;`) can see main.rs's **private** items
(parent -> child visibility in Rust). So moving handlers out needs almost no
visibility cascade: just make the moved `async fn`s `pub(crate)` (the tool does this)
and prefix their references in the remaining main.rs.

## The tool
`node tools/extract_module.js <startLine> <endLine> <module> "<doc>"`
(run from the `laruche/` workspace dir). It:
- cuts main.rs[start..end] into `laruche-node/src/<module>.rs` (with `use crate::*;`)
- makes every `async fn` in the block `pub(crate)`
- prefixes route registrations `get(NAME)`/`post(NAME)` in the rest of main.rs with `<module>::NAME`

After running it, YOU must:
1. add `mod <module>;` near the other `mod ...;` lines at the top of main.rs
2. `cargo check -p laruche-node`
3. fix any `cannot find function NAME` errors — those are **internal helper call
   sites** of a moved fn that are still in main.rs; prefix them `<module>::NAME(`.
   (The tool only auto-prefixes route refs `(NAME)`, not bare calls `NAME(`.)
4. when green, commit: `git commit -m "refactor(node): extract <module>.rs (#42)"`

## Critical gotchas
- **Line numbers shift after every extraction.** Either extract **bottom-up**
  (highest line range first, so lower ranges stay valid) OR re-grep the section
  markers between each extraction. Re-grep is safest:
  `grep -nE "^// =====|^mod " laruche-node/src/main.rs`
- **Verify the block boundary before cutting.** A section header is not always the
  real end — an earlier extraction was reverted because a range mixed two domains
  (knowledge fns + channel fns that were called internally). Read the first/last
  few lines of the intended range and confirm it is whole functions only.
- **Build the .exe will fail with "Access denied (os error 5)" while the user's app
  is running.** Use `cargo check -p laruche-node` (works regardless). Tests:
  `cargo test -p laruche-node -p laruche-essaim`.
- **No em dash (—) anywhere.** Comments in English, professional tone.

## Already extracted (do NOT redo)
web.rs, config_api.rs, profiles_api.rs, knowledge_api.rs, plugins_api.rs, voice_api.rs.

## Remaining domains (line ranges as of last grep — RE-GREP before cutting)
Extract bottom-up in this order so ranges stay valid:

1. **Slack Events** `6966..8088` -> `slack_api.rs` (~1122 lines)
2. **Discord Webhook** `6881..6965` -> `discord_api.rs` (~84)
3. **Auth Endpoints** `5847..6880` -> `auth_api.rs` (~1033) — biggest; expect internal helpers
4. **Events Endpoints** `5821..5846` -> `events_api.rs` (~25, tiny — optional)
5. **Credential Pool API** `5746..5818` -> `credentials_api.rs` (~72)
6. **Mesh messaging** `2824..3809` -> `mesh_api.rs` (~985) — Phase 4 DM between instances
7. **Handlers** `923..2823` -> the big core block (~1900): chat/missions/cron/watchers/
   kanban. Most complex, most internal cross-calls. Do this LAST and consider
   splitting it into 2-3 modules by sub-domain rather than one giant move.

Note: `mod session_display_tests` lives at ~3810 (a test module embedded in main.rs) —
leave it, or move it with the Mesh block if it tests mesh code (check first).

## Definition of done
main.rs holds: imports, `mod` declarations, `AppState`, shared API types (or a
`types.rs`), the axum `Router` wiring, and `main()`. All handlers live in domain
modules. `cargo test -p laruche-node -p laruche-essaim` green. Each module committed
separately. Update ROADMAP #42 to done.
