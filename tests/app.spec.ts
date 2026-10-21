import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('textbox', { name: 'Messaggio per Claude' })).toBeVisible();
});

test('first run is honest about connection and preserves a draft', async ({ page }) => {
  await expect(page.getByRole('heading', { name: /Un pensiero/ })).toBeVisible();
  await expect(page.getByText('Anteprima UI', { exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Messaggio per Claude' }).fill('Una bozza da conservare');
  await page.waitForTimeout(350);
  await page.reload();
  await expect(page.getByRole('textbox', { name: 'Messaggio per Claude' })).toHaveValue(
    'Una bozza da conservare',
  );
  await page.getByRole('button', { name: 'Invia messaggio', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Accedi con Claude', exact: true })).toBeDisabled();
  await expect(page.getByText('Anteprima nel browser.', { exact: false })).toBeVisible();
});

test('theme persists, dialogs dismiss and keyboard search works', async ({ page }) => {
  await page.getByRole('button', { name: /Il tuo spazio Locale/ }).click();
  await page.getByRole('button', { name: 'Scuro', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.keyboard.press('Control+k');
  await expect(page.getByRole('textbox', { name: 'Cerca conversazioni' })).toBeVisible();
  await page.keyboard.press('Escape');
  await page
    .getByRole('button', { name: /Progetti/ })
    .first()
    .click();
  await expect(page.getByRole('heading', { name: /I tuoi progetti/ })).toBeVisible();
});

test('history search, archived conversations and untrusted markdown', async ({ page }) => {
  await page.evaluate(() => {
    const chat = {
      id: 'test',
      title: 'Ricerca prova',
      model: 'sonnet',
      mode: 'chat',
      projectId: null,
      cwd: null,
      sessionId: null,
      pinned: false,
      archived: false,
      createdAt: 1,
      updatedAt: 1,
    };
    localStorage.setItem(
      'snor.ui-preview.v1',
      JSON.stringify({
        chats: [chat],
        projects: [],
        settings: {},
        messages: {
          test: [
            {
              id: 'm',
              role: 'assistant',
              text: '## Risposta\n<script>window.__injected=true</script><img src="https://example.com/tracker" onerror="alert(1)">\nUn testo sicuro.',
              status: 'complete',
              createdAt: 1,
            },
          ],
        },
      }),
    );
  });
  await page.reload();
  await page.getByRole('button', { name: 'Ricerca prova', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Risposta', exact: true })).toBeVisible();
  await expect(page.locator('.markdown img,.markdown script')).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__injected)).toBeUndefined();
  await page.getByRole('button', { name: 'Opzioni conversazione' }).click();
  await page.getByRole('button', { name: 'Archivia', exact: true }).click();
  await page.getByRole('button', { name: 'Archivio', exact: true }).click();
  await expect(page.getByRole('button', { name: /Ricerca prova/ })).toBeVisible();
});

test('tool details and reasoning enter the page only when opened', async ({ page }) => {
  await page.evaluate(() => {
    const chat = {
      id: 'tools',
      title: 'Strumenti prova',
      model: 'sonnet',
      mode: 'code',
      projectId: null,
      cwd: null,
      sessionId: null,
      pinned: false,
      archived: false,
      createdAt: 1,
      updatedAt: 1,
    };
    localStorage.setItem(
      'snor.ui-preview.v1',
      JSON.stringify({
        chats: [chat],
        projects: [],
        settings: {},
        messages: {
          tools: [
            {
              id: 'm',
              role: 'assistant',
              text: 'Fatto.',
              thinking: 'Ragionamento nascosto',
              tools: [
                {
                  id: 't',
                  name: 'Read',
                  input: { file_path: 'note.txt' },
                  status: 'complete',
                  output: 'contenuto letto',
                },
              ],
              status: 'complete',
              createdAt: 1,
            },
          ],
        },
      }),
    );
  });
  await page.reload();
  await page.getByRole('button', { name: 'Strumenti prova', exact: true }).click();
  await expect(page.getByText('Completato', { exact: true })).toBeVisible();
  await expect(page.locator('.tool-card pre')).toHaveCount(0);
  await expect(page.getByText('Ragionamento nascosto')).toHaveCount(0);
  await page.locator('.tool-card summary').click();
  await expect(page.getByText('contenuto letto')).toBeVisible();
  await expect(page.locator('.tool-card pre').first()).toContainText('note.txt');
  await page.locator('.thinking summary').click();
  await expect(page.getByText('Ragionamento nascosto')).toBeVisible();
});
