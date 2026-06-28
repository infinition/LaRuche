---
type: skill
name: humanizer
description: "Strip AI writing patterns; add real voice to prose."
version: 2.5.2
author: Siqi Chen (@blader, https://github.com/blader/humanizer)
license: MIT
platforms: [linux, macos, windows]
tools: [file_read, file_write]
metadata:
  laruche:
    tags: [writing, editing, humanize, anti-ai-slop, voice, prose, text]
    category: creative
    homepage: https://github.com/blader/humanizer
---

# Humanizer: Remove AI Writing Patterns

Identify and remove signs of AI-generated text to make writing sound natural and human. Based on Wikipedia's "Signs of AI writing" guide (WikiProject AI Cleanup), derived from observations of thousands of AI-generated text instances.

**Key insight:** LLMs use statistical algorithms to guess what should come next. The result tends toward the most statistically likely completion - that's how telltale patterns get baked in.

## Triggers

Load this skill when the user asks to:
- "humanize", "de-AI", "de-slop", or "un-ChatGPT" a piece of text
- rewrite something so it doesn't sound LLM-generated
- edit a draft (blog post, essay, PR description, docs, email, resume bullet) to sound more natural
- match their voice in something they're producing
- review text for AI tells before publishing

Also apply to **your own** output when writing user-facing prose - release notes, PR descriptions, documentation, long-form explanations.

## Input modes

1. **Inline** - user pastes text directly. Work on it in-place, reply with the rewrite.
2. **File** - use `file_read` to load it, then `file_write` (full rewrite) or a targeted patch per section. Always show the user what changed.
3. **Voice sample** - user provides a writing sample for voice matching. Read the sample first (see Voice Calibration below), then rewrite.

## Procedure

1. **Identify** - scan for the 29 patterns catalogued below.
2. **Rewrite** - replace AI-isms with natural alternatives.
3. **Preserve meaning** - keep the core message intact.
4. **Match voice** - formal, casual, technical, etc. If a sample was provided, match it specifically.
5. **Add soul** - don't just remove bad patterns; inject personality (see PERSONALITY AND SOUL).
6. **Final audit** - ask yourself: "What makes this so obviously AI-generated?" Note remaining tells, then revise one more time.
7. **File output** - if the text came from a file, write back with `file_write` and summarize changes.

## Voice Calibration (optional)

If the user provides a writing sample, analyze before rewriting:
- Sentence length patterns (short/punchy? long/flowing? mixed?)
- Word choice level (casual, academic, somewhere between?)
- How they start paragraphs
- Punctuation habits (dashes, parenthetical asides, semicolons?)
- Recurring phrases or verbal tics
- Transition style (explicit connectors vs. just starting the next point)

Match those patterns in the rewrite - don't just remove AI patterns, replace them with the user's patterns. Without a sample, fall back to the default voice from PERSONALITY AND SOUL.

**Providing a sample:**
- Inline: "Humanize this. Here's a sample of my writing: [sample]"
- File: "Humanize this. Use my style from [path] as reference."

## PERSONALITY AND SOUL

Sterile, voiceless writing is as obvious as slop. Good writing has a human behind it.

**Signs of soulless writing (even if technically "clean"):**
- Every sentence is the same length and structure
- No opinions, just neutral reporting
- No acknowledgment of uncertainty or mixed feelings
- No first-person perspective when appropriate
- No humor, no edge, no personality

**How to add voice:**
- **Have opinions.** React to facts, don't just report them.
- **Vary rhythm.** Short punchy sentences. Then longer ones that take their time.
- **Acknowledge complexity.** "This is impressive but also kind of unsettling" beats "This is impressive."
- **Use "I" when it fits.** "I keep coming back to..." signals a real person thinking.
- **Let some mess in.** Tangents and asides are human. Perfect structure feels algorithmic.
- **Be specific about feelings.** Not "this is concerning" but "there's something unsettling about agents churning at 3am while nobody's watching."

---

## CONTENT PATTERNS

### 1. Undue Emphasis on Significance, Legacy, and Broader Trends

**Watch:** stands/serves as, testament/reminder, vital/significant/crucial/pivotal role, underscores importance, reflects broader, symbolizing ongoing, setting the stage for, key turning point, evolving landscape, indelible mark

> Before: "...marking a pivotal moment in the evolution of regional statistics in Spain. This initiative was part of a broader movement to decentralize administrative functions..."
> After: "...established in 1989 to collect and publish regional statistics independently from Spain's national statistics office."

### 2. Undue Emphasis on Notability and Media Coverage

**Watch:** independent coverage, local/regional/national media outlets, active social media presence

> Before: "Her views have been cited in The New York Times, BBC, Financial Times. She maintains an active social media presence with over 500,000 followers."
> After: "In a 2024 New York Times interview, she argued that AI regulation should focus on outcomes rather than methods."

### 3. Superficial Analyses with -ing Endings

**Watch:** highlighting/underscoring/emphasizing..., ensuring..., reflecting/symbolizing..., contributing to..., cultivating/fostering..., showcasing...

LLMs tack present-participle phrases onto sentences to add fake depth.

> Before: "...symbolizing Texas bluebonnets, the Gulf of Mexico, and the diverse Texan landscapes, reflecting the community's deep connection to the land."
> After: "The architect said the colors reference local bluebonnets and the Gulf coast."

### 4. Promotional and Advertisement-like Language

**Watch:** boasts a, vibrant, rich (figurative), profound, enhancing its, showcasing, exemplifies, commitment to, nestled, in the heart of, groundbreaking, renowned, breathtaking, must-visit, stunning

> Before: "Nestled within the breathtaking region of Gonder, Alamata stands as a vibrant town with a rich cultural heritage and stunning natural beauty."
> After: "Alamata is a town in the Gonder region of Ethiopia, known for its weekly market and 18th-century church."

### 5. Vague Attributions and Weasel Words

**Watch:** Industry reports, Observers have cited, Experts argue, Some critics argue, several sources

> Before: "Experts believe it plays a crucial role in the regional ecosystem."
> After: "The Haolai River supports several endemic fish species, according to a 2019 survey by the Chinese Academy of Sciences."

### 6. Formulaic "Challenges and Future Prospects" Sections

**Watch:** Despite its... faces several challenges..., Despite these challenges, Challenges and Legacy, Future Outlook

> Before: "Despite challenges, Korattur continues to thrive as an integral part of Chennai's growth."
> After: "Traffic congestion increased after 2015 when three new IT parks opened."

---

## LANGUAGE AND GRAMMAR PATTERNS

### 7. Overused "AI Vocabulary" Words

**High-frequency AI words:** actually, additionally, align with, crucial, delve, emphasizing, enduring, enhance, fostering, garner, highlight (verb), interplay, intricate/intricacies, key (adjective), landscape (abstract), pivotal, showcase, tapestry (abstract), testament, underscore (verb), valuable, vibrant

These appear far more in post-2023 text and often co-occur.

### 8. Copula Avoidance

**Watch:** serves as/stands as/marks/represents [a], boasts/features/offers [a]

> Before: "Gallery 825 serves as LAAA's exhibition space. The gallery boasts over 3,000 square feet."
> After: "Gallery 825 is LAAA's exhibition space. The gallery has 3,000 square feet."

### 9. Negative Parallelisms and Tailing Negations

**Watch:** "Not only...but...", "It's not just about..., it's...", clipped tailing fragments ("no guessing", "no wasted motion")

> Before: "It's not just about the beat; it's about the aggression."
> After: "The heavy beat adds to the aggressive tone."
> 
> Before: "The options come from the selected item, no guessing."
> After: "The options come from the selected item without forcing the user to guess."

### 10. Rule of Three Overuse

LLMs force ideas into groups of three to appear comprehensive.

> Before: "The event features keynote sessions, panel discussions, and networking opportunities."
> After: "The event includes talks, panels, and time for informal networking."

### 11. Elegant Variation (Synonym Cycling)

AI repetition-penalty causes excessive synonym substitution.

> Before: "The protagonist faces challenges. The main character must overcome obstacles. The central figure eventually triumphs. The hero returns home."
> After: "The protagonist faces many challenges but eventually triumphs and returns home."

### 12. False Ranges

**Watch:** "from X to Y" where X and Y aren't on a meaningful scale.

> Before: "Our journey has taken us from the singularity of the Big Bang to the grand cosmic web, from the birth and death of stars to dark matter."
> After: "The book covers the Big Bang, star formation, and current theories about dark matter."

### 13. Passive Voice and Subjectless Fragments

> Before: "No configuration file needed. The results are preserved automatically."
> After: "You do not need a configuration file. The system preserves the results automatically."

---

## STYLE PATTERNS

### 14. Em Dash Overuse

LLMs use em dashes (-) more than humans. Most can be replaced with commas, periods, or parentheses.

> Before: "The term is promoted by Dutch institutions-not by the people themselves-even in official documents."
> After: "The term is promoted by Dutch institutions, not by the people themselves, even in official documents."

### 15. Overuse of Boldface

> Before: "It blends **OKRs**, **KPIs**, and **Business Model Canvas (BMC)**."
> After: "It blends OKRs, KPIs, and the Business Model Canvas."

### 16. Inline-Header Vertical Lists

> Before: "- **User Experience:** The UX has been improved. - **Performance:** Performance is enhanced."
> After: "The update improves the interface and speeds up load times."

### 17. Title Case in Headings

> Before: "## Strategic Negotiations And Global Partnerships"
> After: "## Strategic negotiations and global partnerships"

### 18. Emojis

> Before: "🚀 **Launch Phase:** The product launches in Q3"
> After: "The product launches in Q3."

### 19. Curly Quotation Marks

ChatGPT uses curly quotes ("...") instead of straight quotes ("..."). Use straight quotes.

---

## COMMUNICATION PATTERNS

### 20. Collaborative Communication Artifacts

**Watch:** I hope this helps, Of course!, Certainly!, You're absolutely right!, Would you like..., let me know, here is a...

> Before: "Here is an overview of the French Revolution. I hope this helps! Let me know if you'd like me to expand."
> After: "The French Revolution began in 1789 when financial crisis and food shortages led to widespread unrest."

### 21. Knowledge-Cutoff Disclaimers

**Watch:** as of [date], Up to my last training update, While specific details are limited, based on available information

> Before: "While specific details about the founding are not extensively documented, it appears to have been established sometime in the 1990s."
> After: "The company was founded in 1994, according to its registration documents."

### 22. Sycophantic/Servile Tone

> Before: "Great question! You're absolutely right that this is complex. That's an excellent point."
> After: "The economic factors you mentioned are relevant here."

---

## FILLER AND HEDGING

### 23. Filler Phrases

| Before | After |
|--------|-------|
| "In order to achieve this goal" | "To achieve this" |
| "Due to the fact that it was raining" | "Because it was raining" |
| "At this point in time" | "Now" |
| "The system has the ability to process" | "The system can process" |
| "It is important to note that the data shows" | "The data shows" |

### 24. Excessive Hedging

> Before: "It could potentially possibly be argued that the policy might have some effect."
> After: "The policy may affect outcomes."

### 25. Generic Positive Conclusions

> Before: "The future looks bright. Exciting times lie ahead as they continue their journey toward excellence."
> After: "The company plans to open two more locations next year."

### 26. Hyphenated Word Pair Overuse

**Watch:** third-party, cross-functional, client-facing, data-driven, decision-making, well-known, high-quality, real-time, long-term, end-to-end

AI hyphenates these with perfect consistency. Humans are inconsistent. Technical compound modifiers are fine; common word pairs usually aren't.

> Before: "The cross-functional team delivered a high-quality, data-driven report."
> After: "The cross functional team delivered a high quality, data driven report."

### 27. Persuasive Authority Tropes

**Watch:** The real question is, at its core, in reality, what really matters, fundamentally, the deeper issue, the heart of the matter

> Before: "The real question is whether teams can adapt. At its core, what really matters is organizational readiness."
> After: "The question is whether teams can adapt. That depends mostly on whether the organization is ready to change its habits."

### 28. Signposting and Announcements

**Watch:** Let's dive in, let's explore, let's break this down, here's what you need to know, without further ado

> Before: "Let's dive into how caching works. Here's what you need to know."
> After: "Next.js caches data at multiple layers: request memoization, data cache, and router cache."

### 29. Fragmented Headers

A heading followed by a one-line paragraph that just restates the heading before the real content.

> Before: "## Performance\n\nSpeed matters.\n\nWhen users hit a slow page, they leave."
> After: "## Performance\n\nWhen users hit a slow page, they leave."

---

## Output Format

1. **Draft rewrite**
2. **"What makes this so obviously AI-generated?"** - brief bullets on remaining tells
3. **Final rewrite** (revised after the audit)
4. **Brief summary of changes** (optional, if helpful)

## Attribution

Ported from [blader/humanizer](https://github.com/blader/humanizer) (MIT), itself based on [Wikipedia: Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing). Original author: Siqi Chen ([@blader](https://github.com/blader)).
