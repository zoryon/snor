import { describe, expect, it } from 'vitest';
import { recoveredMessage, reduceMessage } from './events';
import type { Message, AgentEvent } from './types';
const message: Message = {
  id: 'm',
  role: 'assistant',
  text: '',
  status: 'streaming',
  createdAt: 1,
};
const event = (kind: string, data: unknown, messageId = 'm'): AgentEvent => ({
  kind,
  data,
  messageId,
  chatId: 'c',
});
describe('Claude event reconciliation', () => {
  it('ignores events for other turns and preserves ordered streaming', () => {
    expect(reduceMessage(message, event('delta', 'leak', 'other'))).toBe(message);
    expect(
      reduceMessage(reduceMessage(message, event('delta', 'Ciao')), event('delta', ' mondo')).text,
    ).toBe('Ciao mondo');
  });
  it('final snapshots replace partial text and preserve tool progress', () => {
    let m = reduceMessage(message, event('tool', { id: 't', name: 'Read', input: {} }));
    m = reduceMessage(m, event('tool_result', { tool_use_id: 't', content: 'ok' }));
    m = reduceMessage(m, event('done', { text: 'Finito', status: 'complete' }));
    expect(m.text).toBe('Finito');
    expect(m.tools?.[0].status).toBe('complete');
  });
  it('does not present restored unfinished messages as active generation', () => {
    expect(recoveredMessage(message).status).toBe('interrupted');
    expect(recoveredMessage({ ...message, status: 'complete' }).status).toBe('complete');
  });
  it('bounds tool history and reports failure without losing partial text', () => {
    let m = { ...message, text: 'Parziale' };
    for (let i = 0; i < 80; i++)
      m = reduceMessage(m, event('tool', { id: String(i), name: 'Read', input: {} }));
    expect(m.tools?.length).toBe(50);
    expect(reduceMessage(m, event('failure', 'Errore')).text).toBe('Parziale');
  });
});
