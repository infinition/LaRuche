---
type: skill
name: songwriting-and-ai-music
title: Songwriting & AI Music Generation
description: "Write lyrics, craft Suno prompts, engineer AI vocal delivery."
version: "1.1.0"
license: MIT
tags: [songwriting, music, suno, parody, lyrics, creative]
platforms: [linux, macos, windows]
tools: [file_write, file_read, memory_write]
dependencies: []
metadata:
  laruche:
    category: creative
    homepage: https://suno.com
triggers:
  - writing a song
  - song lyrics
  - music prompt
  - suno prompt
  - parody song
  - adapting a song
  - AI music generation
---

# Songwriting & AI Music Generation

Guidelines, not rules - art breaks rules on purpose. Use what serves the song.

---

## 1. Song Structure

Common skeletons:
```
ABABCB  Verse/Chorus/Verse/Chorus/Bridge/Chorus  (pop/rock)
AABA    Verse/Verse/Bridge/Verse                  (jazz, ballads)
AAA     Verse/Verse/Verse, no chorus              (folk, storytelling)
```

Building blocks: **Intro · Verse · Pre-Chorus · Chorus · Bridge · Outro**
Structure serves the emotion - you don't need all of them.

---

## 2. Rhyme, Meter, and Sound

**Rhyme types** (tight → loose): Perfect · Family · Assonance (same vowels) · Consonance (similar endings) · Slant
Mix them - all-perfect sounds like a nursery rhyme; all-slant sounds lazy. Internal rhyme (within a line) adds density.

**Meter:**
- Stressed syllables matter more than total count
- Match syllable counts between parallel lines for singability
- Say it aloud - stumbles signal meter problems
- Breaking meter intentionally creates emphasis

---

## 3. Emotional Arc and Dynamics

Energy map (rough guide):
```
Intro: 2-3 | Verse: 5-6 | Pre-Chorus: 7 | Chorus: 8-9 | Bridge: varies | Final Chorus: 9-10
```

**Contrast is the most powerful dynamic tool:**
- Whisper before a scream hits harder than just screaming
- Sparse before dense; silence is an instrument
- "Whisper → roar → whisper" works for ballads, epics, anthems

---

## 4. Lyric Craft

**Show, don't tell** (usually):
- "I was sad" = flat → "Your hoodie's still on the hook by the door" = alive
- But blunt declaration can BE the power - context decides

**The hook:** the line people remember. Melody + lyric + emotion must align. Place it where it lands hardest (often first/last line of chorus).

**Prosody (lyrics ↔ music must reinforce each other):**
- Stable feelings → settled melodies, perfect rhymes, resolved chords
- Unstable feelings → wandering melodies, near-rhymes, unresolved chords
- Verse melody typically sits lower; chorus goes higher

**Avoid (unless intentional):**
- Clichés on autopilot ("heart of gold" without earning it)
- Forcing word order to hit a rhyme ("Yoda-speak")
- Flat energy across all sections
- Treating first draft as sacred - revision is creation

---

## 5. Parody and Adaptation

**Map the original first:**
- Count syllables per line; mark rhyme scheme (ABAB, AABB, etc.)
- Identify stressed syllables and where held/sustained notes fall

**Fitting new words:**
- Match stressed syllables to the same beats as the original
- Total syllable count can flex ±1-2 unstressed syllables
- On held notes, match the vowel sound: "LOOOVE" → "FOOOD" fits; "LIFE" doesn't
- Monosyllabic swaps in key spots keep rhythm intact (Crime → Code, Snake → Noose)
- Sing new words over the original - stumbles signal revisions needed

**Concept and structure:**
- Start from the title/hook, build outward; generate raw material first (puns, images, phrases), then fit the best into structure
- Reverse-engineer the rhyme scheme backward to set up a specific line
- Leave a few original lines or structures intact - adds recognizability, lets the audience feel the ghost of the original

---

## 6. Suno AI Prompt Engineering

### Style/Genre Description Field

**Formula:** Genre + Mood + Era + Instruments + Vocal Style + Production + Dynamic Arc

```
BAD:  "sad rock song"
GOOD: "Cinematic orchestral spy thriller, 1960s Cold War era, smoky sultry
       female vocalist, big band jazz, brass with trumpets and french horns,
       sweeping strings, minor key, vintage analog warmth. Begins as a
       haunting whisper over sparse piano, layers in muted brass, builds to
       full orchestra in the chorus, then strips back to lone piano fading
       to silence."
```

