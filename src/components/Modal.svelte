<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  export let title: string;
  export let subtitle = '';
  export let onClose: () => void;
  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
    dialog.querySelector<HTMLInputElement>('input, textarea')?.focus();
  });
</script>

<dialog
  bind:this={dialog}
  class="modal"
  on:close={onClose}
  on:click={(e) => {
    if (e.target === dialog) dialog.close();
  }}
>
  <div class="modal-head">
    <div>
      <h2>{title}</h2>
      {#if subtitle}<p>{subtitle}</p>{/if}
    </div>
    <button class="icon-button" aria-label="Chiudi finestra" on:click={() => dialog.close()}
      ><Icon name="close" /></button
    >
  </div>
  <slot />
</dialog>
