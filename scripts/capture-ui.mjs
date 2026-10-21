// Development-only visual review of the actual production UI. No AI responses are simulated.
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const root = fileURLToPath(new URL('../dist/', import.meta.url));
const output = fileURLToPath(new URL('../docs/screenshots/', import.meta.url));
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
};
const server = createServer(async (req, res) => {
  try {
    const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    const file = resolve(root, pathname === '/' ? 'index.html' : '.' + pathname);
    if (!file.startsWith(root.endsWith(sep) ? root : root + sep)) {
      res.writeHead(403).end();
      return;
    }
    res.writeHead(200, { 'Content-Type': types[extname(file)] || 'application/octet-stream' });
    res.end(await readFile(file));
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const { port } = server.address();
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
    colorScheme: 'light',
  });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await mkdir(output, { recursive: true });
  await page.goto(`http://127.0.0.1:${port}/`);
  await page.getByRole('heading', { name: /Un pensiero/ }).waitFor();
  await page.screenshot({ path: resolve(output, 'home.png') });
  await page.getByRole('button', { name: /Il tuo spazio Locale/ }).click();
  await page.getByRole('button', { name: 'Scuro', exact: true }).click();
  await page.screenshot({ path: resolve(output, 'settings-dark.png') });
  await page.keyboard.press('Escape');
  await page.screenshot({ path: resolve(output, 'home-dark.png') });
  await page.getByRole('button', { name: /Il tuo spazio Locale/ }).click();
  await page.getByRole('button', { name: 'Chiaro', exact: true }).click();
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 800, height: 720 });
  await page.screenshot({ path: resolve(output, 'compact-window.png') });
  if (errors.length) throw new Error(errors.join('\n'));
  console.info(`Saved 4 production UI screenshots in ${output}`);
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
