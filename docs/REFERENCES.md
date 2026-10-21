# Riferimenti e compatibilità

Consultati il 21 ottobre 2026. Le fonti descrivono prodotti e protocolli soggetti a cambiamento.

## Anthropic

- [Claude Code programmatico](https://code.claude.com/docs/en/headless): modalità `-p`, streaming, bare mode e limiti.
- [CLI reference](https://code.claude.com/docs/en/cli-reference): flag e comandi di autenticazione.
- [Agent SDK, streaming input](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode): processo persistente, interruzioni e richieste interattive.
- [SDK Python, trasporto](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/transport/subprocess_cli.py): avvio del processo e argomenti.
- [SDK Python, query](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py): handshake e frame di controllo.
- [Uso SDK con il piano Claude](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan): aggiornamento del 7 ottobre 2026 sull'uso tramite abbonamento.
- [SDK overview](https://code.claude.com/docs/en/agent-sdk/overview): conserva indicazioni restrittive per offrire login claude.ai nei prodotti di terzi. La distribuzione commerciale necessita quindi di una verifica dedicata, senza dedurre automaticamente autorizzazioni dal solo funzionamento tecnico.
- [Desktop](https://code.claude.com/docs/en/desktop), [Cowork](https://support.claude.com/en/articles/13345190-get-started-with-claude-cowork), [artefatti](https://support.claude.com/en/articles/17153992-what-are-artifacts-and-how-do-i-use-them): riferimenti di prodotto, non garanzie di accesso alle loro funzionalità da Snor.

## Desktop e framework

- [Prerequisiti Tauri](https://v2.tauri.app/start/prerequisites/).
- [Architettura Tauri](https://v2.tauri.app/concept/architecture/).
- [Modello dei processi WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model).
- [Prestazioni WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance).

## Esempi di applicazioni già realizzate

- [OpenCovibe](https://github.com/AnyiWang/OpenCovibe): Rust, Tauri, Svelte e collegamento JSON persistente.
- [Opcode](https://github.com/winfunc/opcode): Tauri e avvio diretto della CLI.
- [Yume](https://github.com/aofp/yume): GUI basata sul binario ufficiale.
- [Claude in Zed](https://zed.dev/blog/claude-code-via-acp): SDK e adattatore ACP.

I dati di performance dichiarati dagli autori non sono benchmark di Snor. Nessun codice di questi repository è stato importato.