**Tips:**
- Suno v4.5+ supports up to 1,000 chars in the Style field - use them
- NO artist names or trademarks. Describe the sound: "90s grunge" not "Nirvana-style"
- Specify BPM and key when you have a preference
- Use the Exclude Styles field for what you DON'T want
- Unexpected genre combos can be gold: "bossa nova trap", "Appalachian gothic", "chiptune jazz"
- Build a vocal **persona**: "A weathered torch singer with a smoky alto, slight rasp, starts vulnerable, builds to devastating power"
- **Describing the dynamic arc matters more than just listing genres** - "whisper to roar to whisper" gives Suno a performance map

### Metatags (place in [brackets] inside the Lyrics field)

**Structure:**
`[Intro]` `[Verse]` `[Verse 1]` `[Pre-Chorus]` `[Chorus]` `[Post-Chorus]`
`[Hook]` `[Bridge]` `[Interlude]` `[Instrumental]` `[Breakdown]` `[Build-up]`
`[Outro]` `[Silence]` `[End]`

**Vocal performance:**
`[Whispered]` `[Spoken Word]` `[Belted]` `[Falsetto]` `[Powerful]` `[Soulful]`
`[Raspy]` `[Breathy]` `[Staccato]` `[Legato]` `[Vibrato]` `[Melismatic]`
`[Harmonies]` `[Choir]` `[Harmonized Chorus]`

**Dynamics:**
`[High Energy]` `[Low Energy]` `[Building Energy]` `[Explosive]`
`[Emotional Climax]` `[Gradual swell]` `[Orchestral swell]`
`[Quiet arrangement]` `[Falling tension]` `[Slow Down]`

**Gender:** `[Female Vocals]` `[Male Vocals]`

**Atmosphere:** `[Melancholic]` `[Euphoric]` `[Nostalgic]` `[Aggressive]` `[Dreamy]` `[Intimate]` `[Dark Atmosphere]`

**SFX:** `[Vinyl Crackle]` `[Rain]` `[Applause]` `[Static]` `[Thunder]`

**Rules:**
- Put key tags in BOTH the style field AND the lyrics field for reinforcement
- 5-8 tags per section max - too many confuses the AI
- Don't contradict yourself (`[Calm]` + `[Aggressive]` in the same section)

### Custom Mode
- Always use Custom Mode for serious work (separate Style + Lyrics fields)
- Lyrics field limit: ~3,000 chars (~40-60 lines)
- Always add structural tags - without them Suno defaults to flat verse/chorus with no emotional arc

---

## 7. Phonetic Tricks for AI Singers

AI vocalists pronounce, not read. Help them:

**Phonetic respelling:**
- Spell words as they sound: "through" → "thru"
- Proper nouns have the highest failure rate - test early with a 30-second clip
- "Nous" → "Noose" (forces correct pronunciation)
- Hyphenate to guide syllables: "Re-search", "bio-engineering"

**Delivery control:**
- ALL CAPS = louder/more intense
- Vowel extension: "lo-o-o-ove" = sustained/melisma
- Ellipses: "I... need... you" = dramatic pauses
- Hyphenated stretch: "ne-e-ed" = emotional stretch

**Always:**
- Spell out numbers: "24/7" → "twenty four seven"
- Space acronyms: "AI" → "A I" or "A-I"
- Pronunciation is baked in once generated - fix lyrics BEFORE generating

---

## 8. Workflow

1. Nail the concept/hook first - what is the emotional core?
2. If adapting, map the original (syllables, rhyme scheme, stressed syllables, held notes)
3. Brainstorm raw material freely (puns, images, phrases) before structuring
4. Draft lyrics into the chosen structure; use `file_write` to save working drafts
5. Read/sing aloud - catch stumbles, fix meter
6. Build the Suno style description - paint the dynamic journey, not just the genre
7. Add metatags to lyrics for performance direction
8. Generate 3-5 variations minimum - treat them like recording takes
9. Extend/Continue from the best variation; restate genre/mood in extensions (style can drift)
10. If something great happens by accident, keep it
11. Store successful prompt patterns with `memory_write` for reuse

Expect ~3-5 generations per 1 good result. Revision is normal.
