<script lang="ts">
  export let name = 'spark';
  export let size = 18;
  const paths: Record<string, string> = {
    plus: 'M12 5v14M5 12h14',
    search: 'm21 21-4.4-4.4M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
    chat: 'M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.4 8.4 0 0 1 3.8-.9h.5a8.5 8.5 0 0 1 8 8v.5Z',
    folder: 'M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z',
    settings:
      'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM9 3l-.5 2-2 .8-1.8-.7-2 3.5 1.5 1.3-.2 2.2-1.5 1.4 2 3.5 2-.6 1.8 1 1 2.1h4l.8-2 2-.8 1.8.6 2-3.5-1.5-1.4.2-2.2 1.5-1.3-2-3.5-2 .6-1.8-1L13 3H9Z',
    arrow: 'M12 19V5m-6 6 6-6 6 6',
    chevron: 'm9 5 7 7-7 7',
    down: 'm6 9 6 6 6-6',
    paperclip:
      'm21.4 11.6-9.2 9.2a6 6 0 0 1-8.5-8.5L13.6 2.4a4 4 0 0 1 5.7 5.7l-9.9 9.9a2 2 0 0 1-2.8-2.8l9.2-9.2',
    code: 'm8 5-7 7 7 7m8-14 7 7-7 7m-3-17-2 20',
    check: 'm5 12 4 4L19 6',
    close: 'm6 6 12 12M6 18 18 6',
    copy: 'M9 9h12v12H9zM15 5V3H3v12h2',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    panel: 'M3 4h18v16H3zM9 4v16',
    download: 'M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4',
    stop: 'M6 6h12v12H6z',
    bolt: 'm13 2-9 12h7l-1 8 10-12h-7l0-8Z',
    file: 'M14 2H4v20h16V8l-6-6ZM14 2v6h6M8 13h8M8 17h5',
    moon: 'M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8Z',
    sun: 'M12 3v1m0 16v1M3 12h1m16 0h1M5.6 5.6l.7.7m11.4 11.4.7.7m0-12.8-.7.7M6.3 17.7l-.7.7M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
    pin: 'm16 3 5 5-4 2-3 5-2-2-6 6m4-6-2-2 5-3 2-4Z',
    archive: 'M3 3h18v5H3zM5 8v13h14V8M10 12h4',
    trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7',
    edit: 'm16 3 5 5-13 13H3v-5L16 3ZM13 6l5 5',
    book: 'M4 3h6a2 2 0 0 1 2 2v16a3 3 0 0 0-3-2H4V3Zm16 0h-6a2 2 0 0 0-2 2v16a3 3 0 0 1 3-2h5V3Z',
    spark: 'm12 3 2.6 6.4L21 12l-6.4 2.6L12 21l-2.6-6.4L3 12l6.4-2.6L12 3Z',
    leaf: 'M20 3C9 2 2 7 5 15s16 6 15-12ZM5 20 16 8',
    clock: 'M12 8v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
    shield: 'm12 2 9 4v6c0 5-9 10-9 10S3 17 3 12V6l9-4Zm-4 10 3 3 5-6',
    external: 'M14 3h7v7m0-7L11 13M10 3H3v18h18v-7',
    refresh: 'M20 7a9 9 0 1 0 1 9M20 2v5h-5',
    image: 'M3 3h18v18H3zM3 17l6-6 4 4 3-3 5 5M16 7h.01',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name] || paths.spark} /></svg
>
