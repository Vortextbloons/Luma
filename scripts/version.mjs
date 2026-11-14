import { readFileSync, writeFileSync } from 'node:fs';

const paths = ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock'];
const contents = paths.map(path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8'));
const current = JSON.parse(contents[0]).version;
const argument = process.argv[2];
if (argument === '--check') {
  const versions = [JSON.parse(contents[1]).version, JSON.parse(contents[1]).packages[''].version, JSON.parse(contents[2]).version,
    contents[3].match(/\[package\][\s\S]*?\nversion = "([^"]+)"/)?.[1],
    contents[4].match(/\[\[package\]\]\r?\nname = "luma"\r?\nversion = "([^"]+)"/)?.[1]];
  if (versions.some(version => version !== current)) throw new Error('Versions differ. Run npm run version:set -- X.Y.Z.');
  if (process.env.GITHUB_REF_TYPE === 'tag' && process.env.GITHUB_REF_NAME !== `v${current}`) throw new Error('Release tag must match the application version.');
  console.log(`All versions match ${current}`);
  process.exit(0);
}
let version = argument;
if (['major', 'minor', 'patch'].includes(argument)) {
  const parts = current.split('.').map(Number);
  const index = ['major', 'minor', 'patch'].indexOf(argument);
  parts[index]++;
  parts.fill(0, index + 1);
  version = parts.join('.');
}
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version ?? '')) {
  throw new Error('Usage: npm run version:set -- <major|minor|patch|X.Y.Z>');
}
const updated = contents.map((content, index) => {
  if (index < 3) {
    const value = JSON.parse(content);
    value.version = version;
    if (index === 1) value.packages[''].version = version;
    return JSON.stringify(value, null, 2) + '\n';
  }
  const pattern = index === 3 ? /(\[package\][\s\S]*?\nversion = ")[^"]+/ : /(\[\[package\]\]\r?\nname = "luma"\r?\nversion = ")[^"]+/;
  if (!pattern.test(content)) throw new Error(`Cannot locate Luma version in ${paths[index]}`);
  return content.replace(pattern, `$1${version}`);
});
updated.forEach((content, index) => writeFileSync(new URL(`../${paths[index]}`, import.meta.url), content));
console.log(`Luma ${current} → ${version}`);
