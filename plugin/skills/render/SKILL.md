---
description: Render and validate Markdown content as Jira ADF with jirac before sending it to Jira as a description or comment
---

Preview how Markdown will convert to Atlassian Document Format (ADF) before posting it as a Jira issue description or comment.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Determine the input source:
   - File path: use `--input <path>`.
   - Inline content: pass via stdin (`echo '...' | jirac issue render`).
3. Run `jirac issue render --input file.md` (or stdin variant).
4. Show the rendered ADF preview. Highlight any nodes that may not render cleanly in Jira (unsupported HTML, raw images, complex tables).
5. If the result looks correct, the same content can be fed into `jirac issue update --description-file`, `jirac issue comment add --body-file`, or used in batch/bulk manifests.

Notes:
- Useful when authoring rich descriptions with mentions, code blocks, lists, and tables — confirms conversion before going live.
- Jira ADF supports a subset of Markdown: headings, bold/italic, code, lists, links, tables, blockquotes. Raw HTML is generally stripped.
- For @mention preview, the rendered ADF will contain mention nodes when the source uses the `@username` syntax that the CLI recognizes.

Examples:
- "preview this markdown before posting" → `jirac issue render --input draft.md`
- "check if my description renders correctly" → `jirac issue render --input description.md`
