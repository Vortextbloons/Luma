<script lang="ts">
  import { notesForVersion } from '../../../scripts/changelog-section';

  let { version, notes }: { version: string; notes?: string } = $props();

  async function renderNotes(version: string, notes?: string) {
    const [{ default: MarkdownIt }, content] = await Promise.all([
      import('markdown-it'),
      notes === undefined
        ? import('../../../CHANGELOG.md?raw').then(({ default: changelog }) => notesForVersion(changelog, version))
        : notes,
    ]);
    if (!content.trim()) return '';
    const markdown = new MarkdownIt({ html: false });
    markdown.validateLink = url => /^https?:\/\//i.test(url);
    markdown.renderer.rules.link_open = (tokens, index, options, _env, renderer) => {
      tokens[index].attrSet('target', '_blank');
      tokens[index].attrSet('rel', 'noopener noreferrer');
      return renderer.renderToken(tokens, index, options);
    };
    return markdown.render(content);
  }
</script>

<div class="release-notes">
  {#await renderNotes(version, notes)}
    <p role="status">Loading changelog…</p>
  {:then html}
    {#if html}{@html html}{:else}<p>No changelog is available for this version.</p>{/if}
  {:catch}
    <p role="alert">The changelog could not be displayed.</p>
  {/await}
</div>

<style>
  .release-notes { color: var(--text-dim); font-size: clamp(16px, 1.1vw, 21px); line-height: 1.7; overflow-wrap: anywhere; }
  .release-notes :global(:is(h1, h2, h3, h4, h5, h6)) { color: var(--text); line-height: 1.25; font-weight: 700; margin: 1.4em 0 .55em; }
  .release-notes :global(:is(h1, h2, h3)) { font-size: 1.55em; letter-spacing: -.025em; }
  .release-notes :global(:is(h4, h5, h6)) { font-size: 1.15em; }
  .release-notes > :global(:is(h1, h2, h3, h4, h5, h6, p)) { text-align: center; }
  .release-notes > :global(:first-child) { margin-top: 0; }
  .release-notes :global(p) { margin: 0 0 .9em; }
  .release-notes :global(:is(ul, ol)) { margin: 0 0 1em; padding-inline-start: 1.4em; }
  .release-notes :global(ul) { list-style: disc; }
  .release-notes :global(ol) { list-style: decimal; }
  .release-notes :global(li + li) { margin-top: .6em; }
  .release-notes :global(li::marker) { color: var(--accent); }
  .release-notes :global(strong) { color: var(--text); }
  .release-notes :global(a) { color: var(--accent-strong); text-decoration: underline; text-underline-offset: .18em; }
  .release-notes :global(:is(code, pre)) { background: var(--surface-3); border-radius: 5px; }
  .release-notes :global(code) { padding: .1em .3em; font-size: .9em; }
  .release-notes :global(pre) { padding: .8em 1em; overflow-x: auto; }
  .release-notes :global(pre code) { padding: 0; }
  .release-notes :global(img) { max-width: 100%; height: auto; }
  .release-notes :global(blockquote) { margin: 1em 0; padding-inline-start: 1em; border-inline-start: 3px solid var(--accent); }
  .release-notes :global(table) { display: block; overflow-x: auto; border-collapse: collapse; }
  .release-notes :global(:is(th, td)) { padding: .45em .65em; border: 1px solid var(--line-strong); }
  .release-notes :global(hr) { margin: 1.5em 0; border: 0; border-top: 1px solid var(--line-strong); }
</style>
