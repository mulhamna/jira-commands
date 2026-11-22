// Keep the workspace entries in Cargo.lock on the versions release-please writes.
// release-please bumps Cargo.toml through `x-release-please-version` markers but
// cannot mark Cargo.lock, so without this every local cargo run rewrites it.
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function readVersion(relPath) {
  return fs.readFileSync(path.join(root, relPath), 'utf8').trim();
}

const cliVersion = readVersion('VERSION');
const mcpVersion = readVersion('crates/jira-mcp/VERSION');

const crates = [
  { name: 'jira-commands', version: cliVersion },
  { name: 'jira-core', version: cliVersion },
  { name: 'jira-mcp', version: mcpVersion },
];

const lockPath = path.join(root, 'Cargo.lock');
let lock = fs.readFileSync(lockPath, 'utf8');

for (const { name, version } of crates) {
  const entry = new RegExp(`(\\[\\[package\\]\\]\\nname = "${name}"\\nversion = ")[^"]+(")`);
  if (!entry.test(lock)) {
    console.error(`Cargo.lock has no workspace entry for ${name}`);
    process.exit(1);
  }
  lock = lock.replace(entry, `$1${version}$2`);
  console.log(`synced Cargo.lock ${name} -> ${version}`);
}

fs.writeFileSync(lockPath, lock);
