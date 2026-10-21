<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Icon from './components/Icon.svelte';
  import Modal from './components/Modal.svelte';
  import MessageCard, { loadMarkdown } from './components/MessageCard.svelte';
  import { api, isDesktop, subscribe, pickDirectory, pickFiles } from './lib/api';
  import { reduceMessage, recoveredMessage } from './lib/events';
  import {
    defaultSettings,
    modeLabels,
    modelLabels,
    type Chat,
    type Project,
    type Message,
    type Settings,
    type Diagnostic,
    type Attachment,
    type AgentEvent,
    type Permission,
    type Mode,
    type Model,
  } from './lib/types';

  let chats: Chat[] = [];
  let projects: Project[] = [];
  let settings: Settings = { ...defaultSettings };
  let diagnostic: Diagnostic | null = null;
  let activeId: string | null = null;
  let activeProject: string | null = null;
  let messages: Message[] = [];
  let draft = '';
  let mode: Mode = 'chat';
  let model: Model = 'sonnet';
  let attachments: Attachment[] = [];
  let sidebar = true;
  let view: 'chat' | 'projects' | 'archive' = 'chat';
  let showSettings = false;
  let settingsTab: 'general' | 'connection' | 'about' = 'general';
  let showSearch = false;
  let search = '';
  let searchResults: Chat[] = [];
  let showProject = false;
  let projectName = '';
  let projectPath = '';
  let projectInstructions = '';
  let projectEditId: string | undefined;
  let showRename = false;
  let renameValue = '';
  let showDelete = false;
  let menu = false;
  let error = '';
  let toast = '';
  let booted = false;
  let checking = false;
  let loggingIn = false;
  let sending = false;
  let busyChatId: string | null = null;
  let busyMessageId: string | null = null;
  let permissions: Permission[] = [];
  let deciding: string[] = [];
  let more = false;
  let cursor: number | null = null;
  let historyLoading = false;
  let projectSaving = false;
  let scrollElement: HTMLDivElement;
  let composer: HTMLTextAreaElement;
  let followOutput = true;
  let loadGeneration = 0;
  let queuedEvents: AgentEvent[] = [];
  let frameQueue: AgentEvent[] = [];
  let frameTimer: ReturnType<typeof setTimeout> | undefined;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let draftTimer: ReturnType<typeof setTimeout> | undefined;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  let unlisten = () => {};
  let disposed = false;

  $: activeChat = chats.find((c) => c.id === activeId);
  $: selectedProject = projects.find((p) => p.id === activeProject);
  $: visibleChats = chats.filter(
    (c) => !c.archived && (!activeProject || c.projectId === activeProject),
  );
  $: pinned = visibleChats.filter((c) => c.pinned);
  $: recent = visibleChats.filter((c) => !c.pinned);
  $: busy = busyChatId !== null || sending;
  $: {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = settings.theme;
      document.documentElement.classList.toggle('compact', settings.compact);
    }
  }
  $: if (booted) {
    const value = draft;
    const id = activeId;
    clearTimeout(draftTimer);
    draftTimer = setTimeout(() => storeDraft(`snor.draft.${id || 'new'}`, value), 250);
  }
  $: if (showSearch) {
    const q = search;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(async () => {
      try {
        const results = await api.search(q);
        if (search === q) searchResults = results;
      } catch (e) {
        fail(e);
      }
    }, 150);
  }

  onMount(() => {
    const start = performance.now();
    // Listener registration and state loading are independent IPC round-trips.
    const listening = subscribe(onAgentEvent);
    const keep = (off: () => void) => (disposed ? off() : (unlisten = off));
    (async () => {
      try {
        const data = await api.bootstrap();
        chats = data.chats;
        projects = data.projects;
        settings = { ...defaultSettings, ...data.settings };
        model = settings.model;
        keep(await listening);
        if (disposed) return;
        booted = true;
        const last = localStorage.getItem('snor.last-chat');
        if (last && chats.some((c) => c.id === last && !c.archived)) await openChat(last);
        else draft = localStorage.getItem('snor.draft.new') || '';
        performance.mark('snor:ready');
        await tick();
        requestAnimationFrame(() => {
          void api.ready().catch(() => {});
        });
        console.info(
          `[Snor] UI ready after ${Math.round(performance.now() - start)} ms (frontend only)`,
        );
        void checkConnection();
        // Fetch the Markdown chunk once the UI is idle, so the first reply renders directly.
        if ('requestIdleCallback' in window) requestIdleCallback(() => void loadMarkdown());
        else setTimeout(() => void loadMarkdown(), 500);
      } catch (e) {
        booted = true;
        fail(e);
        void listening.then(keep, () => {});
      }
    })();
    const keydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        showSearch = !showSearch;
        search = '';
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
        e.preventDefault();
        newChat();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault();
        showSettings = true;
      }
      if (e.key === 'Escape') menu = false;
    };
    document.addEventListener('keydown', keydown);
    return () => {
      disposed = true;
      unlisten();
      clearTimeout(frameTimer);
      clearTimeout(searchTimer);
      clearTimeout(draftTimer);
      clearTimeout(toastTimer);
      document.removeEventListener('keydown', keydown);
    };
  });

  function fail(e: unknown) {
    error = e instanceof Error ? e.message : String(e);
  }
  function notify(text: string) {
    toast = text;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ''), 3000);
  }
  function draftKey() {
    return `snor.draft.${activeId || 'new'}`;
  }
  function storeDraft(key: string, value: string) {
    try {
      // Empty drafts are removed so localStorage does not keep one key per chat forever.
      if (value) localStorage.setItem(key, value);
      else localStorage.removeItem(key);
    } catch {
      /* Optional draft storage can be disabled by the OS; keep the draft in memory. */
    }
  }
  function stashDraft() {
    storeDraft(draftKey(), draft);
  }
  async function scrollBottom(force = false) {
    await tick();
    if ((followOutput || force) && scrollElement)
      scrollElement.scrollTop = scrollElement.scrollHeight;
  }
  function onScroll() {
    if (scrollElement)
      followOutput =
        scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight < 100;
  }
  function focusComposer() {
    void tick().then(() => composer?.focus());
  }
  function onAgentEvent(event: AgentEvent) {
    if (sending) {
      queuedEvents.push(event);
      return;
    }
    if (event.kind === 'delta' || event.kind === 'thinking') {
      // Rust already coalesces deltas; this only merges text and reasoning of one frame.
      frameQueue.push(event);
      if (!frameTimer) frameTimer = setTimeout(flushFrames, 16);
      return;
    }
    flushFrames();
    applyEvent(event);
  }
  /** Applies events to the affected messages only; returns whether anything changed. */
  function applyToMessages(events: AgentEvent[]) {
    let next: Message[] | null = null;
    for (const event of events) {
      if (event.chatId !== activeId) continue;
      const list: Message[] = next ?? messages;
      let index = list.length - 1;
      while (index >= 0 && list[index].id !== event.messageId) index--;
      if (index < 0) continue;
      const updated = reduceMessage(list[index], event);
      if (updated === list[index]) continue;
      next ??= messages.slice();
      next[index] = updated;
    }
    if (next) messages = next;
    return next !== null;
  }
  function flushFrames() {
    clearTimeout(frameTimer);
    frameTimer = undefined;
    const batch = frameQueue;
    frameQueue = [];
    if (applyToMessages(batch)) void scrollBottom();
  }
  function applyEvent(event: AgentEvent) {
    if (event.chatId === activeId) {
      applyToMessages([event]);
      if (event.kind === 'permission')
        permissions = [...permissions.filter((p) => p.id !== event.data.id), event.data];
      if (event.kind === 'permission_cancelled')
        permissions = permissions.filter((p) => p.id !== event.data.id);
      if (event.kind === 'init')
        chats = chats.map((c) =>
          c.id === activeId ? { ...c, sessionId: event.data.sessionId } : c,
        );
    }
    if (
      ['done', 'failure', 'closed'].includes(event.kind) &&
      busyChatId === event.chatId &&
      event.messageId === busyMessageId
    ) {
      busyChatId = null;
      busyMessageId = null;
      permissions = [];
      if (event.kind === 'failure') fail(event.data);
      if (event.kind === 'done' && event.chatId !== activeId)
        notify('Claude ha completato la risposta.');
    }
    void scrollBottom();
  }
  function newChat() {
    if (sending) return;
    if (busyChatId) {
      activeProject = activeChat?.projectId || null;
      notify('Attendi o interrompi il turno attivo prima di aprire una nuova conversazione.');
      return;
    }
    stashDraft();
    loadGeneration++;
    activeId = null;
    messages = [];
    attachments = [];
    permissions = [];
    more = false;
    error = '';
    view = 'chat';
    menu = false;
    draft = localStorage.getItem('snor.draft.new') || '';
    mode = 'chat';
    model = settings.model;
    localStorage.removeItem('snor.last-chat');
    focusComposer();
  }
  async function openChat(id: string) {
    if (busyChatId === id && activeId === id) {
      view = 'chat';
      focusComposer();
      return;
    }
    if (sending || (busyChatId && busyChatId !== id)) {
      notify('Interrompi o attendi il turno attivo prima di cambiare conversazione.');
      return;
    }
    stashDraft();
    const generation = ++loadGeneration;
    activeId = id;
    view = 'chat';
    messages = [];
    attachments = [];
    permissions = [];
    menu = false;
    error = '';
    more = false;
    const chat = chats.find((c) => c.id === id);
    if (!chat) return;
    mode = chat.mode;
    model = chat.model;
    activeProject = chat.projectId;
    draft = localStorage.getItem(`snor.draft.${id}`) || '';
    localStorage.setItem('snor.last-chat', id);
    try {
      const page = await api.messages(id);
      if (generation !== loadGeneration) return;
      messages = page.messages.map(recoveredMessage);
      more = page.hasMore;
      cursor = page.cursor;
      followOutput = true;
      void scrollBottom(true);
    } catch (e) {
      fail(e);
    }
    focusComposer();
  }
  async function loadOlder() {
    if (!activeId || historyLoading) return;
    const id = activeId;
    historyLoading = true;
    const oldHeight = scrollElement?.scrollHeight || 0;
    try {
      const page = await api.messages(id, cursor);
      if (activeId === id) {
        messages = [...page.messages.map(recoveredMessage), ...messages];
        more = page.hasMore;
        cursor = page.cursor;
        await tick();
        scrollElement.scrollTop += scrollElement.scrollHeight - oldHeight;
      }
    } catch (e) {
      fail(e);
    } finally {
      historyLoading = false;
    }
  }
  async function send() {
    if (busy || (!draft.trim() && !attachments.length)) return;
    if (!isDesktop || !diagnostic?.installed || !diagnostic.authenticated) {
      settingsTab = 'connection';
      showSettings = true;
      return;
    }
    sending = true;
    error = '';
    followOutput = true;
    const text = draft.trim();
    const files = attachments;
    try {
      let chatId = activeId;
      if (!chatId) {
        const chat = await api.createChat(model, mode, activeProject);
        chats = [chat, ...chats];
        activeId = chatId = chat.id;
        localStorage.setItem('snor.last-chat', chat.id);
      } else if (activeChat && (activeChat.model !== model || activeChat.mode !== mode)) {
        const chat = { ...activeChat, model, mode };
        await api.updateChat(chat);
        chats = chats.map((c) => (c.id === chat.id ? chat : c));
      }
      const result = await api.send(chatId, text, files);
      chats = [result.chat, ...chats.filter((c) => c.id !== result.chat.id)];
      if (activeId === chatId) messages = [...messages, result.user, result.assistant];
      busyChatId = chatId;
      busyMessageId = result.assistant.id;
      draft = '';
      attachments = [];
      localStorage.removeItem('snor.draft.new');
      localStorage.removeItem(draftKey());
      await scrollBottom(true);
    } catch (e) {
      fail(e);
    } finally {
      sending = false;
      const pending = queuedEvents;
      queuedEvents = [];
      for (const event of pending) onAgentEvent(event);
      focusComposer();
    }
  }
  async function stop() {
    if (!busyChatId) return;
    try {
      await api.interrupt(busyChatId);
      notify('Interruzione richiesta a Claude Code.');
    } catch (e) {
      fail(e);
    }
  }
  async function decide(permission: Permission, allow: boolean) {
    if (!activeId || deciding.includes(permission.id)) return;
    deciding = [...deciding, permission.id];
    try {
      await api.permission(activeId, permission.id, allow);
      permissions = permissions.filter((p) => p.id !== permission.id);
    } catch (e) {
      fail(e);
    } finally {
      deciding = deciding.filter((id) => id !== permission.id);
    }
  }
  async function checkConnection() {
    checking = true;
    try {
      diagnostic = await api.diagnostics();
    } catch (e) {
      fail(e);
    } finally {
      checking = false;
    }
  }
  async function login() {
    loggingIn = true;
    try {
      await saveSettings();
      await api.login();
      await checkConnection();
      if (diagnostic?.authenticated) notify('Claude Code collegato.');
    } catch (e) {
      fail(e);
    } finally {
      loggingIn = false;
    }
  }
  async function saveSettings() {
    try {
      await api.saveSettings(settings);
    } catch (e) {
      fail(e);
    }
  }
  async function attach() {
    try {
      const paths = await pickFiles();
      if (paths.length + attachments.length > 4)
        throw new Error('Puoi allegare fino a 4 file per messaggio.');
      const files = await Promise.all(paths.map((path) => api.attachment(path)));
      const next = [...attachments, ...files];
      if (JSON.stringify(next).length > 8 * 1024 * 1024)
        throw new Error('Gli allegati superano il limite complessivo di 8 MiB.');
      attachments = next;
    } catch (e) {
      fail(e);
    }
  }
  async function chooseProjectFolder() {
    try {
      const path = await pickDirectory();
      if (path) {
        projectPath = path;
        if (!projectName) projectName = path.split(/[\\/]/).filter(Boolean).at(-1) || '';
      }
    } catch (e) {
      fail(e);
    }
  }
  async function saveProject() {
    projectSaving = true;
    try {
      const project = await api.saveProject({
        id: projectEditId,
        name: projectName.trim(),
        path: projectPath,
        instructions: projectInstructions,
      });
      projects = [...projects.filter((p) => p.id !== project.id), project];
      showProject = false;
      activeProject = project.id;
      newChat();
      notify('Progetto salvato.');
    } catch (e) {
      fail(e);
    } finally {
      projectSaving = false;
    }
  }
  function projectDialog(project?: Project) {
    projectEditId = project?.id;
    projectName = project?.name || '';
    projectPath = project?.path || '';
    projectInstructions = project?.instructions || '';
    showProject = true;
  }
  async function changeChat(patch: Partial<Chat>) {
    if (!activeChat) return;
    try {
      const chat = { ...activeChat, ...patch };
      await api.updateChat(chat);
      chats = chats.map((c) => (c.id === chat.id ? chat : c));
      menu = false;
    } catch (e) {
      fail(e);
    }
  }
  async function removeChat() {
    if (!activeId) return;
    try {
      await api.deleteChat(activeId);
      storeDraft(draftKey(), '');
      draft = '';
      chats = chats.filter((c) => c.id !== activeId);
      showDelete = false;
      newChat();
    } catch (e) {
      fail(e);
    }
  }
  async function exportChat() {
    if (!activeChat) return;
    try {
      if (!isDesktop) throw new Error('Esporta dalla versione desktop.');
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        defaultPath: `${activeChat.title.replace(/[<>:"/\\|?*]/g, '').slice(0, 60)}.md`,
        filters: [
          { name: 'Markdown', extensions: ['md'] },
          { name: 'JSON', extensions: ['json'] },
        ],
      });
      if (path) {
        await api.export(activeChat.id, path, path.endsWith('.json') ? 'json' : 'markdown');
        notify('Conversazione esportata.');
      }
    } catch (e) {
      fail(e);
    }
    menu = false;
  }
  function usePrompt(text: string) {
    draft = text;
    focusComposer();
  }
  function keyComposer(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void send();
    }
  }
