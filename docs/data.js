/* jirac landing — content data (install methods, commands, shortcuts, TUI themes) */
window.JIRAC = (function () {
  const REPO = "https://github.com/mulhamna/jira-commands";
  const RAW = "https://raw.githubusercontent.com/mulhamna/jira-commands/main/assets/readme";

  // ---- Install matrix: OS -> ordered methods, each a single copy-paste command ----
  const install = {
    macos: [
      { id: "brew", label: "Homebrew", recommended: true, cmd: "brew tap mulhamna/tap && brew install jira-commands jira-mcp" },
      { id: "shell", label: "Shell script", cmd: "curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash" },
      { id: "cargo", label: "Cargo", cmd: "cargo install jira-commands jira-mcp" },
      { id: "npm", label: "npm", cmd: "npm install -g @mulham28/jirac @mulham28/jirac-mcp" },
      { id: "source", label: "From source", cmd: "git clone https://github.com/mulhamna/jira-commands && cd jira-commands && cargo install --path crates/jira --locked" },
    ],
    linux: [
      { id: "shell", label: "Shell script", recommended: true, cmd: "curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash" },
      { id: "cargo", label: "Cargo", cmd: "cargo install jira-commands jira-mcp" },
      { id: "npm", label: "npm", cmd: "npm install -g @mulham28/jirac @mulham28/jirac-mcp" },
      { id: "source", label: "From source", cmd: "git clone https://github.com/mulhamna/jira-commands && cd jira-commands && cargo install --path crates/jira --locked" },
    ],
    windows: [
      { id: "winget", label: "Winget", recommended: true, cmd: "winget install mulhamna.jirac mulhamna.jirac-mcp" },
      { id: "scoop", label: "Scoop", cmd: "scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket && scoop install mulhamna/jirac mulhamna/jirac-mcp" },
      { id: "powershell", label: "PowerShell", cmd: "powershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\"" },
      { id: "npm", label: "npm", cmd: "npm install -g @mulham28/jirac @mulham28/jirac-mcp" },
      { id: "cargo", label: "Cargo", cmd: "cargo install jira-commands jira-mcp" },
    ],
  };

  // ClawHub / OpenClaw extras (shown as a small footnote row)
  const clawhub = [
    { label: "ClawHub skill", cmd: "openclaw skills install jirac" },
    { label: "ClawHub plugin", cmd: "openclaw plugins install clawhub:jirac-plugin" },
  ];

  // ---- Command explorer ----
  const commands = [
    { group: "Issues", use: "List issues assigned to you", cmd: "jirac issue list" },
    { group: "Issues", use: "List issues by project", cmd: "jirac issue list --project MYPROJ" },
    { group: "Issues", use: "View one issue in detail", cmd: "jirac issue view MYPROJ-123" },
    { group: "Issues", use: "Create a new issue", cmd: "jirac issue create --project MYPROJ" },
    { group: "Issues", use: "Move an issue to another status", cmd: "jirac issue transition MYPROJ-123 --to Done" },
    { group: "Issues", use: "Change an issue's type (native)", cmd: "jirac issue change-type MYPROJ-123 Story" },
    { group: "Issues", use: "Move an issue to another project (native)", cmd: "jirac issue move MYPROJ-123 OTHER" },
    { group: "Issues", use: "Upload an attachment", cmd: "jirac issue attach MYPROJ-123 ./screenshot.png" },
    { group: "Issues", use: "List attachments on an issue", cmd: "jirac issue attachment list MYPROJ-123" },
    { group: "Issues", use: "Download an attachment by ID", cmd: "jirac issue attachment download 10100 --out ./tmp" },
    { group: "Issues", use: "Delete an attachment by ID", cmd: "jirac issue attachment delete 10100 --force" },
    { group: "Boards", use: "List Agile boards (by project)", cmd: "jirac board list --project MYPROJ" },
    { group: "Boards", use: "Show one Agile board", cmd: "jirac board get 12" },
    { group: "Boards", use: "List issues currently on a board", cmd: "jirac board issues 12 --jql 'status = \"To Do\"'" },
    { group: "Boards", use: "List backlog issues on a board", cmd: "jirac board backlog 12 --max 50" },
    { group: "Reports", use: "Daily standup summary", cmd: "jirac issue standup" },
    { group: "Reports", use: "Current sprint summary", cmd: "jirac issue sprint-summary --project MYPROJ" },
    { group: "Reports", use: "Mention notifications", cmd: "jirac issue notifications --since 7d" },
    { group: "Versions", use: "Browse project fix versions", cmd: "jirac issue versions --project MYPROJ" },
    { group: "Versions", use: "Preview backlog for one fix version", cmd: "jirac issue versions --project MYPROJ --version \"v1.2.0\"" },
    { group: "Versions", use: "Create a project version", cmd: "jirac issue versions --project MYPROJ --create --version \"v1.3.0\"" },
    { group: "Versions", use: "Set a release date on a version", cmd: "jirac issue versions --project MYPROJ --version \"v1.2.0\" --set-release-date 2026-06-30 --released" },
    { group: "Worklog", use: "Add a worklog", cmd: "jirac issue worklog add MYPROJ-123 --time 2h --comment \"Worked on API\"" },
    { group: "Worklog", use: "Add a worklog with a custom start time", cmd: "jirac issue worklog add MYPROJ-123 --time 2h --date 2026-04-21 --start 09:30 --comment \"Backfilled\"" },
    { group: "Worklog", use: "Backfill worklogs across a date range", cmd: "jirac issue worklog add MYPROJ-123 --time 2h --from 2026-04-21 --to 2026-04-25 --exclude-weekends" },
    { group: "Watch", use: "Watch an issue (add yourself)", cmd: "jirac issue watch MYPROJ-123 add" },
    { group: "Watch", use: "Add another user as watcher", cmd: "jirac issue watch MYPROJ-123 add --account-id 5b10ac8d82e05b22cc7d4ef5" },
    { group: "Watch", use: "List watchers on an issue", cmd: "jirac issue watch MYPROJ-123 list" },
    { group: "Watch", use: "Remove a watcher", cmd: "jirac issue watch MYPROJ-123 rm 5b10ac8d82e05b22cc7d4ef5" },
    { group: "Bulk", use: "Bulk comment by JQL", cmd: "jirac issue bulk-comment --jql 'project = MYPROJ AND sprint = openSprints()' --body \"Status before standup\"" },
    { group: "Bulk", use: "Bulk comment by explicit keys", cmd: "jirac issue bulk-comment --keys MYPROJ-123 MYPROJ-456 --file note.md" },
    { group: "Bulk", use: "Bulk transition by JQL", cmd: "jirac issue bulk-transition --jql 'project = MYPROJ AND status = \"To Do\"' --to \"In Progress\"" },
    { group: "Bulk", use: "Bulk update a field", cmd: "jirac issue bulk-update --jql 'project = MYPROJ AND assignee = EMPTY' --assignee me" },
    { group: "Bulk", use: "Bulk create from a manifest", cmd: "jirac issue bulk-create --manifest issues.json" },
    { group: "Search", use: "Interactive JQL builder", cmd: "jirac issue jql" },
    { group: "Search", use: "JQL from structured params", cmd: "jirac issue jql --params '{\"project\":\"MYPROJ\",\"status\":[\"In Progress\"]}'" },
    { group: "Auth", use: "Log in for the first time", cmd: "jirac auth login" },
    { group: "Auth", use: "Check connection status", cmd: "jirac auth status" },
    { group: "Auth", use: "Switch between profiles", cmd: "jirac auth use work-cloud" },
    { group: "TUI", use: "Launch the interactive TUI", cmd: "jirac tui -p MYPROJ" },
    { group: "API", use: "Make a raw API call", cmd: "jirac api get /rest/api/3/serverInfo" },
  ];

  // ---- TUI keyboard shortcuts ----
  const shortcuts = [
    { keys: ["↑", "k"], desc: "Move to the issue above", group: "Navigate" },
    { keys: ["↓", "j"], desc: "Move to the issue below", group: "Navigate" },
    { keys: ["Enter"], desc: "Open details for the selected issue", group: "Navigate" },
    { keys: ["?"], desc: "Show shortcut help", group: "Navigate" },
    { keys: ["c"], desc: "Create a new issue", group: "Act" },
    { keys: ["e"], desc: "Edit the selected issue", group: "Act" },
    { keys: ["t"], desc: "Change status (transition)", group: "Act" },
    { keys: ["y"], desc: "Change backlog item type in a modal", group: "Act" },
    { keys: ["M"], desc: "Move item to another project", group: "Act" },
    { keys: ["s"], desc: "Assign the issue to a sprint", group: "Act" },
    { keys: ["u"], desc: "Upload an attachment", group: "Act" },
    { keys: [";"], desc: "Add one comment to the selected issue", group: "Act" },
    { keys: [":"], desc: "Bulk-comment many issues by JQL or keys", group: "Act" },
    { keys: ["w"], desc: "Add a single worklog", group: "Act" },
    { keys: ["W"], desc: "Watch the issue (add yourself as a watcher)", group: "Act" },
    { keys: ["b"], desc: "Add bulk worklogs across a date range", group: "Act" },
    { keys: ["v"], desc: "Edit fix versions", group: "Act" },
    { keys: ["n"], desc: "Scan and open mention notifications", group: "View" },
    { keys: ["R"], desc: "Mark the selected notification as read", group: "View" },
    { keys: ["V"], desc: "Browse project fix versions + backlog", group: "View" },
    { keys: ["B"], desc: "Open project-scoped Agile board picker", group: "View" },
    { keys: ["p"], desc: "Open saved JQL queries", group: "View" },
    { keys: ["C"], desc: "Choose visible table columns (persisted)", group: "View" },
    { keys: ["T"], desc: "Open the theme picker", group: "View" },
  ];

  // ---- Faux-TUI theme presets (for the live preview) ----
  const themes = [
    { id: "default", name: "Default Dark", c: { bg: "#0b1220", panel: "#0e1626", border: "#1e2a44", text: "#cdd6f4", dim: "#7d8db0", accent: "#4e97ff", sel: "#13294d", green: "#34d058", yellow: "#e3b341", red: "#ff6b6b" } },
    { id: "github-light", name: "GitHub Light", c: { bg: "#ffffff", panel: "#f6f8fa", border: "#d0d7de", text: "#1f2328", dim: "#656d76", accent: "#0969da", sel: "#ddf4ff", green: "#1a7f37", yellow: "#9a6700", red: "#cf222e" } },
    { id: "kanagawa", name: "Kanagawa Wave", c: { bg: "#1f1f28", panel: "#2a2a37", border: "#363646", text: "#dcd7ba", dim: "#727169", accent: "#7e9cd8", sel: "#363646", green: "#98bb6c", yellow: "#e6c384", red: "#e46876" } },
    { id: "tokyonight", name: "Tokyo Night", c: { bg: "#1a1b26", panel: "#1f2335", border: "#2f334d", text: "#c0caf5", dim: "#565f89", accent: "#7aa2f7", sel: "#283457", green: "#9ece6a", yellow: "#e0af68", red: "#f7768e" } },
    { id: "gruvbox", name: "Gruvbox", c: { bg: "#282828", panel: "#32302f", border: "#504945", text: "#ebdbb2", dim: "#a89984", accent: "#83a598", sel: "#3c3836", green: "#b8bb26", yellow: "#fabd2f", red: "#fb4934" } },
  ];

  // Sample rows for the faux-TUI
  const tuiRows = [
    { key: "CORE-183", status: "In Progress", st: "prog", summary: "Refactor auth error handling" },
    { key: "CORE-177", status: "To Do", st: "todo", summary: "Add MCP transition confirmation" },
    { key: "CORE-171", status: "Done", st: "done", summary: "Improve TUI issue detail renderer" },
    { key: "CORE-168", status: "In Review", st: "rev", summary: "Cursor pagination for /search/jql" },
    { key: "CORE-160", status: "To Do", st: "todo", summary: "Backfill worklog date-range flow" },
  ];

  return { REPO, RAW, install, clawhub, commands, shortcuts, themes, tuiRows };
})();
