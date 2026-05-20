pub const ASCII_JIRAC: &str = r#"     _ _                     
    (_|_)                    
     _ _ _ __ __ _  ___     
    | | | '__/ _` |/ __|    
    | | | | | (_| | (__     
    |_|_|_|  \__,_|\___|    
"#;

pub const ROOT_LONG_ABOUT: &str = concat!(
    "     _ _                     \n",
    "    (_|_)                    \n",
    "     _ _ _ __ __ _  ___     \n",
    "    | | | '__/ _` |/ __|    \n",
    "    | | | | | (_| | (__     \n",
    "    |_|_|_|  \\__,_|\\___|    \n",
    "\n",
    "A fast Jira terminal client built in Rust.\n\n",
    "Quick start:\n",
    "  jirac auth login                       Set up credentials\n",
    "  jirac issue list                       List your assigned issues\n",
    "  jirac issue standup                    Generate a daily standup summary\n",
    "  jirac issue sprint-summary -p MYPROJ   Summarize the current sprint\n",
    "  jirac issue sprints -p MYPROJ          List project sprints and states\n",
    "  jirac tui -p MYPROJ                    Launch the interactive TUI\n",
    "  jirac issue versions -p MYPROJ         Browse project fix versions\n",
    "  jirac mcp doctor                       Check MCP prerequisites and client readiness\n",
    "  jirac mcp install --client claude-code Register jirac-mcp with Claude Code\n",
    "  jirac mcp install --client opencode     Register jirac-mcp with OpenCode\n\n",
    "Install options:\n",
    "  Homebrew:  brew tap mulhamna/tap && brew install jira-commands\n",
    "  Scoop:     scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket\n",
    "             scoop install mulhamna/jirac\n",
    "  Winget:    winget install mulhamna.jirac\n",
    "  Cargo:     cargo install jira-commands\n\n",
    "Docs:\n",
    "  README.md     Usage overview and examples\n",
    "  INSTALL.md    Detailed install instructions, including MCP helper targets\n",
    "  TUI.md        Full keyboard shortcuts and modal behavior\n",
    "  CHANGELOG.md  Release history and shipped changes\n\n",
    "Config file: ~/.config/jira/config.toml\n",
    "Env vars:    JIRA_PROFILE, JIRA_URL, JIRA_EMAIL, JIRA_TOKEN\n",
    "Version:     use --version or -V (note: -v enables verbose logging)"
);

pub const AUTH_LONG_ABOUT: &str = concat!(
    "     _ _                     \n",
    "    (_|_)                    \n",
    "     _ _ _ __ __ _  ___     \n",
    "    | | | '__/ _` |/ __|    \n",
    "    | | | | | (_| | (__     \n",
    "    |_|_|_|  \\__,_|\\___|    \n",
    "\n",
    "Manage Jira auth profiles, credentials, and active profile selection.\n\n",
    "Typical flow:\n",
    "  jirac auth login                 Create or update credentials interactively\n",
    "  jirac auth status                Show the active profile and token status\n",
    "  jirac auth profiles              List saved profiles\n",
    "  jirac auth use <profile>         Switch the active profile\n",
    "  jirac auth logout --profile foo  Clear one profile token without deleting metadata"
);

pub const AUTH_STATUS_LONG_ABOUT: &str = concat!(
    "     _ _                     \n",
    "    (_|_)                    \n",
    "     _ _ _ __ __ _  ___     \n",
    "    | | | '__/ _` |/ __|    \n",
    "    | | | | | (_| | (__     \n",
    "    |_|_|_|  \\__,_|\\___|    \n",
    "\n",
    "Show current authentication status and active profile.\n\n",
    "Useful checks:\n",
    "  jirac auth status\n",
    "  jirac auth status --profile work"
);

pub const AUTH_LOGIN_LONG_ABOUT: &str = concat!(
    "     _ _                     \n",
    "    (_|_)                    \n",
    "     _ _ _ __ __ _  ___     \n",
    "    | | | '__/ _` |/ __|    \n",
    "    | | | | | (_| | (__     \n",
    "    |_|_|_|  \\__,_|\\___|    \n",
    "\n",
    "Set up Jira credentials — URL, email/username, and token/password.\n\n",
    "Credentials are saved to ~/.config/jira/config.toml (chmod 600 on Unix).\n",
    "Override the active runtime profile with environment variables:\n",
    "  JIRA_PROFILE, JIRA_URL, JIRA_EMAIL, JIRA_TOKEN\n\n",
    "Examples:\n",
    "  jirac auth login\n",
    "  jirac auth login --profile work --url https://yourorg.atlassian.net --email you@example.com\n",
    "  jirac auth login --profile dc --deployment datacenter --auth-type datacenter-pat"
);
