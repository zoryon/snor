# Sviluppo

## Ambiente di riferimento

- Windows x64; target Rust `x86_64-pc-windows-msvc`.
- Node 24 LTS, npm e toolchain Rust stabile.
- MSVC C++ Build Tools, Windows SDK, WebView2.
- Claude Code nativo solo per verifiche AI reali.

Gli strumenti frontend vengono usati per sviluppo/build. L'eseguibile distribuito non richiede Node per la UI o per un server intermedio. Claude Code gestisce autonomamente il proprio runtime.

## Installazione e comandi

```powershell
npm.cmd ci
npm.cmd run check
npm.cmd run test
npm.cmd run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm.cmd run tauri -- dev
npm.cmd run tauri -- build
```

`package-lock.json` e `src-tauri/Cargo.lock` fissano le risoluzioni. Usare `npm ci` per riprodurre l'installazione. Cargo può richiedere download alla prima build; la cache è nella directory utente.

`npm.cmd run verify` esegue controlli frontend, test unitari e bundle di produzione. I test browser richiedono `npm.cmd exec -- playwright install chromium`, poi `npm.cmd run test:ui`.

## Cartelle

```text
src/
  App.svelte                orchestrazione UI e viste
  styles.css, tokens.css    stile e variabili di tema
  components/               icone, modali, messaggi, Markdown caricato su richiesta
  lib/api.ts                IPC desktop e anteprima browser separata
  lib/events.ts             riconciliazione degli eventi
  lib/types.ts              contratti TypeScript
src-tauri/
  src/lib.rs                comandi IPC e composizione dell'app
  src/agent.rs              ciclo di vita del processo persistente
  src/protocol.rs           frame del protocollo Claude Code
  src/store.rs              SQLite e query
  capabilities/main.json    permessi della finestra locale
tests/                      test browser
scripts/                    misurazioni riproducibili
docs/                       documentazione
```

## Troubleshooting

- **npm.ps1 bloccato:** usare `npm.cmd`; non abbassare l'execution policy.
- **link.exe / Windows SDK mancanti:** installare il workload C++ di Visual Studio. Riprovare da Developer PowerShell se il rilevamento automatico non funziona.
- **WebView2 mancante:** installare il runtime Evergreen ufficiale Microsoft.
- **Claude non trovato:** indicare il percorso dell'eseguibile nativo nelle impostazioni. Script `.cmd` e `.ps1` non sono supportati come binario su Windows.
- **Login:** provare `claude auth status` e `claude auth login` nel terminale. Le credenziali rimangono gestite dalla CLI.
- **Protocollo incompatibile:** registrare versione CLI e fase dell'errore. Non aggirare le approvazioni con `--dangerously-skip-permissions`.
- **Una sola esecuzione alla volta:** è una scelta intenzionale di questa versione per tenere sotto controllo le risorse. Si possono creare più chat, ma un solo turno alla volta.

## Distribuzione

L'installer NSIS generato localmente non è firmato. Prima di una distribuzione pubblica servono firma, aggiornamenti firmati, revisione delle condizioni Anthropic e completamento delle verifiche desktop. Non distribuire segreti, profili di login o dati locali nel bundle.

## Modifiche future

Aggiornare roadmap e documento di verifica insieme al codice. Le funzioni dipendenti dalla CLI devono avere un test reale per una versione CLI dichiarata, oltre ai test con fixture. Evitare server intermedi, librerie UI pesanti, polling continuo e caricamento dell'intera cronologia all'avvio.
