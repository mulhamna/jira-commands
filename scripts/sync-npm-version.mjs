import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const versionPath = path.join(root, 'VERSION');
const pkgPath = path.join(root, 'packaging', 'npm', 'package.json');

const version = fs.readFileSync(versionPath, 'utf8').trim();
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`synced ${pkg.name} -> ${version}`);
