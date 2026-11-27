export function notesForVersion(changelog: string, version: string) {
  let section: string[] | undefined;
  for (const line of changelog.split(/\r?\n/)) {
    if (/^##\s/.test(line)) {
      if (section) break;
      const heading = line.match(/^##\s+\[?v?(\d+\.\d+\.\d+(?:-[\w.-]+)?)\]?/);
      if (heading?.[1] === version) section = [];
    } else if (section) {
      section.push(line);
    }
  }
  return section?.join('\n').trim() ?? '';
}
