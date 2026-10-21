<script lang="ts">
  import Icon from './Icon.svelte';
  import type { Tool } from '../lib/types';

  let { tool }: { tool: Tool } = $props();
  let open = $state(false);
  // Inputs and outputs can be large: they enter the DOM only while the card is open.
  const input = $derived(open ? JSON.stringify(tool.input, null, 2) : '');
</script>

<details class="tool-card" bind:open>
  <summary
    ><Icon
      name={tool.status === 'complete' ? 'check' : tool.status === 'error' ? 'close' : 'bolt'}
      size={15}
    /><span>{tool.name}</span><span class="tool-state"
      >{tool.status === 'complete'
        ? 'Completato'
        : tool.status === 'error'
          ? 'Errore'
          : 'In corso'}</span
    ><Icon name="down" size={13} /></summary
  >
  {#if open}<pre>{input}</pre>
    {#if tool.output}<pre>{tool.output}</pre>{/if}{/if}
</details>
