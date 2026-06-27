---
type: skill
name: youtube_video_retrieval
description: Find and download a specific YouTube video by title, channel, or keyword.
tools: [web_deep_search, web_fetch, browser_navigate, video_downloader]
---

# YouTube Video Retrieval

Locate a YouTube video URL (by title, channel, or keyword) then download it via `video_downloader`.

## Steps

### 1. Search for the URL

Use `web_deep_search` with targeted queries:
- `"<video title>" site:youtube.com`
- `"<channel name>" "<keyword>" youtube`
- `youtube "<exact title>" inurl:watch`

Parse the results for a canonical `youtube.com/watch?v=<ID>` URL.

**If no direct URL is found:**
- Try `web_fetch("https://www.youtube.com/results?search_query=<encoded+query>")` and extract `watch?v=` links from the HTML.
- Try `browser_navigate("https://www.youtube.com/results?search_query=<encoded+query>")` and scrape visible results.
- **Warning:** YouTube actively blocks scraping. If all automated paths fail, ask the user to provide the URL manually.

### 2. Validate the URL

Confirm the extracted URL matches the expected video (title, channel) before downloading. A wrong `watch?v=` ID wastes bandwidth.

### 3. Download

Once the URL is confirmed:

```
video_downloader(url="https://www.youtube.com/watch?v=<ID>")
```

`video_downloader` handles format selection, retries, and saving locally.

## Failure Handling

| Situation | Action |
|---|---|
| Search returns channel page, not video | Inspect channel page with `browser_navigate`, look for matching video title in listings |
| `web_fetch` blocked (cookie wall) | Fall back to `browser_navigate` |
| `browser_navigate` also blocked | Ask user for the URL directly |
| `video_downloader` fails (geo-block, age gate) | Report the error verbatim; suggest user retrieves via VPN or a manual tool |
| Video is private or deleted | Inform user; no workaround available |
