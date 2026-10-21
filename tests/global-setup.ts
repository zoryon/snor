import { serveProduction } from '../scripts/preview-server.mjs';

export default async function setup() {
  const preview = await serveProduction(1420);
  return async () => {
    await preview.close();
  };
}
