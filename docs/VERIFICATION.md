# Verifica della versione 0.1.0

Questo documento viene aggiornato con i risultati effettivamente ottenuti durante lo sviluppo. Non confondere codice implementato con integrazione reale collaudata.

## Ambiente

- Workspace: Windows x64.
- Node: 24.16.0.
- Rust: 1.97.1.
- MSVC e Windows SDK presenti.
- Claude Code: non rilevato nel PATH o nelle cartelle standard all'avvio del lavoro.

## Risultati

| Controllo                           | Risultato                                                                                                                                                                                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Svelte / TypeScript                 | Passato: zero errori e zero warning                                                                                                                                                                                                                |
| Test unitari frontend               | 4 passati: routing dei delta, snapshot finali, recupero e limiti                                                                                                                                                                                   |
| Test Rust                           | 11 passati: protocollo, frame limitati, lettura stdout a frammenti lenti, svuotamento stderr non UTF-8, errori di fine turno, input strumenti grandi, ricerca/paginazione/cancellazione SQLite, session ID senza sovrascritture, URL e job Windows |
| Test browser sul bundle production  | 4 passati: primo avvio e bozza, temi/tastiera, archivio e Markdown non attendibile, strumenti e ragionamento renderizzati solo all'apertura                                                                                                        |
| Rust Clippy con warning come errori | Passato                                                                                                                                                                                                                                            |
| Audit npm                           | Zero vulnerabilità dopo aggiornamento dei tool di sviluppo                                                                                                                                                                                         |
| Build frontend                      | Passata; JS principale circa 94,2 kB / 34,4 kB gzip, Markdown separato circa 67,7 kB                                                                                                                                                               |
| Revisione visiva                    | Home chiara/scura, impostazioni e finestra 800×720, senza errori JavaScript rilevati                                                                                                                                                               |
| Build release Windows               | Passata, eseguibile x64 da 4.968.960 byte (circa 4,74 MiB) con asset locali incorporati                                                                                                                                                            |
| Avvio nativo e misura memoria       | Passati: UI release e IPC effettivi, cinque avvii e cinque campioni di memoria                                                                                                                                                                     |

I test browser sono eseguiti su un server statico locale soltanto di sviluppo, terminato nello stesso processo del runner. Non dimostrano la funzionalità del backend nativo. Il test dei processi Rust avvia davvero Node e un suo figlio e verifica la terminazione tramite job Windows; non invoca un modello né un account Claude.

Le schermate in [screenshots/](screenshots/) mostrano l'anteprima UI etichettata. Non contengono risposte AI simulate.

## Misure native effettive

Eseguite il 21 ottobre 2026 sull'ambiente locale Windows 10 build 19045, con il binario release x64 e il profilo WebView2 già esistente. Cinque riavvii del processo, non un richiamo da tray:

| Metrica                                         | Risultato osservato                                     |
| ----------------------------------------------- | ------------------------------------------------------- |
| Tempi esterni avvio → segnale DOM pronto        | 1.551,90 / 1.490,12 / 1.322,00 / 1.347,08 / 1.375,19 ms |
| Mediana                                         | 1.375,19 ms                                             |
| p95 campionario (cinque prove)                  | 1.551,90 ms                                             |
| Memoria privata residente, UI + WebView2        | 97,83–101,78 MiB                                        |
| Memoria privata impegnata (private bytes)       | 141,29–145,27 MiB                                       |
| Somma dei working set, incluse pagine condivise | 320,16–326,41 MiB                                       |
| Processi nell'albero                            | 7: Snor e sei processi WebView2                         |
| Claude Code / strumenti AI                      | Non avviati, non inclusi                                |

Il primo smoke test su un profilo appena creato aveva richiesto circa 3.099 ms; non è un benchmark controllato dell'avvio a freddo. Il target di apertura in pochi millisecondi **non è raggiunto**. Il target di 200 MB con motore e strumenti **non è dimostrato**. La somma dei working set conteggia anche pagine condivise più volte e non equivale alla memoria privata residente; entrambe sono riportate per evitare confronti fuorvianti.

Report completi: [avvii](measurements/startup.json), [memoria](measurements/memory.json). Gli script di misura sono in `scripts/`. Le prove avviano la finestra in modalità nascosta, aspettano un segnale reale dal frontend e richiudono l'app. I risultati dipendono dalla macchina, dalle cache e dal runtime WebView2; non sono promesse per tutti i computer.

## Ottimizzazioni e correzioni del 21 ottobre 2026

Dopo le modifiche descritte nel [changelog](../CHANGELOG.md) sono stati rieseguiti svelte-check (zero errori e warning), test frontend, Clippy con warning come errori, test Rust e test browser, tutti passati. Build release passata: eseguibile di 5.069.824 byte (circa 4,83 MiB, +98 KiB per i nuovi task e la gestione degli eventi).

Confronto nativo eseguito con lo script di smoke, cinque avvii alternati del binario precedente e di quello nuovo sulla stessa macchina, profilo WebView2 caldo, senza motore AI:

| Metrica (mediana di 5)                   | Prima       | Dopo        |
| ---------------------------------------- | ----------- | ----------- |
| Avvio esterno → segnale DOM pronto       | 1.109,54 ms | 1.058,71 ms |
| Ingresso Rust → IPC frontend pronto      | 1.064,46 ms | 985,74 ms   |
| Memoria privata residente, UI + WebView2 | 88,34 MiB   | 89,93 MiB   |
| Memoria privata impegnata                | 132,04 MiB  | 132,92 MiB  |

Le differenze a riposo rientrano nella variabilità fra avvii (singoli avvii fra 975 e 2.119 ms): **avvio e memoria inattiva sono invariati**, non migliorati. Le modifiche agiscono sullo streaming, sull'IPC e su SQLite, che queste prove non esercitano; i loro effetti con una risposta AI reale **non sono misurati** perché Claude Code non è installato su questa macchina. Report: [confronto](measurements/optimization-comparison.json).

## Non verificato senza account/motore

- Autenticazione tramite abbonamento, costo effettivo e quote.
- Conversazione reale, strumenti, interruzione e ripresa con una versione specifica di Claude Code.
- Accorpamento dei delta in Rust, separazione dei blocchi di testo e gestione delle approvazioni tardive con eventi reali della CLI.
- Consumo di CPU e memoria durante una risposta lunga, prima e dopo le ottimizzazioni.
- Disponibilità dei singoli modelli per l'account.
- Equivalenza delle risposte rispetto a Claude Desktop.

## Limiti da non presentare come risolti

- Nessuna garanzia di 200 MB totali o avvio a freddo in pochi millisecondi.
- Nessuna sincronizzazione della cronologia claude.ai.
- Anteprima browser distinta dal desktop nativo.
- Nessuna equivalenza completa con Claude Desktop, Cowork o i servizi cloud Anthropic.
