# Architettura

## Flusso

```text
Svelte 5 nella WebView di Tauri
          ⇅ IPC Tauri, finestra locale
Rust: comandi / SQLite / attore della sessione
          ⇅ stdin + stdout, JSON delimitato da newline
Binario ufficiale Claude Code
          ⇅ rete e autenticazione gestite dalla CLI
Anthropic
```

Non esiste un backend Node, un proxy HTTP del modello o uno scraping del terminale. Non sono state copiate implementazioni dai progetti consultati; i riferimenti servono a documentare gli approcci.

## Avvio

La finestra visualizza gli asset locali. Il frontend chiama `bootstrap` per impostazioni, progetti e un massimo di 200 metadati di chat. I messaggi vengono letti solo per la conversazione aperta, 60 alla volta. La diagnostica Claude parte dopo il caricamento dello stato e non ritarda la visualizzazione intenzionalmente. Il parser Markdown è un chunk separato, importato quando vengono montati messaggi.

Il database viene aperto in setup Tauri. I comandi Tauri sincroni girerebbero sul thread principale: tutti i comandi che usano SQLite sono quindi asincroni e svolgono le query nel pool bloccante, così una ricerca lenta non congela la finestra. Per un archivio molto grande la ricerca `LIKE` resta lineare e andrà sostituita da FTS.

## Sessione

`Engine` possiede al massimo un `Session`. Ogni sessione ha una coda Tokio limitata e un solo attore che possiede processo, stdin e richieste pendenti. Un `AtomicBool` impedisce turni simultanei anche se arrivano richieste IPC concorrenti.

stdout e stderr sono letti da due task dedicati. stdout consegna all'attore frame JSON già decodificati tramite un canale limitato a 8 frame: la lettura di una riga non è cancel-safe e, dentro il `select!` dell'attore, un altro ramo vinto a metà frame avrebbe scartato i byte già letti. stderr viene svuotato fino alla fine con decodifica tollerante e memoria costante (si conserva solo l'ultima riga), così la CLI non può bloccarsi su una pipe piena.

La verifica dell'abbonamento (`claude auth status`) avvia un processo: un esito positivo viene riutilizzato per 10 minuti nei riavvii di sessione, ed è invalidato da login, diagnostica negativa o errore della sessione.

Il processo viene avviato alla prima richiesta, inizializzato con un handshake e mantenuto aperto per i messaggi successivi. Un cambio di chat, modello o modalità sostituisce la sessione inattiva; la CLI può riprenderla tramite il suo session ID. Dopo cinque minuti senza turni il processo viene chiuso per liberare memoria. Questo scambia un successivo costo di avvio con un minor consumo inattivo.

La richiesta di stop invia `interrupt`; dopo otto secondi senza una conclusione il processo viene terminato. Su Windows un job con `KILL_ON_JOB_CLOSE` comprende processo e discendenti. Un test esegue realmente un processo Node con un figlio e verifica che la chiusura del job termini entrambi. Il comportamento degli strumenti Claude reali e il ripristino dei task interrotti restano da collaudare. Il job gestisce il ciclo di vita, non è una sandbox di sicurezza.

## Eventi

Ogni evento ha `chatId`, `messageId`, `kind`, `data`. Testo e ragionamento sono accorpati in Rust in finestre di 50 ms prima di attraversare l'IPC: un evento per finestra invece di uno per token. Qualsiasi altro evento svuota prima il buffer, quindi l'ordine resta quello prodotto da Claude Code. Il frontend unisce gli eventi dello stesso frame (16 ms) e aggiorna soltanto il messaggio interessato. Eventi di altre risposte non vengono applicati al messaggio corrente. Gli eventi arrivati fra l'invio IPC e il ritorno dell'ID vengono temporaneamente accodati nel frontend.

Quando un turno contiene più messaggi dell'assistente (testo, strumento, testo), i blocchi di testo vengono separati da una riga vuota invece di essere concatenati.

Eventi principali: `status`, `init`, `delta`, `thinking`, `tool`, `tool_result`, `permission`, `permission_cancelled`, `done`, `failure`, `closed`. I messaggi completi e falliti vengono salvati dal backend; durante il turno viene salvato un checkpoint ogni due secondi, solo se il contenuto è cambiato. Un errore della sessione aggiorna soltanto la risposta in corso, mai un messaggio già concluso; i messaggi accettati ma mai consegnati alla CLI vengono marcati come falliti. Una risposta di approvazione tardiva o duplicata viene ignorata invece di chiudere la sessione. Le bozze sono salvate separatamente in localStorage con debounce; le bozze vuote vengono rimosse.

## Rendering

`MessageCard`, `ToolCard` e `Markdown` usano le rune di Svelte 5. `App.svelte` è ancora in modalità legacy, che segnala come modificato ogni elemento della lista a ogni aggiornamento: le card leggono i campi del messaggio tramite `$derived`, così l'aggiornamento si ferma per i messaggi invariati. Durante lo streaming il Markdown della risposta viene rianalizzato e sanitizzato al massimo ogni 90 ms; il testo finale subito. Input e output degli strumenti e il ragionamento entrano nel DOM solo quando la relativa sezione viene aperta.

## Dati

`snor.sqlite3` usa WAL con `synchronous=NORMAL` (durevole rispetto a un crash dell'app; in caso di interruzione di corrente si possono perdere gli ultimi commit), chiavi esterne e query parametrizzate. Una connessione scrive e una seconda, in sola lettura, serve liste, ricerche, messaggi ed esportazioni: in WAL i lettori non bloccano lo scrittore, quindi una ricerca non ritarda i checkpoint dello streaming. Il session ID viene aggiornato con `json_set`, senza riscrivere la chat con una copia potenzialmente vecchia. Tabelle: `chats`, `messages`, `projects`, `settings`. I messaggi sono ordinati da una sequenza stabile; gli ID non vengono rigenerati quando un contenuto viene aggiornato. La cancellazione della chat elimina i suoi messaggi nel database Snor, non i transcript originali Claude Code.

I progetti hanno nome, cartella e istruzioni aggiunte al prompt di sistema. La cartella di lavoro non costituisce una sandbox del filesystem.

## Anteprima browser

`isDesktop` distingue la WebView dalla pagina servita da Vite. In browser alcune operazioni UI usano uno storage locale esplicitamente separato. Invio AI, file nativi, login ed esportazione desktop non vengono simulati. I test browser verificano l'interfaccia, non il motore o SQLite.

## Decisioni

| Scelta                      | Motivo                                                        | Conseguenza                                                |
| --------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------- |
| Tauri + Svelte              | UI web compatta e backend compilato                           | Il costo WebView2 va misurato nel totale                   |
| Rust diretto → CLI          | Evitare sidecar Node/Python aggiuntivi                        | Responsabilità propria per la compatibilità del protocollo |
| Un motore alla volta        | Budget di memoria                                             | Nessuna generazione parallela nella v0.1                   |
| SQLite locale               | Persistenza transazionale e nessun servizio                   | Nessuna sincronizzazione cloud                             |
| Idle timeout                | Liberare il runtime inattivo                                  | Il prossimo messaggio può riavviarlo                       |
| Markdown lazy + sanitizzato | Ridurre il percorso iniziale e isolare output non attendibile | Anteprime HTML attive non ancora incluse                   |
