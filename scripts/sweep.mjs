import { execFileSync } from 'node:child_process';

try {
  execFileSync('cargo', ['sweep', '--version'], { stdio: 'ignore' });
} catch {
  console.warn('cargo-sweep is not installed; skipping Cargo artifact cleanup.');
  process.exit(0);
}

try {
  execFileSync('cargo', ['sweep', '--maxsize', '20GB'], { cwd: new URL('../src-tauri', import.meta.url), stdio: 'inherit' });
} catch {
  console.warn('Cargo artifact cleanup failed; continuing.');
}
