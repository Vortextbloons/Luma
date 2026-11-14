import { writeFileSync } from 'node:fs';

const repository = process.env.GITHUB_REPOSITORY;
const pubkey = process.env.TAURI_SIGNING_PUBLIC_KEY?.trim();
if (!repository || !/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('GITHUB_REPOSITORY must be owner/repository.');
if (!pubkey) throw new Error('Set the TAURI_SIGNING_PUBLIC_KEY repository variable before releasing.');
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Set the TAURI_SIGNING_PRIVATE_KEY repository secret before releasing.');
writeFileSync('src-tauri/tauri.release.conf.json', JSON.stringify({
  bundle: { targets: ['nsis'], createUpdaterArtifacts: true, windows: { nsis: { installMode: 'currentUser' } } },
  plugins: { updater: {
    pubkey,
    endpoints: [`https://github.com/${repository}/releases/latest/download/latest.json`],
    windows: { installMode: 'quiet' },
  } },
}, null, 2) + '\n');
