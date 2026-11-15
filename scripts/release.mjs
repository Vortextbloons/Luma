import { execFileSync, execSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const capture = (file, args, env = process.env) => execFileSync(file, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], env }).trim();
const version = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version;
const tag = `v${version}`;
const notes = capture('node', ['scripts/changelog.mjs'], { ...process.env, GITHUB_OUTPUT: '' });
const repository = capture('gh', ['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner']);

if (capture('git', ['status', '--porcelain'])) throw new Error('Commit and push all changes before releasing.');
const head = capture('git', ['rev-parse', 'HEAD']);
let upstream;
try {
  upstream = capture('git', ['rev-parse', '@{upstream}']);
} catch {
  throw new Error('Push the current branch and set its upstream before releasing.');
}
if (head !== upstream) throw new Error('Push the current commit before releasing.');

if (capture('git', ['ls-remote', '--tags', 'origin', `refs/tags/${tag}`])) {
  throw new Error(`${tag} already exists. Delete the unfinished draft or bump the version before releasing.`);
}

if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  throw new Error('Set TAURI_SIGNING_PRIVATE_KEY to the updater private key file before releasing.');
}
let publicKey = process.env.TAURI_SIGNING_PUBLIC_KEY;
if (!publicKey) {
  try {
    publicKey = capture('gh', ['variable', 'get', 'TAURI_SIGNING_PUBLIC_KEY']);
  } catch {
    throw new Error('Set TAURI_SIGNING_PUBLIC_KEY or add the repository variable of the same name.');
  }
}

execSync('node scripts/version.mjs --check', { stdio: 'inherit' });
execSync('npm run check', { stdio: 'inherit' });
execSync('npm test', { stdio: 'inherit' });

execSync('node scripts/release-config.mjs', {
  stdio: 'inherit',
  env: { ...process.env, GITHUB_REPOSITORY: repository, TAURI_SIGNING_PUBLIC_KEY: publicKey },
});
execSync('npm run tauri -- build --config src-tauri/tauri.release.conf.json -- --locked', { stdio: 'inherit' });

const installer = `src-tauri/target/release/bundle/nsis/Luma_${version}_x64-setup.exe`;
const signature = `${installer}.sig`;
if (!existsSync(installer) || !existsSync(signature)) throw new Error(`Missing ${installer} or its signature.`);

const url = `https://github.com/${repository}/releases/download/${tag}/Luma_${version}_x64-setup.exe`;
const platforms = Object.fromEntries(['windows-x86_64', 'windows-x86_64-nsis'].map(platform => [
  platform,
  { signature: readFileSync(signature, 'utf8'), url },
]));
const manifest = 'src-tauri/target/release/bundle/latest.json';
writeFileSync(manifest, JSON.stringify({
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
}, null, 2) + '\n');
const notesFile = join(tmpdir(), `luma-${tag}-notes.md`);
writeFileSync(notesFile, `${notes}\n`);

execFileSync('gh', [
  'release', 'create', tag, installer, signature, manifest,
  '--target', head,
  '--title', `Luma ${tag}`,
  '--notes-file', notesFile,
  '--draft',
], { stdio: 'inherit' });
rmSync(notesFile, { force: true });
