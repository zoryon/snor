# Prestazioni

## Obiettivi, non risultati dichiarati

| Metrica                               | Obiettivo iniziale                                          |
| ------------------------------------- | ----------------------------------------------------------- |
| Avvio a freddo, finestra utilizzabile | Entro 1 s sul PC di riferimento                             |
| Richiamo residente                    | Entro 100 ms; richiede tray/hotkey, non ancora implementati |
| UI durante streaming                  | Interazioni fluide; misurare latenza input e long task      |
| RAM complessiva                       | Obiettivo 200 MB, includendo tutti i processi               |
| CPU inattiva                          | Nessun polling intenzionale del frontend                    |

Un avvio completo di Tauri/WebView2 in pochi millisecondi non è garantito. La risposta del modello dipende anche dalla rete e dal servizio: separare UI pronta, processo Claude pronto e primo token.

## Scelte già presenti

- Font e asset locali; niente download nell'apertura della UI.
- Bundle Markdown separato, importato solo montando i messaggi.
- Nessuna scansione dei transcript Claude all'avvio.
- Metadati di massimo 200 chat e 60 messaggi iniziali.
- Delta di testo e ragionamento accorpati in Rust ogni 50 ms, prima dell'IPC.
- Nel frontend viene aggiornato solo il messaggio interessato; le card dei messaggi invariati non vengono rivalutate (barriere `$derived`).
- Markdown + DOMPurify della risposta in streaming al massimo ogni 90 ms; chunk Markdown caricato una sola volta e prelevato quando la UI è inattiva.
- Input/output degli strumenti e ragionamento renderizzati solo quando aperti.
- Comandi SQLite fuori dal thread principale; connessione di lettura separata; `synchronous=NORMAL` in WAL; ricerca con `EXISTS` e indice su `updated_at`; statement preparati in cache.
- Checkpoint dello streaming solo se il contenuto è cambiato; buffer del turno liberati a fine risposta.
- Allegati spostati nel frame JSON invece di essere copiati; input degli strumenti oltre 32 KiB non duplicati verso UI e database.
- Diagnostica: `--version` e `auth status` in parallelo; verifica dell'abbonamento riutilizzata per 10 minuti nei riavvii di sessione.
- Un solo motore, avvio su richiesta e chiusura dopo 5 minuti inattivi.
- Code e buffer limitati; risposta massima locale di 2 MiB.
- `content-visibility` sui messaggi; **non è ancora una virtualizzazione completa**.
- Ottimizzazione release Rust con thin LTO e strip.

Queste scelte riducono lavoro e allocazioni nel percorso dello streaming. I guadagni durante una risposta AI reale **non sono ancora misurati**: richiedono Claude Code installato e i criteri descritti sotto.

## Misurare la memoria correttamente

Non confrontare il solo `snor.exe` con l'intero gruppo di un altro programma. Includere WebView2, GPU/utility renderer, Claude Code e gli strumenti figli. Il working set comprende pagine condivise: sommarlo è un indicatore operativo conservativo, non una misura esatta della RAM fisica unica. Riportare anche private bytes e la metrica scelta.

```powershell
.\scripts\measure-process-tree.ps1 -RootProcessId 1234 -Samples 30 -IntervalMs 1000 -OutputPath .\measurements\idle.json
```

Lo script è in sola lettura sui processi e scrive soltanto il report scelto. Se WebView2 o altri processi vengono reparentati, il campionamento del solo albero può perderli: controllare il gruppo anche con Process Explorer. Memoria GPU dedicata va riportata separatamente.

I report usano MiB (1.048.576 byte). Un budget di 200 MB decimali corrisponde a circa 190,73 MiB: indicare sempre l'unità quando si confrontano i risultati.

## Scenari riproducibili

Compilare release, annotare Windows, CPU, RAM, storage, WebView2 e versione Claude Code. Eseguire almeno 10 avvii a freddo e 20 riaperture, riportare mediana e p95. Provare: home vuota, archivio 1.000 chat, conversazione 200 messaggi, risposta lunga, strumenti, immagine, inattività 10 minuti e chiusura.

Il log frontend `[Snor] UI ready ...` misura da `onMount` al caricamento dello stato. **Non misura l'avvio del processo**, la creazione WebView2 o il tempo fino al primo token. Non usarlo per affermare «avvio in X ms» dell'app completa.

Per una prova nativa con chiusura automatica:

```powershell
.\scripts\smoke-desktop.ps1
```

Lo script avvia il binario release nascosto, imposta `SNOR_STARTUP_REPORT` soltanto per la prova e attende un segnale IPC del frontend dopo il caricamento dello stato e l'aggiornamento del DOM. Riporta sia il tempo esterno da Start-Process al segnale, sia il tempo dall'ingresso Rust alla richiesta IPC. La scrittura del segnale è atomica. Non misura il primo token, non certifica il paint completo e un singolo campione non costituisce un benchmark a freddo. Raccoglie cinque campioni di memoria dell'albero e chiude l'app; non avvia il motore AI.

Se la policy locale blocca gli script di sviluppo, è possibile eseguire questo file già revisionato con `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-desktop.ps1`. L'opzione vale per quel processo e non modifica la configurazione globale.

Misurare gli asset con `npm run build`. Usare i test browser per rilevare regressioni funzionali, non per affermare i consumi dell'eseguibile Tauri.

## Rischi da affrontare

Il motore Claude può superare da solo il budget. Aprire terminali, anteprime e più agenti aumenta i consumi. La ricerca LIKE e la serializzazione di allegati grandi richiedono profilazione. `App.svelte` è ancora in modalità legacy: convertirlo alle rune ridurrebbe ulteriormente il lavoro per aggiornamento. WebView2 espone un livello di memoria ridotto per le finestre inattive (`MemoryUsageTargetLevel`), da valutare con misure quando esisterà una modalità residente.

## Baseline della prima versione

La prima release è stata realmente avviata: mediana di circa 1,38 secondi su cinque avvii con cache calda e circa 98–102 MiB di memoria privata residente per UI e WebView2, senza motore AI. Tutte le metriche e le limitazioni sono in [VERIFICATION.md](VERIFICATION.md). Questi dati non soddisfano ancora il requisito di pochi millisecondi né provano il budget complessivo con Claude Code.

Le prossime ottimizzazioni devono distinguere il costo di inizializzazione WebView2 dal caricamento degli asset. Il richiamo in pochi millisecondi richiede probabilmente una finestra residente con tray/hotkey, da misurare insieme al costo di memoria inattiva. Non disabilitare protezioni del browser o GPU alla cieca per ottenere un numero migliore.
