import { appendFileSync, readFileSync } from 'node:fs';

const changelog = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8');
const version = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version;

let section;
for (const line of changelog.split(/\r?\n/)) {
  const heading = line.match(/^##\s+\[?v?(\d+\.\d+\.\d+)\]?/);
  if (heading) {
    if (section) break;
    if (heading[1] === version) section = [];
  } else if (/^##\s/.test(line)) {
    if (section) break;
  } else if (section) {
    section.push(line);
  }
}

const notes = section?.join('\n').trim();
if (!notes) throw new Error(`Add a "## [${version}]" section to CHANGELOG.md before releasing.`);
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `body<<LUMA_CHANGELOG\n${notes}\nLUMA_CHANGELOG\n`);
else process.stdout.write(`${notes}\n`);
