import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  Chat,
  Project,
  Settings,
  MessagePage,
  Diagnostic,
  Attachment,
  Message,
  Mode,
  Model,
  AgentEvent,
} from './types';

export const isDesktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
type Data = {
  chats: Chat[];
  projects: Project[];
  settings: Partial<Settings>;
  messages: Record<string, Message[]>;
};
const key = 'snor.ui-preview.v1';
function local(): Data {
  try {
    return {
      chats: [],
      projects: [],
      settings: {},
      messages: {},
      ...JSON.parse(localStorage.getItem(key) || '{}'),
    };
  } catch {
    return { chats: [], projects: [], settings: {}, messages: {} };
  }
}
function persist(data: Data) {
  localStorage.setItem(key, JSON.stringify(data));
}

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isDesktop) return invoke<T>(command, args);
  const data = local();
  let result: unknown;
  switch (command) {
    case 'bootstrap':
      result = data;
      break;
    case 'list_chats':
      result = data.chats.filter(
        (c) =>
          !args.search ||
          c.title.toLowerCase().includes(String(args.search).toLowerCase()) ||
          (data.messages[c.id] || []).some((m) =>
            m.text.toLowerCase().includes(String(args.search).toLowerCase()),
          ),
      );
      break;
    case 'create_chat': {
      const p = data.projects.find((p) => p.id === args.projectId);
      const chat: Chat = {
        id: crypto.randomUUID(),
        title: 'Nuova conversazione',
        model: args.model as Model,
        mode: args.mode as Mode,
        projectId: p?.id || null,
        cwd: p?.path || null,
        sessionId: null,
        pinned: false,
        archived: false,
        createdAt: Date.now(),
        updatedAt: Date.now(),
      };
      data.chats.unshift(chat);
      result = chat;
      break;
    }
    case 'get_messages':
      result = { messages: data.messages[String(args.chatId)] || [], cursor: null, hasMore: false };
      break;
    case 'update_chat':
      data.chats = data.chats.map((c) =>
        c.id === (args.chat as Chat).id ? (args.chat as Chat) : c,
      );
      break;
    case 'delete_chat':
      data.chats = data.chats.filter((c) => c.id !== args.chatId);
      delete data.messages[String(args.chatId)];
      break;
    case 'save_settings':
      data.settings = args.settings as Settings;
      break;
    case 'save_project': {
      const p = {
        ...(args.project as Project),
        id: (args.project as Project).id || crypto.randomUUID(),
      };
      data.projects = [...data.projects.filter((x) => x.id !== p.id), p];
      result = p;
      break;
    }
    case 'diagnostics':
      result = {
        installed: false,
        authenticated: false,
        error: 'Anteprima nel browser. Avvia l’app Tauri per collegare Claude Code.',
      };
      break;
    case 'frontend_ready':
      result = null;
      break;
    case 'open_external': {
      const url = new URL(String(args.url));
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password)
        throw new Error('Indirizzo non consentito.');
      window.open(url.href, '_blank', 'noopener,noreferrer');
      break;
    }
    default:
      throw new Error(
        'Questa funzione richiede l’app desktop. L’anteprima non esegue Claude Code.',
      );
  }
  if (!['bootstrap', 'get_messages', 'list_chats', 'diagnostics'].includes(command)) persist(data);
  return result as T;
}

export const api = {
  ready: () => call<unknown>('frontend_ready'),
  bootstrap: () =>
    call<{ chats: Chat[]; projects: Project[]; settings: Partial<Settings> }>('bootstrap'),
  search: (search: string) => call<Chat[]>('list_chats', { search }),
  createChat: (model: Model, mode: Mode, projectId: string | null) =>
    call<Chat>('create_chat', { model, mode, projectId }),
  updateChat: (chat: Chat) => call<void>('update_chat', { chat }),
  messages: (chatId: string, before: number | null = null) =>
    call<MessagePage>('get_messages', { chatId, before }),
  deleteChat: (chatId: string) => call<void>('delete_chat', { chatId }),
  saveSettings: (settings: Settings) => call<void>('save_settings', { settings }),
  saveProject: (project: Omit<Project, 'id'> & { id?: string }) =>
    call<Project>('save_project', { project }),
  diagnostics: () => call<Diagnostic>('diagnostics'),
  login: () => call<void>('login'),
  send: (chatId: string, text: string, attachments: Attachment[]) =>
    call<{ chat: Chat; user: Message; assistant: Message }>('send_prompt', {
      chatId,
      text,
      attachments,
    }),
  interrupt: (chatId: string) => call<void>('interrupt', { chatId }),
  permission: (chatId: string, requestId: string, allow: boolean) =>
    call<void>('respond_permission', { chatId, requestId, allow }),
  attachment: (path: string) => call<Attachment>('read_attachment', { path }),
  export: (chatId: string, path: string, format: string) =>
    call<void>('export_chat', { chatId, path, format }),
  openLink: (url: string) => call<void>('open_external', { url }),
};
export async function subscribe(handler: (event: AgentEvent) => void): Promise<() => void> {
  if (!isDesktop) return () => {};
  return listen<AgentEvent>('agent:event', (e) => handler(e.payload));
}
export async function pickDirectory(): Promise<string | null> {
  if (!isDesktop) throw new Error('La scelta di cartelle richiede l’app desktop.');
  const { open } = await import('@tauri-apps/plugin-dialog');
  return open({ directory: true, multiple: false }) as Promise<string | null>;
}
export async function pickFiles(): Promise<string[]> {
  if (!isDesktop) throw new Error('Gli allegati richiedono l’app desktop.');
  const { open } = await import('@tauri-apps/plugin-dialog');
  const result = await open({
    multiple: true,
    filters: [
      {
        name: 'Immagini e testo',
        extensions: [
          'png',
          'jpg',
          'jpeg',
          'gif',
          'webp',
          'txt',
          'md',
          'csv',
          'json',
          'ts',
          'js',
          'py',
          'rs',
          'html',
          'css',
          'xml',
          'log',
          'yaml',
          'yml',
        ],
      },
    ],
  });
  return result ? (Array.isArray(result) ? result : [result]) : [];
}
