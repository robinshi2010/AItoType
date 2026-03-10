#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
  symlinkSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, '..');
const tauriDir = path.join(appRoot, 'src-tauri');
const bundleDir = path.join(tauriDir, 'target', 'release', 'bundle');
const extraArgs = process.argv.slice(2);
const tempPathsToClean = [];

function resolveCommand(command) {
  if (process.platform === 'win32' && (command === 'npm' || command === 'npx')) {
    return `${command}.cmd`;
  }

  return command;
}

function run(command, args, options = {}) {
  const resolvedCommand = resolveCommand(command);
  const result = spawnSync(resolvedCommand, args, {
    cwd: appRoot,
    stdio: 'inherit',
    ...options,
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    const joinedArgs = args.join(' ');
    throw new Error(`Command failed: ${resolvedCommand} ${joinedArgs}`);
  }
}

function runCapture(command, args) {
  const resolvedCommand = resolveCommand(command);
  const result = spawnSync(resolvedCommand, args, {
    cwd: appRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  if (result.error) {
    throw result.error;
  }

  return result;
}

function findSingleBundleEntry(directory, suffix, kind) {
  if (!existsSync(directory)) {
    return null;
  }

  const entries = readdirSync(directory, { withFileTypes: true }).filter((entry) => {
    if (kind === 'directory') {
      return entry.isDirectory() && entry.name.endsWith(suffix);
    }

    return entry.isFile() && entry.name.endsWith(suffix);
  });

  if (entries.length === 0) {
    return null;
  }

  if (entries.length > 1) {
    throw new Error(`Expected one ${suffix} in ${directory}, found ${entries.length}`);
  }

  return path.join(directory, entries[0].name);
}

function verifyCodeSignature(appPath) {
  const result = runCapture('codesign', ['--verify', '--deep', '--strict', '--verbose=4', appPath]);

  return {
    ok: result.status === 0,
    output: [result.stdout, result.stderr].filter(Boolean).join('').trim(),
  };
}

function getGitTagVersion() {
  const exactTag = runCapture('git', ['describe', '--tags', '--exact-match']);
  if (exactTag.status === 0) {
    return exactTag.stdout.trim().replace(/^[vV]/, '');
  }

  const latestTag = runCapture('git', ['describe', '--tags', '--abbrev=0']);
  if (latestTag.status === 0) {
    return latestTag.stdout.trim().replace(/^[vV]/, '');
  }

  return null;
}

function buildCommandArgs() {
  const tauriConfigPath = path.join(tauriDir, 'tauri.conf.json');
  const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8'));
  const configArgs = [];

  if (tauriConfig.version === '0.1.0' && !extraArgs.includes('--config')) {
      const derivedVersion = getGitTagVersion();
    if (derivedVersion) {
      const tempRoot = mkdtempSync(path.join(os.tmpdir(), 'aitotype-tauri-config-'));
      const tempConfigPath = path.join(tempRoot, 'tauri.version.override.json');
      writeFileSync(tempConfigPath, `${JSON.stringify({ version: derivedVersion }, null, 2)}\n`);
      tempPathsToClean.push(tempRoot);
      console.warn(`Using build version ${derivedVersion} derived from git tags.`);
      configArgs.push('--config', tempConfigPath);
    }
  }

  return ['tauri', 'build', ...configArgs, ...extraArgs];
}

function rebuildDmg(appPath, dmgPath) {
  const tauriConfig = JSON.parse(readFileSync(path.join(tauriDir, 'tauri.conf.json'), 'utf8'));
  const volumeName = tauriConfig.productName || path.basename(appPath, '.app');
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), 'aitotype-dmg-'));
  const stageDir = path.join(tempRoot, 'stage');
  const stagedAppPath = path.join(stageDir, path.basename(appPath));
  const repairedDmgPath = path.join(tempRoot, path.basename(dmgPath));

  try {
    mkdirSync(stageDir, { recursive: true });
    cpSync(appPath, stagedAppPath, { recursive: true });
    symlinkSync('/Applications', path.join(stageDir, 'Applications'));

    run('hdiutil', [
      'create',
      '-volname',
      volumeName,
      '-srcfolder',
      stageDir,
      '-ov',
      '-format',
      'UDZO',
      repairedDmgPath,
    ]);

    cpSync(repairedDmgPath, dmgPath);
  } finally {
    rmSync(tempRoot, { force: true, recursive: true });
  }
}

try {
  run('npx', buildCommandArgs());
} finally {
  for (const tempPath of tempPathsToClean) {
    rmSync(tempPath, { force: true, recursive: true });
  }
}

if (process.platform !== 'darwin') {
  process.exit(0);
}

const appPath = findSingleBundleEntry(path.join(bundleDir, 'macos'), '.app', 'directory');
const dmgPath = findSingleBundleEntry(path.join(bundleDir, 'dmg'), '.dmg', 'file');

if (!appPath || !dmgPath) {
  process.exit(0);
}

const initialVerification = verifyCodeSignature(appPath);

if (initialVerification.ok) {
  process.exit(0);
}

if (initialVerification.output) {
  console.warn(initialVerification.output);
}

console.warn('Repairing invalid macOS bundle signature with ad-hoc codesign.');
run('codesign', ['--force', '--deep', '--sign', '-', '--timestamp=none', appPath]);

const repairedVerification = verifyCodeSignature(appPath);
if (!repairedVerification.ok) {
  if (repairedVerification.output) {
    console.error(repairedVerification.output);
  }
  throw new Error('Repaired macOS app bundle still failed codesign verification');
}

rebuildDmg(appPath, dmgPath);