</script>

<svelte:head
  ><title>{activeChat ? `${activeChat.title} · Snor` : 'Snor · Spazio alle idee'}</title
  ></svelte:head
>

<div class="app-shell" class:sidebar-hidden={!sidebar}>
  {#if sidebar}
    <aside class="sidebar">
      <div class="brand-row">
        <button
          class="brand"
          on:click={() => {
            activeProject = null;
            newChat();
          }}
          aria-label="Snor, pagina iniziale"
          ><span class="brand-mark">s<span>•</span></span>snor<span class="brand-period">.</span
          ></button
        ><button
          class="icon-button subtle"
          aria-label="Nascondi barra laterale"
          on:click={() => (sidebar = false)}><Icon name="panel" size={17} /></button
        >
      </div>
      <button class="new-chat-button" on:click={newChat}
        ><Icon name="plus" size={17} /><span>Nuova conversazione</span><kbd>Ctrl N</kbd></button
      >
      <nav class="main-nav" aria-label="Navigazione principale">
        <button
          on:click={() => {
            search = '';
            showSearch = true;
          }}><Icon name="search" /><span>Cerca ovunque</span><kbd>Ctrl K</kbd></button
        >
        <button
          class:active={view === 'projects'}
          on:click={() => {
            view = 'projects';
            activeProject = null;
          }}
          ><Icon name="folder" /><span>Progetti</span>{#if projects.length}<span class="count"
              >{projects.length}</span
            >{/if}</button
        >
      </nav>
      <div class="sidebar-scroll">
        {#if projects.length}<div class="nav-section">
            <div class="section-label">
              I TUOI SPAZI<button
                class="icon-button"
                aria-label="Aggiungi progetto"
                on:click={() => projectDialog()}><Icon name="plus" size={13} /></button
              >
            </div>
            {#each projects as project}<button
                class="project-nav"
                class:active={activeProject === project.id}
                on:click={() => {
                  activeProject = project.id;
                  newChat();
                }}><span class="project-dot"></span><span>{project.name}</span></button
              >{/each}
          </div>{/if}
        {#if pinned.length}<div class="nav-section">
            <div class="section-label">IN EVIDENZA</div>
            {#each pinned as chat}<button
                class="chat-nav"
                class:active={activeId === chat.id}
                on:click={() => openChat(chat.id)}
                ><Icon name="pin" size={13} /><span>{chat.title}</span></button
              >{/each}
          </div>{/if}
        <div class="nav-section">
          <div class="section-label">
            {activeProject ? 'NEL PROGETTO' : 'CONVERSAZIONI'}<span
              >{visibleChats.length || ''}</span
            >
          </div>
          {#if !booted}<div class="sidebar-skeleton"></div>
            <div class="sidebar-skeleton short"></div>
          {:else if recent.length}{#each recent as chat}<button
                class="chat-nav"
                class:active={activeId === chat.id}
                on:click={() => openChat(chat.id)}
                ><span>{chat.title}</span>{#if busyChatId === chat.id}<span class="live-dot"
                  ></span>{/if}</button
              >{/each}
          {:else}<div class="sidebar-empty">
              <Icon name="chat" size={22} />
              <p>Ogni idea inizia<br />da una conversazione.</p>
              <span>Le ritroverai tutte qui.</span>
            </div>{/if}
        </div>
      </div>
      <div class="sidebar-bottom">
        <button
          class="archive-button"
          on:click={() => {
            view = 'archive';
            activeProject = null;
          }}><Icon name="archive" size={16} />Archivio</button
        >
        <div class="connection-card">
          <div class="connection-symbol"><Icon name="spark" size={18} /></div>
          <div>
            <strong>Claude Code</strong><span
              ><i class:connected={diagnostic?.authenticated}></i>{checking
                ? 'Verifica connessione…'
                : diagnostic?.authenticated
                  ? 'Abbonamento collegato'
                  : 'Da collegare'}</span
            >
          </div>
          <button
            class="icon-button"
            aria-label="Configura Claude Code"
            on:click={() => {
              settingsTab = 'connection';
              showSettings = true;
            }}><Icon name="chevron" size={15} /></button
          >
        </div>
        <button
          class="profile-button"
          on:click={() => {
            settingsTab = 'general';
            showSettings = true;
          }}
          ><span class="profile-avatar">S</span>
          <div><strong>Il tuo spazio</strong><span>Locale, personale</span></div>
          <Icon name="settings" size={17} /></button
        >
      </div>
    </aside>
  {/if}
  <main class="main-panel">
    <header class="topbar">
      <div class="breadcrumb">
        {#if !sidebar}<button
            class="icon-button"
            aria-label="Mostra barra laterale"
            on:click={() => (sidebar = true)}><Icon name="panel" /></button
          >{/if}<Icon name={selectedProject ? 'folder' : 'leaf'} size={15} /><button
          on:click={() => {
            activeProject = null;
            newChat();
          }}>{selectedProject?.name || 'Spazio personale'}</button
        ><span class="breadcrumb-slash">/</span><span class="breadcrumb-current"
          >{view === 'projects'
            ? 'Progetti'
            : view === 'archive'
              ? 'Archivio'
              : activeChat?.title || 'Nuova conversazione'}</span
        >
      </div>
      <div class="topbar-actions">
        <span class="local-badge"
          ><span></span>{isDesktop ? 'Sul tuo computer' : 'Anteprima UI'}</span
        >{#if activeChat && view === 'chat'}<div class="menu-container">
            <button
              class="icon-button"
              aria-label="Opzioni conversazione"
              aria-expanded={menu}
              on:click={() => (menu = !menu)}><Icon name="more" /></button
            >{#if menu}<div class="dropdown chat-menu">
                <button
                  on:click={() => {
                    renameValue = activeChat!.title;
                    showRename = true;
                    menu = false;
                  }}><Icon name="edit" size={15} />Rinomina</button
                ><button on:click={() => changeChat({ pinned: !activeChat!.pinned })}
                  ><Icon name="pin" size={15} />{activeChat.pinned
                    ? 'Rimuovi dai preferiti'
                    : 'Metti in evidenza'}</button
                ><button on:click={exportChat}
                  ><Icon name="download" size={15} />Esporta conversazione</button
                ><button
                  on:click={async () => {
                    await changeChat({ archived: !activeChat!.archived });
                    newChat();
                  }}
                  ><Icon name="archive" size={15} />{activeChat.archived
                    ? 'Ripristina'
                    : 'Archivia'}</button
                >
                <div class="menu-divider"></div>
                <button
                  class="danger-text"
                  on:click={() => {
                    showDelete = true;
                    menu = false;
                  }}><Icon name="trash" size={15} />Elimina</button
                >
              </div>{/if}
          </div>{/if}
      </div>
    </header>

    {#if view === 'projects'}
      <div class="collection-page">
        <div class="collection-heading">
          <div>
            <div class="eyebrow">CONTESTO CHE RESTA</div>
            <h1>I tuoi progetti<span>.</span></h1>
            <p>Uno spazio per ogni cosa a cui tieni.</p>
          </div>
          <button class="primary-button" on:click={() => projectDialog()}
            ><Icon name="plus" size={17} />Nuovo progetto</button
          >
        </div>
        {#if projects.length}<div class="project-grid">
            {#each projects as project}<article class="project-card">
                <div class="project-card-head">
                  <div class="project-icon"><Icon name="folder" size={23} /></div>
                  <button
                    class="icon-button"
                    aria-label={`Modifica ${project.name}`}
                    on:click={() => projectDialog(project)}><Icon name="edit" size={16} /></button
                  >
                </div>
                <button
                  class="project-card-name"
                  on:click={() => {
                    activeProject = project.id;
                    newChat();
                  }}>{project.name}<Icon name="chevron" size={18} /></button
                >
                <p>
                  {project.instructions ||
                    'Aggiungi istruzioni per dare a Claude il contesto giusto.'}
                </p>
                <span class="project-path">{project.path}</span>
                <div class="project-card-footer">
                  {chats.filter((c) => c.projectId === project.id).length} conversazioni
                </div>
              </article>{/each}
          </div>{:else}<div class="collection-empty">
            <Icon name="folder" size={42} />
            <h2>Le idee grandi hanno bisogno di spazio.</h2>
            <p>
              Collega una cartella e aggiungi le tue istruzioni.<br />Ogni conversazione partirà dal
              contesto giusto.
            </p>
            <button class="secondary-button" on:click={() => projectDialog()}
              >Crea il primo progetto<Icon name="plus" size={16} /></button
            >
          </div>{/if}
      </div>
    {:else if view === 'archive'}
      <div class="collection-page">
        <div class="eyebrow">TUTTO AL SUO POSTO</div>
        <h1>Archivio<span>.</span></h1>
        <p class="muted">Le conversazioni che vuoi conservare, senza averle sempre davanti.</p>
        <div class="archive-list">
          {#each chats.filter((c) => c.archived) as chat}<button on:click={() => openChat(chat.id)}
              ><Icon name="chat" /><span>{chat.title}</span><span class="muted small"
                >{new Date(chat.updatedAt).toLocaleDateString('it-IT')}</span
              ><Icon name="chevron" size={16} /></button
            >{:else}<div class="collection-empty">
              <Icon name="archive" size={38} />
              <h2>Qui c’è ancora spazio.</h2>
              <p>Le conversazioni archiviate appariranno qui.</p>
            </div>{/each}
        </div>
      </div>
    {:else}
      <div
        class="chat-area"
        class:has-messages={messages.length > 0}
        bind:this={scrollElement}
        on:scroll={onScroll}
        role="log"
        aria-label="Conversazione"
        aria-live="off"
      >
        {#if !messages.length}
          <section class="welcome">
            <div class="welcome-meta">
              <span class="tiny-line"></span> MENO RUMORE, PIÙ POSSIBILITÀ
              <span class="tiny-line"></span>
            </div>
            <div class="welcome-symbol" aria-hidden="true">
              <svg viewBox="0 0 100 100"
                ><g stroke="currentColor" stroke-width="5.5" stroke-linecap="round"
                  ><path
                    d="M50 12v20m0 36v20M12 50h20m36 0h20M23 23l15 15m24 24 15 15M23 77l15-15m24-24 15-15M35 15l8 19m14 32 8 19M15 35l19 8m32 14 19 8M15 65l19-8m32-14 19-8M35 85l8-19m14-32 8-19"
                  /></g
                ></svg
              >
            </div>
            <h1>Un pensiero.<br /><em>Infinite possibilità.</em></h1>
            <p class="welcome-description">
              Un posto tranquillo per fare domande, mettere a fuoco<br class="desktop-break" /> le idee
              e dare forma a quello che hai in mente.
            </p>
            {#if selectedProject}<div class="project-context">
                <Icon name="folder" size={15} />{selectedProject.name}<span
                  >Contesto del progetto attivo</span
                >
              </div>{/if}
            <div class="suggestions">
              <button on:click={() => usePrompt('Aiutami a mettere ordine in questa idea: ')}
                ><span class="suggestion-icon peach"><Icon name="spark" size={19} /></span><strong
                  >Metti a fuoco un’idea</strong
                ><span>Da dove iniziamo?</span><Icon name="chevron" size={15} /></button
              ><button on:click={() => usePrompt('Aiutami a scrivere un testo su ')}
                ><span class="suggestion-icon sage"><Icon name="edit" size={19} /></span><strong
                  >Trova le parole giuste</strong
                ><span>Scriviamo qualcosa insieme</span><Icon name="chevron" size={15} /></button
              ><button
                on:click={() => {
                  mode = 'code';
                  usePrompt('Vorrei costruire ');
                }}
                ><span class="suggestion-icon lavender"><Icon name="code" size={19} /></span><strong
                  >Costruisci qualcosa</strong
                ><span>Una riga alla volta</span><Icon name="chevron" size={15} /></button
              >
            </div>
          </section>
        {:else}<div class="messages-wrap">
            {#if more}<button class="load-older" disabled={historyLoading} on:click={loadOlder}
                ><Icon name="clock" size={14} />{historyLoading
                  ? 'Caricamento…'
                  : 'Carica messaggi precedenti'}</button
              >{/if}{#each messages as message (message.id)}<MessageCard
                {message}
                onReuse={usePrompt}
              />{/each}
          </div>{/if}
      </div>
      <div class="composer-zone">
        {#if error}<div class="error-banner" role="alert">
            <Icon name="shield" size={17} /><span>{error}</span><button
              class="icon-button"
              aria-label="Chiudi errore"
              on:click={() => (error = '')}><Icon name="close" size={15} /></button
            >
          </div>{/if}
        {#each permissions as permission}<div class="permission-card">
            <div class="permission-heading">
              <Icon name="shield" size={18} /><strong>Claude chiede il tuo permesso</strong><span
                >{permission.tool}</span
              >
            </div>
            <pre>{JSON.stringify(permission.input, null, 2).slice(0, 8000)}</pre>
            <div class="permission-actions">
              <button
                class="secondary-button"
                disabled={deciding.includes(permission.id)}
                on:click={() => decide(permission, false)}>Rifiuta</button
              ><button
                class="primary-button"
                disabled={deciding.includes(permission.id)}
                on:click={() => decide(permission, true)}>Consenti questa volta</button
              >
            </div>
          </div>{/each}
        {#if busyChatId && busyChatId !== activeId}<button
            class="busy-notice"
            on:click={() => openChat(busyChatId!)}
            ><span class="live-dot"></span>Claude sta lavorando in un’altra conversazione<Icon
              name="chevron"
              size={14}
            /></button
          >{/if}
        <div class="composer" class:composer-busy={busy}>
          {#if attachments.length}<div class="attachment-strip">
              {#each attachments as attachment}<span class="attachment-chip"
                  ><Icon name={attachment.kind === 'image' ? 'image' : 'file'} size={16} /><span
                    >{attachment.name}</span
                  ><button
                    class="icon-button"
                    aria-label={`Rimuovi ${attachment.name}`}
                    on:click={() =>
                      (attachments = attachments.filter((a) => a.id !== attachment.id))}
                    ><Icon name="close" size={12} /></button
                  ></span
                >{/each}
            </div>{/if}
          <textarea
            bind:this={composer}
            bind:value={draft}
            on:keydown={keyComposer}
            placeholder={mode === 'code'
              ? 'Cosa costruiamo oggi?'
              : mode === 'task'
                ? 'Descrivi quello che vuoi fare…'
                : 'Da quale pensiero partiamo?'}
            aria-label="Messaggio per Claude"
            rows="2"
            maxlength="131072"
            disabled={sending}></textarea>
          <div class="composer-toolbar">
            <div class="composer-tools">
              <button
                class="icon-button attach-button"
                aria-label="Allega immagini o file di testo"
                title="Allega immagini o file di testo"
                disabled={busy}
                on:click={attach}><Icon name="plus" size={21} /></button
              ><span class="toolbar-divider"></span><label class="mode-select"
                ><Icon
                  name={mode === 'chat' ? 'chat' : mode === 'code' ? 'code' : 'bolt'}
                  size={15}
                /><select bind:value={mode} disabled={busy} aria-label="Modalità di lavoro"
                  >{#each Object.entries(modeLabels) as [value, label]}<option {value}
                      >{label}</option
                    >{/each}</select
                ><Icon name="down" size={12} /></label
              >
            </div>
            <div class="composer-right">
              <label class="model-select"
                ><select bind:value={model} disabled={busy} aria-label="Modello Claude"
                  >{#each Object.entries(modelLabels) as [value, label]}<option {value}
                      >{label}</option
                    >{/each}</select
                ><Icon name="down" size={12} /></label
              >{#if busyChatId === activeId && busyChatId}<button
                  class="send-button stop-button"
                  aria-label="Interrompi risposta"
                  on:click={stop}><Icon name="stop" size={17} /></button
                >{:else}<button
                  class="send-button"
                  disabled={busy || (!draft.trim() && !attachments.length)}
                  aria-label="Invia messaggio"
                  on:click={send}><Icon name="arrow" size={19} /></button
                >{/if}
            </div>
          </div>
        </div>
        <div class="composer-caption">
          <span
            ><Icon name="shield" size={12} />{mode === 'chat'
              ? 'Conversazione senza strumenti'
              : 'Le azioni richiedono i permessi di Claude Code'}</span
          ><span
            >Invio per inviare <span class="caption-separator">·</span> Shift + Invio per andare a capo</span
          >
        </div>
      </div>
      <footer class="workspace-footer">
        <span>Fatto per pensare meglio.</span><span
          >{isDesktop
            ? 'Claude Code, il tuo abbonamento.'
            : 'Anteprima dell’interfaccia · il motore è disponibile nell’app desktop.'}<span
            class="footer-dot"
          ></span><span class="version-label">v0.1</span></span
        >
      </footer>
    {/if}
  </main>
</div>

{#if toast}<div class="toast" role="status"><Icon name="check" size={16} />{toast}</div>{/if}

{#if showSettings}<Modal
    title="Il tuo spazio, a modo tuo."
    subtitle="Poche impostazioni. Tutto sotto controllo."
    onClose={() => (showSettings = false)}
    ><div class="settings-tabs" role="tablist" aria-label="Impostazioni">
      <button
        role="tab"
        aria-selected={settingsTab === 'general'}
        class:active={settingsTab === 'general'}
        on:click={() => (settingsTab = 'general')}>Aspetto</button
      ><button
        role="tab"
        aria-selected={settingsTab === 'connection'}
        class:active={settingsTab === 'connection'}
        on:click={() => (settingsTab = 'connection')}>Claude Code</button
      ><button
        role="tab"
        aria-selected={settingsTab === 'about'}
        class:active={settingsTab === 'about'}
        on:click={() => (settingsTab = 'about')}>Informazioni</button
      >
    </div>
    <div class="modal-content">
      {#if settingsTab === 'general'}<div class="setting-group">
          <label for="theme">Aspetto</label>
          <p>Trova la luce giusta per le tue idee.</p>
          <div class="theme-options">
            {#each [['light', 'sun', 'Chiaro'], ['dark', 'moon', 'Scuro'], ['system', 'panel', 'Sistema']] as [value, icon, label]}<button
                class:chosen={settings.theme === value}
                on:click={() => {
                  settings = { ...settings, theme: value as Settings['theme'] };
                  void saveSettings();
                }}><Icon name={icon} size={22} />{label}</button
              >{/each}
          </div>
        </div>
        <div class="setting-row">
          <div>
            <label for="default-model">Modello predefinito</label>
            <p>Per le nuove conversazioni.</p>
          </div>
          <select id="default-model" bind:value={settings.model} on:change={saveSettings}
            >{#each Object.entries(modelLabels) as [value, label]}<option {value}>{label}</option
              >{/each}</select
          >
        </div>
        <div class="setting-row">
          <div>
            <label for="compact">Spaziatura compatta</label>
            <p>Più contenuto nello stesso spazio.</p>
          </div>
          <input
            id="compact"
            class="toggle"
            type="checkbox"
            bind:checked={settings.compact}
            on:change={saveSettings}
          />
        </div>
        <div class="note-box">
          <Icon name="shield" size={18} />
          <p>
            Cronologia e progetti sono salvati sul computer. I messaggi inviati vengono elaborati
            dai servizi Anthropic tramite Claude Code.
          </p>
        </div>
      {:else if settingsTab === 'connection'}<div
          class="connection-status"
          class:success={diagnostic?.authenticated}
        >
          <div class="connection-symbol"><Icon name="spark" size={24} /></div>
          <div>
            <strong
              >{diagnostic?.authenticated
                ? 'Pronto a pensare insieme.'
                : diagnostic?.installed
                  ? 'Manca solo l’accesso.'
                  : 'Colleghiamo Claude Code.'}</strong
            >
            <p>
              {diagnostic?.authenticated
                ? diagnostic.version
                : diagnostic?.error || 'Verifica della connessione in corso…'}
            </p>
          </div>
        </div>
        <div class="setting-group">
          <label for="claude-path"
            >Percorso del binario <span class="muted">(facoltativo)</span></label
          >
          <p>Snor cerca il binario nativo nel PATH e nelle cartelle standard.</p>
          <input
            id="claude-path"
            bind:value={settings.claudePath}
            placeholder="C:\Users\nome\.local\bin\claude.exe"
            spellcheck="false"
          />
          <div class="connection-buttons">
            <button
              class="secondary-button"
              disabled={checking}
              on:click={async () => {
                await saveSettings();
                await checkConnection();
              }}
              ><Icon name="refresh" size={15} />{checking
                ? 'Verifica…'
                : 'Salva e verifica'}</button
            ><button class="primary-button" disabled={!isDesktop || loggingIn} on:click={login}
              ><Icon name="external" size={15} />{loggingIn
                ? 'Completa l’accesso nel browser…'
                : 'Accedi con Claude'}</button
            >
          </div>
        </div>
        <div class="setup-steps">
          <h3>Prima volta?</h3>
          <p>1. Installa Claude Code dal sito ufficiale Anthropic.</p>
          <p>2. Esegui <code>claude auth login</code> e scegli il tuo abbonamento.</p>
          <p>3. Torna qui e premi «Salva e verifica».</p>
        </div>
        <div class="note-box">
          <Icon name="bolt" size={17} />
          <p>
            Nessuna chiave API richiesta. Snor comunica direttamente con il processo Claude Code.
            Disponibilità dei modelli e limiti dipendono dal tuo piano.
          </p>
        </div>
      {:else}<div class="about-brand">snor<span>.</span></div>
        <p class="about-tagline">Uno spazio tranquillo per pensare e creare.</p>
        <div class="about-grid">
          <div><span>VERSIONE</span><strong>0.1.0 · Prima implementazione</strong></div>
          <div><span>MOTORE</span><strong>Claude Code · collegamento diretto</strong></div>
          <div><span>ARCHITETTURA</span><strong>Tauri 2 · Rust · Svelte 5 · SQLite</strong></div>
          <div><span>PRESTAZIONI</span><strong>Obiettivi da verificare in release</strong></div>
        </div>
        <div class="note-box">
          <Icon name="book" size={18} />
          <p>
            La documentazione completa è nella cartella <code>docs/</code> del progetto: sviluppo, architettura,
            protocollo, prestazioni e roadmap. Snor è un progetto indipendente.
          </p>
        </div>{/if}
      {#if error}<div class="inline-error" role="alert">{error}</div>{/if}
    </div></Modal
  >{/if}

{#if showSearch}<Modal
    title="Ritrova un pensiero."
    subtitle="Cerca nei titoli e nel testo delle conversazioni locali."
    onClose={() => (showSearch = false)}
    ><div class="modal-content">
      <div class="search-input">
        <Icon name="search" size={20} /><input
          bind:value={search}
          aria-label="Cerca conversazioni"
          placeholder="Una parola, un’idea, un progetto…"
        />
      </div>
      <div class="search-results">
        {#each searchResults as chat}<button
            on:click={() => {
              showSearch = false;
              void openChat(chat.id);
            }}
            ><Icon name="chat" size={18} />
            <div>
              <strong>{chat.title}</strong><span
                >{new Date(chat.updatedAt).toLocaleDateString('it-IT')} · {modeLabels[
                  chat.mode
                ]}</span
              >
            </div>
            <Icon name="chevron" size={16} /></button
          >{:else}<div class="search-empty">
            {search ? 'Nessuna conversazione trovata.' : 'Le tue conversazioni appariranno qui.'}
          </div>{/each}
      </div>
    </div></Modal
  >{/if}

{#if showProject}<Modal
    title={projectEditId ? 'Diamo più contesto.' : 'Un nuovo spazio.'}
    subtitle="Cartella, istruzioni e conversazioni, insieme."
    onClose={() => (showProject = false)}
    ><form class="modal-content form-stack" on:submit|preventDefault={saveProject}>
      <label
        >Nome del progetto<input
          bind:value={projectName}
          required
          maxlength="100"
          placeholder="La prossima grande idea"
        /></label
      ><label
        >Cartella di lavoro
        <div class="path-input">
          <input bind:value={projectPath} required placeholder="Seleziona una cartella…" /><button
            class="secondary-button"
            type="button"
            on:click={chooseProjectFolder}><Icon name="folder" size={17} />Sfoglia</button
          >
        </div></label
      ><label
        >Istruzioni per Claude <span class="muted">(facoltative)</span><textarea
          bind:value={projectInstructions}
          maxlength="32000"
          rows="5"
          placeholder="Di cosa tratta il progetto? Come preferisci lavorare?"></textarea></label
      >
      <p class="small muted">
        Le modalità Attività e Codice operano nella cartella scelta secondo i permessi di Claude
        Code. La cartella di lavoro non è un isolamento di sicurezza.
      </p>
      {#if error}<div class="inline-error" role="alert">{error}</div>{/if}
      <div class="modal-actions">
        <button class="secondary-button" type="button" on:click={() => (showProject = false)}
          >Annulla</button
        ><button class="primary-button" type="submit" disabled={projectSaving}
          >{projectSaving ? 'Salvataggio…' : 'Salva progetto'}<Icon
            name="arrow"
            size={16}
          /></button
        >
      </div>
    </form></Modal
  >{/if}
{#if showRename}<Modal title="Un nome che ritrovi." onClose={() => (showRename = false)}
    ><form
      class="modal-content form-stack"
      on:submit|preventDefault={async () => {
        await changeChat({ title: renameValue.trim() });
        showRename = false;
      }}
    >
      <label
        >Titolo della conversazione<input
          bind:value={renameValue}
          required
          maxlength="200"
        /></label
      >
      <div class="modal-actions">
        <button class="primary-button" type="submit">Salva titolo</button>
      </div>
    </form></Modal
  >{/if}
{#if showDelete}<Modal
    title="Eliminare questa conversazione?"
    subtitle="La cronologia locale di Snor verrà eliminata. La sessione originale di Claude Code rimane sul computer."
    onClose={() => (showDelete = false)}
    ><div class="modal-content">
      <div class="modal-actions">
        <button class="secondary-button" on:click={() => (showDelete = false)}>Conserva</button
        ><button class="danger-button" disabled={busyChatId === activeId} on:click={removeChat}
          >Elimina conversazione</button
        >
      </div>
    </div></Modal
  >{/if}
