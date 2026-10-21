export type Mode = 'chat' | 'task' | 'code';
export type Model = 'sonnet' | 'opus' | 'haiku';
export interface Chat {
  id: string;
  title: string;
  model: Model;
  mode: Mode;
  projectId: string | null;
  cwd: string | null;
  sessionId: string | null;
  pinned: boolean;
  archived: boolean;
  createdAt: number;
  updatedAt: number;
}
export interface Project {
  id: string;
  name: string;
  path: string;
  instructions: string;
}
export interface Attachment {
  id: string;
  name: string;
  kind: 'image' | 'text';
  mime: string;
  data?: string;
  size: number;
}
export interface Tool {
  id: string;
  name: string;
  input: Record<string, unknown>;
  status: 'running' | 'complete' | 'error';
  output?: string;
}
export interface Message {
  id: string;
  role: 'user' | 'assistant';
  text: string;
  thinking?: string;
  status: 'complete' | 'streaming' | 'interrupted' | 'error';
  createdAt: number;
  attachments?: Attachment[];
  tools?: Tool[];
  error?: string;
  usage?: { input_tokens?: number; output_tokens?: number; cache_read_input_tokens?: number };
}
export interface Settings {
  theme: 'system' | 'light' | 'dark';
  claudePath: string;
  model: Model;
  compact: boolean;
}
export interface Diagnostic {
  installed: boolean;
  authenticated: boolean;
  path?: string;
  version?: string;
  error?: string;
  authMethod?: string;
}
export interface Permission {
  id: string;
  tool: string;
  input: Record<string, unknown>;
}
export interface AgentEvent {
  chatId: string;
  messageId: string;
  kind: string;
  data: any;
}
export interface MessagePage {
  messages: Message[];
  cursor: number | null;
  hasMore: boolean;
}
export const defaultSettings: Settings = {
  theme: 'system',
  claudePath: '',
  model: 'sonnet',
  compact: false,
};
export const modeLabels: Record<Mode, string> = {
  chat: 'Conversazione',
  task: 'Attività',
  code: 'Codice',
};
export const modelLabels: Record<Model, string> = {
  sonnet: 'Sonnet',
  opus: 'Opus',
  haiku: 'Haiku',
};
