---
name: issue
description: "Manage Jira issues, watchers, links, and attachments, including listing, downloading, and deleting files."
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## View, create, and update

- View: `jirac issue view <KEY>`
- Create: `jirac issue create -p <PROJECT> -t '<TYPE>' -s '<SUMMARY>' [OPTIONS]`
- Update: `jirac issue update <KEY> [OPTIONS]`

Create accepts `--assignee`, `--priority`, `--labels`, `--components`,
`--parent`, `--sprint`, `--fix-version`, `--description-file`, and repeatable
`--field '<FIELD_ID>=<VALUE>'`; update supports the applicable field flags.
For a description file, `--description-format markdown|wiki|text|adf`
overrides the configured `input_format`: `text` is literal text, `adf` is
an ADF JSON document, and `wiki` is native Jira wiki markup for API v2 only.
Without the flag, `jirac config show` reports the effective input format;
`JIRA_INPUT_FORMAT` can override it for one command. Markdown is the default.
Use `jirac issue fields -p <PROJECT> --issue-type '<TYPE>'` to discover actual
field IDs and value shapes. `--field` uses the ID literally; never assume a
story-points ID or hardcode a custom field.

For agent or non-interactive creation, ask the user for missing required values
instead of relying on prompts. Inspect required fields with `--required-only`,
provide them via appropriate flags, and use `--no-custom-fields` to skip the
CLI's custom-field prompts. This flag does not supply missing values.

Updates with `--labels`, `--components`, or `--fix-version` **replace** the
whole respective list. To add or remove items, first inspect existing values,
then send the complete intended list; leave unrelated fields untouched.

Human-readable issue views render API v2 wiki or API v3 ADF descriptions for
display. `jirac issue view <KEY> --json` retains the Jira description value
(wiki string or ADF object); do not treat rendered text as lossless source.
API v2 formatting conversions can emit warnings on stderr; report these.

## Clone, move, change type, and delete

- Clone: `jirac issue clone <KEY> [--project TARGET] [--summary TEXT] [--assignee EMAIL|me] [--move]`.
  Clone copies summary, description, type, priority, labels, components, and
  fix versions, but not comments, attachments, worklogs, or assignee by default.
  `--move` deletes the original after cloning; explicitly confirm this loss.
- Native move: `jirac issue move <KEY> <PROJECT> [--issue-type <TYPE>]`.
  Confirm first: the key changes and incompatible custom fields or workflow
  state may change, but history, comments, attachments, and worklogs remain.
- Change type: `jirac issue change-type <KEY> <TYPE>`. Confirm first: the key
  stays the same, but required fields and screens can change and incompatible
  values may be dropped.
- Delete: `jirac issue delete <KEY> [--force]`. Deletion is permanent and also
  deletes subtasks. Confirm explicitly; use `--force` only after confirmation
  when the CLI prompt cannot be answered.

## Watchers, links, and attachments

- Watch: `jirac issue watch <KEY> add [--account-id <ID>]`, `list`, or
  `rm <ID> [--force]`. Confirm removal; use `--force` only after confirmation.
- Link types: `jirac issue link list-types`; add with
  `jirac issue link add --link-type '<TYPE>' <OUTWARD-KEY> <INWARD-KEY>
  [--comment '<TEXT>']`. The outward issue is the source (e.g. blocker);
  the comment uses the configured `input_format`. Inspect existing links to
  avoid duplicates. Delete with `jirac issue link delete <LINK-ID> [--force]`
  after confirmation.
- Attach: verify the file exists, then run `jirac issue attach <KEY> <FILE>`.
- List attachments: `jirac issue attachment list <KEY> [--json]`.
- Download: `jirac issue attachment download <ID> [--out <DIR>] [--filename <NAME>] [--force]`.
  Check whether the destination exists before using `--force` to overwrite it.
- Delete: `jirac issue attachment delete <ID> [--force]`. Confirm deletion
  explicitly; use `--force` only after confirmation when the CLI prompt cannot
  be answered.
