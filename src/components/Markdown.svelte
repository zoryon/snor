<script lang="ts">
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { api } from '../lib/api';

  let { text = '', live = false }: { text?: string; live?: boolean } = $props();
  const sanitizeOptions = {
    FORBID_TAGS: ['img', 'iframe', 'video', 'audio', 'style', 'form', 'input', 'button'],
    FORBID_ATTR: ['style', 'id', 'name'],
  };
  // Parsing and sanitising the whole reply is the main cost while streaming:
  // live text is re-rendered at most every LIVE_INTERVAL ms, final text immediately.
  const LIVE_INTERVAL = 90;
  let throttled = $state<string | null>(null);
  let renderedAt = 0;
  let linkError = $state('');
  const source = $derived(live && throttled !== null ? throttled : text);
  const html = $derived(
    DOMPurify.sanitize(
      marked.parse(source, { async: false, breaks: true }) as string,
      sanitizeOptions,
    ),
  );

  $effect(() => {
    const value = text;
    if (!live) {
      throttled = null;
      return;
    }
    const wait = LIVE_INTERVAL - (performance.now() - renderedAt);
    const show = () => {
      renderedAt = performance.now();
      throttled = value;
    };
    if (wait <= 0) {
      show();
      return;
    }
    const timer = setTimeout(show, wait);
    return () => clearTimeout(timer);
  });

  function externalLinks(node: HTMLElement) {
    const click = (event: MouseEvent) => {
      const anchor = (event.target as HTMLElement).closest('a');
      if (anchor) {
        event.preventDefault();
        const href = anchor.getAttribute('href');
        if (href)
          void api.openLink(href).catch((e: unknown) => {
            linkError = String(e);
          });
      }
    };
    node.addEventListener('click', click);
    return {
      destroy() {
        node.removeEventListener('click', click);
      },
    };
  }
</script>

<div class="markdown" use:externalLinks>{@html html}</div>
{#if linkError}<p class="small danger-text" role="alert">{linkError}</p>{/if}
