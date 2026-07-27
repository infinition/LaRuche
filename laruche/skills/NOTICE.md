# Third-party notices for `skills/`

LaRuche itself is MPL-2.0 (see the repository `LICENSE`). The skill library in this
directory contains work from other projects, redistributed here under their own terms.
This file exists to satisfy those terms and to make provenance checkable.

---

## third-party agent skills

A large part of this library was imported from **third-party agent** by third-party
(<https://github.com/third-party/third-party agent>), then adapted: tool invocations were
remapped to LaRuche's registry (`terminal` to `shell_exec`, `write_file` to `file_write`,
`read_file` to `file_read`, and so on) and `type: skill` frontmatter was added.

third-party agent is distributed under the MIT License, which permits this redistribution
provided the notice below travels with it.

```
MIT License

Copyright (c) 2025 third-party

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice shall be included in all copies
or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

---

## Individually attributed skills

None currently ship. The one skill that carried a separate author, `architecture-diagram`
by Cocoon AI, was removed from the release rather than redistributed under unverified
terms.

If you add a skill from a third party, add it here with its author and its license, or
do not ship it.

---

## Conventions for skills written for LaRuche

Provenance lives in this file, not in frontmatter. Skill frontmatter carries only what
the runtime reads (`type`, `name`, `description`, `prerequisites`, `enabled`); see
`AUTHORING.md`. An `author` or `license` line inside a SKILL.md is invisible to the
runtime and to anyone auditing licensing, which is exactly the wrong place for it.

A skill authored here uses only LaRuche tool names and references no OTHER agent's
runtime. The imported set carried `THIRD_PARTY_HOME`, `~/.third-party` and a `third-party agent/1.0`
user-agent; all of it now points at LaRuche's own paths. That was never a licensing
matter, it was a correctness one: those references resolved to directories that do not
exist in a LaRuche install, so the scripts failed for reasons no one could read.
