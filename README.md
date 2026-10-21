# Snor

Un'app desktop Tauri per conversare e lavorare con **Claude Code installato sul computer**, usando il suo accesso tramite abbonamento. Interfaccia Svelte, backend Rust, cronologia SQLite. Nessun server Node a runtime e nessuna chiamata del frontend alle API dei modelli.

**Stato: prima implementazione 0.1.0.** Non è ancora una replica completa di Claude Desktop. Le funzioni presenti, quelle pianificate e i limiti di verifica sono dichiarati in [docs/ROADMAP.md](docs/ROADMAP.md) e [docs/VERIFICATION.md](docs/VERIFICATION.md).

[Anteprima dell'interfaccia](docs/screenshots/home.png) · [Risultati del collaudo nativo](docs/VERIFICATION.md)

La prima build Windows è disponibile nella cartella `releases/`, come eseguibile portabile e ZIP con documentazione. Non richiede Node o Rust per essere usata; richiede WebView2 e Claude Code per l'AI. Il pacchetto locale non è firmato e non contiene un installer né aggiornamenti automatici.

## Avvio rapido su Windows

Prerequisiti: Node.js 24 LTS, Rust stabile con target MSVC, Visual Studio Build Tools con sviluppo desktop C++, Windows SDK e WebView2. Per usare l'AI serve il binario nativo ufficiale Claude Code e un abbonamento compatibile.

```powershell
npm.cmd ci
npm.cmd run tauri -- dev
```

`npm.cmd` evita i problemi di execution policy di PowerShell con `npm.ps1`. Non occorre modificare la policy di sistema.

Per compilare l'eseguibile e l'installer:

```powershell
npm.cmd run tauri -- build
```

Output: `src-tauri/target/release/snor.exe`; installer in `src-tauri/target/release/bundle/nsis/`. La prima compilazione scarica dipendenze e può richiedere diversi minuti. Il processo di sviluppo e la build release hanno consumi e tempi differenti.

## Collegare Claude Code

1. Installa il **binario nativo** seguendo [le istruzioni ufficiali](https://code.claude.com/docs/en/setup).
2. Esegui `claude auth login` e accedi con il tuo abbonamento.
3. Apri **Impostazioni → Claude Code → Salva e verifica**. Se necessario, indica il percorso completo di `claude.exe`.

Snor avvia il binario come processo e usa `stream-json` su stdin/stdout. Non legge né copia il file delle credenziali. Non usa `--bare`, perché questa modalità non legge l'accesso tramite abbonamento. L'autenticazione e le regole del servizio restano di Anthropic.

## Cosa puoi già fare

- Conversazioni con streaming di testo e ragionamento, strumenti e approvazioni puntuali.
- Modalità Conversazione, Attività e Codice; selezione Sonnet, Opus o Haiku tramite alias della CLI.
- Progetti con cartella e istruzioni; cronologia locale paginata, ricerca nei titoli e nel testo, preferiti, archivio ed esportazione.
- Allegati immagini e testo UTF-8 con limiti di dimensione.
- Temi chiaro/scuro/sistema, spaziatura compatta, bozze e scorciatoie.
- Diagnostica dell'installazione e login tramite il flusso ufficiale.

La CLI non è inclusa nell'app. Senza Claude Code installato e autenticato puoi esplorare la UI, ma non ricevere risposte AI.

## Anteprima web

```powershell
npm.cmd run dev
```

Apri `http://127.0.0.1:1420`. L'anteprima è etichettata e conserva solo dati UI in localStorage. **Non simula un'AI** e non avvia processi, autenticazione o accessi ai file. Serve per sviluppare e verificare l'interfaccia; non sostituisce i test desktop.

## Verifiche

```powershell
npm.cmd run verify
cargo test --manifest-path src-tauri/Cargo.toml
npm.cmd exec -- playwright install chromium
npm.cmd run test:ui
```

## Documentazione

| Documento                                         | Contenuto                                                    |
| ------------------------------------------------- | ------------------------------------------------------------ |
| [Sviluppo](docs/DEVELOPMENT.md)                   | Setup, struttura, comandi, troubleshooting e build           |
| [Architettura](docs/ARCHITECTURE.md)              | Componenti, sessioni, eventi, persistenza e decisioni        |
| [Protocollo Claude Code](docs/CLAUDE_PROTOCOL.md) | Avvio, messaggi, handshake, permessi e compatibilità         |
| [Dati e sicurezza](docs/SECURITY.md)              | Confini reali, contenuti non attendibili, credenziali e dati |
| [Prestazioni](docs/PERFORMANCE.md)                | Obiettivi, metodologia, RAM totale e strumenti di misura     |
| [Specifica prodotto](docs/PRODUCT.md)             | Esperienza e funzionalità desiderate                         |
| [Roadmap](docs/ROADMAP.md)                        | Stato delle funzioni e fasi successive                       |
| [Verifica](docs/VERIFICATION.md)                  | Risultati verificati e prove ancora necessarie               |
| [Riferimenti](docs/REFERENCES.md)                 | Documentazione e progetti consultati                         |

Snor è indipendente da Anthropic. Il supporto Anthropic al 7 ottobre 2026 dichiara che SDK, `claude -p` e app di terzi possono usare i limiti dell'abbonamento; le condizioni possono cambiare. La distribuzione commerciale, il marchio e l'offerta di login agli utenti richiedono una valutazione separata: vedere [riferimenti e discrepanze documentali](docs/REFERENCES.md).
