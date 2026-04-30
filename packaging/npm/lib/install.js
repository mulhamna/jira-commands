#!/usr/bin/env node
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const { pipeline } = require('node:stream/promises');
const { Readable } = require('node:stream');
const { spawnSync } = require('node:child_process');

const REPO = 'mulhamna/jira-commands';
const VERSION = require('../package.json').version;
const TAG = `v${VERSION}`;
const BASE_URL = `https://github.com/${REPO}/releases/download/${TAG}`;
const BIN_DIR = path.join(__dirname, 'bin');
const TMP_DIR = path.join(os.tmpdir(), `jirac-npm-${process.pid}-${Date.now()}`);

function getPlatformAsset() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && arch === 'arm64') return { archive: 'jirac-macos-aarch64.tar.gz', binary: 'jirac' };
  if (platform === 'darwin' && arch === 'x64') return { archive: 'jirac-macos-x86_64.tar.gz', binary: 'jirac' };
  if (platform === 'linux' && arch === 'x64') return { archive: 'jirac-linux-x86_64.tar.gz', binary: 'jirac' };
  if (platform === 'linux' && arch === 'arm64') return { archive: 'jirac-linux-aarch64.tar.gz', binary: 'jirac' };
  if (platform === 'win32' && arch === 'x64') return { archive: 'jirac-windows-x86_64.zip', binary: 'jirac.exe' };

  throw new Error(`Unsupported platform for jira-commands npm install: ${platform}/${arch}`);
}

async function fetchToFile(url, outPath) {
  const response = await fetch(url, {
    headers: {
      'user-agent': 'jira-commands-npm-installer'
    }
  });

  if (!response.ok || !response.body) {
    throw new Error(`Download failed: ${url} (${response.status})`);
  }

  await fs.promises.mkdir(path.dirname(outPath), { recursive: true });
  await pipeline(Readable.fromWeb(response.body), fs.createWriteStream(outPath));
}

async function sha256File(filePath) {
  const hash = crypto.createHash('sha256');
  const stream = fs.createReadStream(filePath);
  for await (const chunk of stream) hash.update(chunk);
  return hash.digest('hex');
}

async function extractArchive(archivePath, destDir) {
  await fs.promises.mkdir(destDir, { recursive: true });

  if (archivePath.endsWith('.tar.gz')) {
    const result = spawnSync('tar', ['-xzf', archivePath, '-C', destDir], { stdio: 'inherit' });
    if (result.status !== 0) throw new Error('Failed to extract tar.gz archive with tar');
    return;
  }

  if (archivePath.endsWith('.zip')) {
    if (process.platform === 'win32') {
      const command = `Expand-Archive -LiteralPath '${archivePath.replace(/'/g, "''")}' -DestinationPath '${destDir.replace(/'/g, "''")}' -Force`;
      const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', command], { stdio: 'inherit' });
      if (result.status !== 0) throw new Error('Failed to extract zip archive with PowerShell');
      return;
    }

    const result = spawnSync('unzip', ['-o', archivePath, '-d', destDir], { stdio: 'inherit' });
    if (result.status !== 0) throw new Error('Failed to extract zip archive with unzip');
    return;
  }

  throw new Error(`Unsupported archive format: ${archivePath}`);
}

async function main() {
  const { archive, binary } = getPlatformAsset();
  const archiveUrl = `${BASE_URL}/${archive}`;
  const checksumsUrl = `${BASE_URL}/checksums.txt`;
  const archivePath = path.join(TMP_DIR, archive);
  const checksumsPath = path.join(TMP_DIR, 'checksums.txt');
  const extractDir = path.join(TMP_DIR, 'extract');
  const finalBinPath = path.join(BIN_DIR, binary);

  console.log(`jira-commands npm installer: ${TAG} -> ${archive}`);

  await fetchToFile(checksumsUrl, checksumsPath);
  await fetchToFile(archiveUrl, archivePath);

  const checksums = await fs.promises.readFile(checksumsPath, 'utf8');
  const expectedLine = checksums.split(/\r?\n/).find((line) => line.includes(archive));
  if (!expectedLine) throw new Error(`Missing checksum entry for ${archive}`);
  const expectedSha = expectedLine.trim().split(/\s+/)[0];
  const actualSha = await sha256File(archivePath);
  if (expectedSha !== actualSha) {
    throw new Error(`Checksum mismatch for ${archive}. Expected ${expectedSha}, got ${actualSha}`);
  }

  await extractArchive(archivePath, extractDir);

  const candidates = [
    path.join(extractDir, binary),
    path.join(extractDir, archive.replace(/\.tar\.gz$/, '').replace(/\.zip$/, ''))
  ];
  const extractedBinary = candidates.find((candidate) => fs.existsSync(candidate));
  if (!extractedBinary) {
    throw new Error(`Extracted binary not found in archive: ${binary}`);
  }

  await fs.promises.mkdir(BIN_DIR, { recursive: true });
  await fs.promises.copyFile(extractedBinary, finalBinPath);

  if (process.platform !== 'win32') {
    await fs.promises.chmod(finalBinPath, 0o755);
  }

  await fs.promises.rm(TMP_DIR, { recursive: true, force: true });
  console.log(`Installed ${finalBinPath}`);
}

main().catch(async (error) => {
  console.error(error.message || error);
  try {
    await fs.promises.rm(TMP_DIR, { recursive: true, force: true });
  } catch {}
  process.exit(1);
});
