---
type: skill
name: dogfood
description: "Systematic exploratory QA of web apps: browse, find bugs, report."
version: 1.0.1
platforms: [linux, macos, windows]
tools: [browser_navigate, browser_screenshot, shell_exec, file_write]
metadata:
  laruche:
    tags: [qa, testing, browser, web, dogfood]
    related_skills: []
---

# Dogfood: Systematic Web Application QA Testing

## Inputs

1. **Target URL** - entry point for testing
2. **Scope** - areas/features to focus on, or "full site"
3. **Output directory** (optional) - default: `./dogfood-output`

## Prerequisites

LaRuche browser tools: `browser_navigate`, `browser_screenshot`.
Create the output directory before starting:

```
shell_exec("mkdir -p ./dogfood-output/screenshots")
```

## Workflow

### Phase 1: Plan

Based on scope, list pages/features to test:
- Home, nav links (header, footer, sidebar)
- Key user flows (sign-up, login, search, checkout)
- Forms and interactive elements
- Edge cases (empty states, 404s, invalid inputs)

### Phase 2: Explore

Repeat for each page/feature:

1. **Navigate:**
   ```
   browser_navigate(url="https://example.com/page")
   ```

2. **Screenshot + visual inspection:**
   ```
   browser_screenshot()
   ```
   Describe layout, broken elements, visual issues, accessibility concerns.

3. **Check visible console output / network errors** via the screenshot or page source if accessible. Note any JS errors, 4xx/5xx responses visible in the UI.

4. **Test interactions** by navigating to sub-pages, submitting forms (empty, invalid, oversized, special chars `<`, `"`, `'`), and following links.

5. **After each significant interaction**, take another screenshot and note state changes.

6. **Scroll pages**: take screenshots at different scroll positions - lazy-load failures appear below the fold.

### Phase 3: Collect Evidence

For every issue found:

1. Capture screenshot and note path:
   ```
   browser_screenshot()
   ```

2. Log the following per issue:
   - URL where found
   - Steps to reproduce
   - Expected vs actual behavior
   - Console errors (if visible)
   - Screenshot path

3. Classify using `references/issue-taxonomy.md`:
   - **Severity**: Critical / High / Medium / Low
   - **Category**: Functional / Visual / Accessibility / Console / UX / Content

### Phase 4: Categorize

1. De-duplicate - same bug on multiple pages = one issue.
2. Assign final severity and category.
3. Sort: Critical → High → Medium → Low.
4. Count by severity and category for the executive summary.

### Phase 5: Report

Generate the report from `templates/dogfood-report-template.md`. Must include:

1. **Executive summary** - total count, breakdown by severity, testing scope
2. **Per-issue sections** - number, title, severity/category, URL, description, steps to reproduce, expected vs actual, screenshot (`MEDIA:<screenshot_path>`), console errors
3. **Summary table** - all issues in one view
4. **Testing coverage** - what was tested, what was not, blockers

Save with `file_write`:
```
file_write(path="./dogfood-output/report.md", content=<rendered report>)
```

## Pitfalls

- **Check console output after every navigation** - uncaught JS exceptions are invisible to the eye but critical findings. Use any visible error banners, network tab info, or page-embedded error messages.
- **Test form edge cases**: empty submit, very long strings, special chars (`<`, `"`, `'`), rapid double-clicks.
- **Scroll all pages** - content below the fold may have rendering or lazy-load failures.
- **Display screenshots inline** with `MEDIA:<screenshot_path>` for immediate user visibility.
- **Scope creep**: stay within the agreed scope; log out-of-scope findings as "Notes" rather than issues.
