<script module lang="ts">
  import type { Component } from 'svelte';

  type MarkdownView = Component<{ text?: string; live?: boolean }>;
  let markdown: MarkdownView | null = null;
  let loading: Promise<MarkdownView> | null = null;
  /** Loads the Markdown chunk once for every card; later cards render it immediately. */
  export function loadMarkdown(): Promise<MarkdownView> {
    loading ??= import('./Markdown.svelte').then((m) => (markdown = m.default));
    return loading;
  }
  const timeFormat = new Intl.DateTimeFormat('it-IT', { hour: '2-digit', minute: '2-digit' });
  const tokenFormat = new Intl.NumberFormat('it-IT');
</script>

<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ToolCard from './ToolCard.svelte';
  import type { Message } from '../lib/types';

  let { message, onReuse }: { message: Message; onReuse: (text: string) => void } = $props();
  // The legacy parent signals every item whenever the list changes. Reading fields
  // through $derived stops the update here for messages whose content is unchanged.
  const role = $derived(message.role);
  const text = $derived(message.text);
  const status = $derived(message.status);
  const thinking = $derived(message.thinking);
  const tools = $derived(message.tools);
  const error = $derived(message.error);
  const attachments = $derived(message.attachments);
  const createdAt = $derived(message.createdAt);
  const outputTokens = $derived(message.usage?.output_tokens);
  const time = $derived(timeFormat.format(createdAt));
  let Markdown = $state(markdown);
  let copied = $state(false);
  let showThinking = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;
  if (!markdown) void loadMarkdown().then((view) => (Markdown = view));
  onDestroy(() => clearTimeout(copyTimer));

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1600);
    } catch {
      copied = false;
    }
  }
</script>

<article
  class:user-message={role === 'user'}
  class="message"
  aria-label={role === 'user' ? 'Il tuo messaggio' : 'Risposta di Claude'}
>
  {#if role === 'assistant'}<div class="assistant-avatar">
      <Icon name="spark" size={21} />
    </div>{/if}
  <div class="message-body">
    {#if role === 'assistant'}<div class="message-author">
        Claude <span>{status === 'streaming' ? 'Sta lavorando' : time}</span>
      </div>{/if}
    {#if attachments?.length}<div class="message-attachments">
        {#each attachments as a}<span class="file-tag"
            ><Icon name={a.kind === 'image' ? 'image' : 'file'} size={14} />{a.name}</span
          >{/each}
      </div>{/if}
    {#if thinking}<details class="thinking" bind:open={showThinking}>
        <summary><Icon name="spark" size={14} /> Ragionamento</summary>
        {#if showThinking}<div>{thinking}</div>{/if}
      </details>{/if}
    {#if text}
      {#if role === 'user'}<div class="user-text">
          {text}
        </div>{:else if Markdown}<Markdown {text} live={status === 'streaming'} />{:else}<div
          class="user-text"
        >
          {text}
        </div>{/if}
    {:else if status === 'streaming'}<div
        class="typing-indicator"
        aria-label="In attesa della risposta"
      >
        <i></i><i></i><i></i>
      </div>{/if}
    {#if tools?.length}<div class="tool-list">
        {#each tools as tool (tool.id)}<ToolCard {tool} />{/each}
      </div>{/if}
    {#if error}<div class="message-error">
        <Icon name="shield" size={16} /><span>{error}</span>
      </div>{/if}
    {#if status === 'interrupted' && !error}<p class="muted small">Risposta interrotta.</p>{/if}
    {#if text && status !== 'streaming'}<div class="message-actions">
        <button
          class="icon-button"
          aria-label={copied ? 'Copiato' : 'Copia messaggio'}
          title="Copia messaggio"
          onclick={copy}><Icon name={copied ? 'check' : 'copy'} size={15} /></button
        ><button
          class="icon-button"
          aria-label="Riutilizza messaggio"
          title="Riutilizza nel campo di scrittura"
          onclick={() => onReuse(text)}><Icon name="edit" size={15} /></button
        >{#if outputTokens}<span>{tokenFormat.format(outputTokens)} token in uscita</span>{/if}
      </div>{/if}
  </div>
</article>
