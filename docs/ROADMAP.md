# Funzionalità e roadmap

Legenda: **Implementato** = codice presente, vedere il livello di verifica separato. **Parziale** = solo una parte del requisito. **Pianificato** = non disponibile. Le voci implementate non implicano un collaudo AI reale.

| Area                             | Stato v0.1                             | Dettaglio                                                         |
| -------------------------------- | -------------------------------------- | ----------------------------------------------------------------- |
| Shell Tauri + Svelte             | Implementato                           | Nessun server Node a runtime                                      |
| Temi chiaro/scuro/sistema        | Implementato                           | Font di sistema e spaziatura compatta                             |
| Scorciatoie                      | Implementato                           | Ctrl/Cmd K ricerca, N nuova chat, virgola impostazioni            |
| Chat persistenti                 | Implementato                           | SQLite, pagine da 60 messaggi                                     |
| Ricerca globale locale           | Implementato                           | Titoli e testo via LIKE fuori dal thread UI, massimo 200 chat     |
| Preferiti, archivio, rinomina    | Implementato                           | Gestione UI e SQLite                                              |
| Streaming e sessione persistente | Implementato, da provare con CLI reale | JSON bidirezionale e handshake                                    |
| Ripresa sessione CLI             | Implementato, da provare con CLI reale | Session ID e --resume                                             |
| Login e diagnostica              | Implementato, da provare con account   | CLI ufficiale; nessuna lettura di token                           |
| Modelli                          | Parziale                               | Alias sonnet/opus/haiku; nessuna discovery dinamica               |
| Ragionamento                     | Parziale                               | Visualizzazione del testo esposto; nessun controllo budget        |
| Strumenti e permessi             | Parziale                               | Schede, allow/deny singolo; niente regole permanenti              |
| Stop                             | Parziale                               | Interrupt del protocollo; collaudo process tree ancora necessario |
| Domande strutturate              | Pianificato                            | UI dedicata AskUserQuestion                                       |
| Progetti                         | Parziale                               | Cartella, istruzioni, chat; niente biblioteca indicizzata         |
| Memoria                          | Pianificato                            | La CLI può usare la propria; non esiste ancora un editor Snor     |
| Allegati                         | Parziale                               | Immagini e testo; niente PDF/Office o drag-and-drop               |
| Markdown                         | Parziale                               | Rendering sanitizzato; niente highlighter o formule avanzate      |
| Esportazione                     | Implementato                           | Markdown e JSON con dialogo nativo                                |
| Importazione                     | Pianificato                            | Sessioni Claude e backup Snor                                     |
| Modifica e rigenerazione         | Parziale                               | Riutilizzo nel composer; niente modifica storica o rami           |
| Artefatti e anteprime            | Pianificato                            | Caricamento su richiesta e isolamento necessari                   |
| Editor, diff e terminale         | Pianificato                            | Nessun editor completo incluso                                    |
| Git / worktree                   | Pianificato                            | Possibili strumenti della CLI, senza UI dedicata                  |
| MCP, plugin e skill              | Parziale                               | La CLI può caricare i propri; manca un gestore UI                 |
| Sessioni simultanee              | Pianificato                            | Un solo turno/motore attivo per ora                               |
| Tray e hotkey globale            | Pianificato                            | Nessun residente in background per il richiamo                    |
| Pianificazione e notifiche       | Pianificato                            | Nessun servizio residente                                         |
| Voce, browser, computer use      | Pianificato                            | Integrazioni separate                                             |
| Aggiornamenti automatici         | Pianificato                            | Prima firma e distribuzione verificata                            |
| Backup e cifratura               | Parziale                               | Export sì; nessun backup automatico o cifratura app               |
| Prestazioni <200 MB totali       | Da misurare                            | Non dichiarate come raggiunte                                     |
| Avvio in pochi ms                | Non garantito                          | Distinguere avvio a freddo e richiamo residente                   |

## Fase 1: consolidamento

- Prova reale con binario Claude Code e abbonamento.
- Processi figli, stop affidabile e persistenza dei turni interrotti.
- Test desktop nativi e misurazioni release con WebView e CLI incluse.
- Accessibilità, focus e gestione degli errori.

## Fase 2: uso quotidiano

- Importazione sessioni, rigenerazione/rami e domande strutturate.
- Allegati PDF, drag-and-drop, riferimenti @file.
- FTS per ricerca e virtualizzazione completa delle chat lunghe.
- Tray, richiamo globale, notifiche e gestione idle configurabile.

## Fase 3: workspace

- Artefatti, anteprime isolate, editor/diff e Git.
- Gestore MCP/skill/plugin e memoria modificabile.
- Concorrenza opzionale con budget delle risorse esplicito.

## Fase 4: distribuzione

- Installer firmati e aggiornamenti firmati.
- Portabilità, backup coerenti e politiche dati.
- Verifiche delle integrazioni cloud e delle condizioni commerciali.
