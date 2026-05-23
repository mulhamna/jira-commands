import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function readVersion(relPath) {
  return fs.readFileSync(path.join(root, relPath), 'utf8').trim();
}

const cliVersion = readVersion('VERSION');
const mcpVersion = readVersion('crates/jira-mcp/VERSION');

const targets = [
  { pkg: 'packaging/npm/package.json', version: cliVersion },
  { pkg: 'packaging/npm-mcp/package.json', version: mcpVersion },
];

for (const { pkg, version } of targets) {
  const pkgPath = path.join(root, pkg);
  if (!fs.existsSync(pkgPath)) continue;
  const data = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
  data.version = version;
  fs.writeFileSync(pkgPath, JSON.stringify(data, null, 2) + '\n');
  console.log(`synced ${data.name} -> ${version}`);
}
