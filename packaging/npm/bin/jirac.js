#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const binName = process.platform === 'win32' ? 'jirac.exe' : 'jirac';
const installedBin = path.join(__dirname, '..', 'lib', 'bin', binName);

if (!fs.existsSync(installedBin)) {
  console.error('jirac binary not installed yet. Re-run: npm rebuild -g jira-commands');
  process.exit(1);
}

const result = spawnSync(installedBin, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 0);
