<script lang="ts">
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  let { name, content }: { name: string; content: string } = $props();
  const markdown = $derived(/\.(md|markdown)$/i.test(name));
  const html = $derived(markdown ? DOMPurify.sanitize(marked.parse(content, { async: false }), {
    USE_PROFILES: { html: true }, FORBID_TAGS: ['style', 'img', 'video', 'audio', 'iframe', 'form'], FORBID_ATTR: ['style', 'href', 'src', 'srcset'],
  }) : '');
</script>

<div class="min-h-0 flex-1 overflow-auto bg-[#181818] p-6 text-[#d4d4d4]" role="region" aria-label={`Preview ${name}`}>
  {#if markdown}<article class="preview mx-auto max-w-3xl">{@html html}</article>
  {:else}<pre class="whitespace-pre-wrap break-words font-mono text-sm leading-6">{content}</pre>{/if}
</div>

<style>
  .preview { line-height: 1.7; font-size: 14px; overflow-wrap: anywhere; }
  .preview :global(h1) { font-size: 2em; }
  .preview :global(h2) { font-size: 1.5em; }
  .preview :global(h3) { font-size: 1.2em; }
  .preview :global(h1), .preview :global(h2), .preview :global(h3) { font-weight: 600; margin: 1em 0 .5em; }
  .preview :global(p), .preview :global(ul), .preview :global(ol), .preview :global(pre) { margin: 1em 0; }
  .preview :global(ul) { list-style: disc; padding-left: 24px; }
  .preview :global(ol) { list-style: decimal; padding-left: 24px; }
  .preview :global(pre) { padding: 16px; background: #222; overflow: auto; border-radius: 6px; }
  .preview :global(code) { font-family: ui-monospace, monospace; background: #222; padding: 2px 4px; }
  .preview :global(blockquote) { border-left: 3px solid #555; padding-left: 16px; color: #aaa; }
  .preview :global(table) { border-collapse: collapse; }
  .preview :global(td), .preview :global(th) { padding: 6px 12px; border: 1px solid #444; }
  .preview :global(a) { color: #8bb8e8; }
</style>
