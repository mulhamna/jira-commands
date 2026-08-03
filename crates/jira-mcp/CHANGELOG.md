# Changelog

## [2.3.5](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.3.4...jira-mcp-v2.3.5) (2026-08-03)


### Bug Fixes

* **jira-mcp:** trigger patch release for custom field guidance ([280910b](https://github.com/mulhamna/jira-commands/commit/280910be03819815df63f012803f13d6186552a7))

## [2.3.4](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.3.3...jira-mcp-v2.3.4) (2026-07-23)


### Bug Fixes

* honest jira_issue_create errors (-32602 on 4xx, keep issue) ([#430](https://github.com/mulhamna/jira-commands/issues/430)) ([c574b53](https://github.com/mulhamna/jira-commands/commit/c574b538ebac45f5abf56f72e197390483865d70))

## [2.3.3](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.3.2...jira-mcp-v2.3.3) (2026-06-29)


### Bug Fixes

* **jira-mcp:** structured validation errors + jira_search_users ([#408](https://github.com/mulhamna/jira-commands/issues/408)) ([5d14ef2](https://github.com/mulhamna/jira-commands/commit/5d14ef2dcd9f8daa0688d07b1933f5607c6d1c8b))

## [2.3.2](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.3.1...jira-mcp-v2.3.2) (2026-06-23)


### Bug Fixes

* **jira-mcp:** document release lane to trigger mcp release ([#402](https://github.com/mulhamna/jira-commands/issues/402)) ([1f53933](https://github.com/mulhamna/jira-commands/commit/1f539338c2ad4622c4f94b79da7b8617ba02c3ad))

## [2.3.1](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.3.0...jira-mcp-v2.3.1) (2026-06-22)


### Bug Fixes

* **jira-mcp:** document markdown/ADF affordance on issue tools ([#391](https://github.com/mulhamna/jira-commands/issues/391)) ([cf74365](https://github.com/mulhamna/jira-commands/commit/cf74365bdbcae7586fb6f0d897c5fe6b5dfca9da))

## [2.3.0](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.2.1...jira-mcp-v2.3.0) (2026-06-21)


### Features

* **jira-mcp:** add issue move, whoami, server-info, and notification mark-read tools ([#384](https://github.com/mulhamna/jira-commands/issues/384)) ([bc00e39](https://github.com/mulhamna/jira-commands/commit/bc00e3959b84e61bc9be56adc73dfb639d428dc0))

## [2.2.1](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.2.0...jira-mcp-v2.2.1) (2026-06-19)


### Bug Fixes

* **jira-mcp:** bound tool responses under stdio line limits ([#378](https://github.com/mulhamna/jira-commands/issues/378)) ([c1d058a](https://github.com/mulhamna/jira-commands/commit/c1d058a6e5888ec0136f196f6c329288783cc114))

## [2.2.0](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.1.1...jira-mcp-v2.2.0) (2026-06-15)


### Features

* **jira-mcp:** refactor app.rs + add watcher/jql/attachment/board tools ([#372](https://github.com/mulhamna/jira-commands/issues/372)) ([e1d17b6](https://github.com/mulhamna/jira-commands/commit/e1d17b650e28521ff46a2e43bcf840ea0d209435))

## [2.1.1](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.1.0...jira-mcp-v2.1.1) (2026-06-10)


### Bug Fixes

* **release:** route mcp work to the mcp lane and stop jirac leakage ([#361](https://github.com/mulhamna/jira-commands/issues/361)) ([2c3f6fd](https://github.com/mulhamna/jira-commands/commit/2c3f6fde483bfbdd923d4f769addec572f13b96d))

## [2.1.0](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.0.1...jira-mcp-v2.1.0) (2026-06-02)


### Features

* **mcp:** add bulk comment and batch tools ([#319](https://github.com/mulhamna/jira-commands/issues/319)) ([496d4fc](https://github.com/mulhamna/jira-commands/commit/496d4fc1070444bd3595056b810102cbf0e71302))
* **mcp:** add jira issue clone tool ([#317](https://github.com/mulhamna/jira-commands/issues/317)) ([fe30984](https://github.com/mulhamna/jira-commands/commit/fe309849deeec39a324859cb772a79237e9994f3))
* **mcp:** add standup, sprint summary, and notifications ([#320](https://github.com/mulhamna/jira-commands/issues/320)) ([ec4c2f8](https://github.com/mulhamna/jira-commands/commit/ec4c2f81f4a961855c552f34a19ce74b4df2334f))

## [2.0.1](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v2.0.0...jira-mcp-v2.0.1) (2026-05-24)


### Bug Fixes

* **jira-mcp:** implement Display and std::error::Error for AppError ([#291](https://github.com/mulhamna/jira-commands/issues/291)) ([5f9bfec](https://github.com/mulhamna/jira-commands/commit/5f9bfecea06621f2bca2cf836fa80987176658ca))

## [2.0.0](https://github.com/mulhamna/jira-commands/compare/jira-mcp-v1.2.0...jira-mcp-v2.0.0) (2026-05-24)


### ⚠ BREAKING CHANGES

* **jira-core,tui:** get_remote_links and get_project_components return types changed from Vec<Value> to typed Vec<RemoteLink> / Vec<Component>.

### Features

* add Jira Data Center multi-profile auth support ([#111](https://github.com/mulhamna/jira-commands/issues/111)) ([4345cfd](https://github.com/mulhamna/jira-commands/commit/4345cfd28a82a3caca46be4eb47de8e9e9e41d43))
* add Jira issue comment support ([511ec27](https://github.com/mulhamna/jira-commands/commit/511ec279591d592ef628de299e24396c20db718d))
* add jira mcp ([2189838](https://github.com/mulhamna/jira-commands/commit/21898386c4f0c8e45c8d97a33e35f1cdd3075d88))
* add Jira standups, sprint summaries, and release lane fixes ([#244](https://github.com/mulhamna/jira-commands/issues/244)) ([291572c](https://github.com/mulhamna/jira-commands/commit/291572cdd3ab7c1b10b269d8bcfa7fdab55d9e15))
* add Zed MCP extension wrapper ([#225](https://github.com/mulhamna/jira-commands/issues/225)) ([1910064](https://github.com/mulhamna/jira-commands/commit/191006410a6e5d86b143054f044caee6de6e9da0))
* finish split TUI panels, prefs overlays, and release packaging cleanup ([#114](https://github.com/mulhamna/jira-commands/issues/114)) ([e2b2328](https://github.com/mulhamna/jira-commands/commit/e2b2328035f3bd2924708994e57bb5ea2f6e9504))
* **jira-mcp:** add issue and remote link tools ([#281](https://github.com/mulhamna/jira-commands/issues/281)) ([2b9f9d4](https://github.com/mulhamna/jira-commands/commit/2b9f9d4dc9cc2050bae9225f96a3ff438308e0f0))
* **jira-mcp:** add sprint and project metadata tools ([#277](https://github.com/mulhamna/jira-commands/issues/277)) ([8f86ae3](https://github.com/mulhamna/jira-commands/commit/8f86ae3958e5324419eb16c05947c25d0b48ff49))
* **plugin:** 8 new skills + clawhub SKILL refresh + install docs sync ([#266](https://github.com/mulhamna/jira-commands/issues/266)) ([f34ad5e](https://github.com/mulhamna/jira-commands/commit/f34ad5e646187a2de126775b972e5c5d5cb63873))
* release please config crates ([18f7646](https://github.com/mulhamna/jira-commands/commit/18f764624055e4d4bf1b35b0e09006cbf71966b1))
* **release:** add jira-mcp scoop docs and automation ([#284](https://github.com/mulhamna/jira-commands/issues/284)) ([e4f3459](https://github.com/mulhamna/jira-commands/commit/e4f34599db275fe221d8c886b7d8b756709c5097))
* stabilize release-please workspace publishing ([e1e0b56](https://github.com/mulhamna/jira-commands/commit/e1e0b561961f828f7ee162117eba3790f3ede772))


### Bug Fixes

* align homebrew + MCP install and release flow ([#141](https://github.com/mulhamna/jira-commands/issues/141)) ([82fa15d](https://github.com/mulhamna/jira-commands/commit/82fa15db50b823b63632e612bc23ee31616465a2))
* **jira-mcp:** replace boolean schemas with object schemas for arbitrary JSON fields ([01f6c62](https://github.com/mulhamna/jira-commands/commit/01f6c6242def958ca4ff1999d51048ff3ec9af3e))
* **jira-mcp:** shorten keyword to fit crates.io 20-char limit ([437540e](https://github.com/mulhamna/jira-commands/commit/437540ebb16dec94c9d73d73c6647aea5da5cd0b))
* repair CI action pin and tidy crate READMEs ([150a5f5](https://github.com/mulhamna/jira-commands/commit/150a5f51ac331478b92294315d4baf5f839e5dbd))
* write opencode MCP config directly ([#249](https://github.com/mulhamna/jira-commands/issues/249)) ([0568369](https://github.com/mulhamna/jira-commands/commit/0568369364ac22f3522e6d84e3702b0af9b108bb))


### Miscellaneous Chores

* **jira-core,tui:** SDK polish + mouse support for 1.0.0 ([#253](https://github.com/mulhamna/jira-commands/issues/253)) ([1b35d0e](https://github.com/mulhamna/jira-commands/commit/1b35d0e70b0fa1810890345f1305f6a0fc3efa08))
