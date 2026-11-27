import { appendFileSync, readFileSync } from 'node:fs';
import { notesForVersion } from './changelog-section.ts';

const changelog = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8');
const version = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version;

const notes = notesForVersion(changelog, version);
if (!notes) throw new Error(`Add a "## [${version}]" section to CHANGELOG.md before releasing.`);
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `body<<LUMA_CHANGELOG\n${notes}\nLUMA_CHANGELOG\n`);
else process.stdout.write(`${notes}\n`);
