import type { AgentEvent, Message } from './types';

export function reduceMessage(message: Message, event: AgentEvent): Message {
  if (event.messageId !== message.id) return message;
  switch (event.kind) {
    case 'delta':
      return { ...message, text: message.text + String(event.data) };
    case 'thinking':
      return { ...message, thinking: (message.thinking || '') + String(event.data) };
    case 'tool':
      return {
        ...message,
        tools: [
          ...(message.tools || []).filter((t) => t.id !== event.data.id).slice(-49),
          { ...event.data, status: 'running' },
        ],
      };
    case 'tool_result':
      return {
        ...message,
        tools: message.tools?.map((t) =>
          t.id === event.data.tool_use_id
            ? {
                ...t,
                status: event.data.is_error ? 'error' : 'complete',
                output:
                  typeof event.data.content === 'string'
                    ? event.data.content
                    : JSON.stringify(event.data.content),
              }
            : t,
        ),
      };
    case 'done':
      return { ...message, ...event.data, tools: event.data.tools || message.tools };
    case 'failure':
      return { ...message, status: 'error', error: String(event.data) };
    default:
      return message;
  }
}

export function recoveredMessage(message: Message): Message {
  return message.status === 'streaming'
    ? {
        ...message,
        status: 'interrupted',
        error:
          'La sessione precedente si è interrotta. Puoi inviare un nuovo messaggio per riprenderla.',
      }
    : message;
}
