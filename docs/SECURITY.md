# Dati, permessi e confini

## Credenziali

Snor non richiede chiavi API e non importa file di credenziali. Avvia `claude auth login` e usa il login gestito dal programma ufficiale. Il percorso di un eseguibile personalizzato è una scelta dell'utente: selezionare soltanto il binario attendibile installato.

Le variabili API più comuni sono rimosse dal processo agente. Profili, helper e impostazioni amministrate della CLI richiedono comunque verifica: l'app non deve presentare una fatturazione come garantita quando il metodo non è identificato.

## Persistenza

Nel percorso dati dell'app Tauri (`app.snor.desktop` su Windows, sotto AppData) vengono salvati SQLite, WAL e la cartella di lavoro personale. Il percorso preciso è risolto da Tauri, non hard-coded. La CLI mantiene separatamente i propri transcript e le proprie credenziali.

Il database **non è cifrato da Snor**. Usare le protezioni del sistema operativo. Il backup deve includere un database chiuso o uno snapshot SQLite coerente; copiare soltanto il file `.sqlite3` mentre WAL è attivo può perdere dati recenti.

Eliminare una chat Snor non elimina i dati Claude Code né quelli del servizio Anthropic. L'esportazione può contenere dati sensibili: il percorso viene scelto con un dialogo nativo.

## Invio al servizio

I messaggi e gli allegati scelti vengono elaborati tramite Claude Code e i servizi Anthropic. «Locale» indica dove vivono UI e cronologia Snor, non inferenza offline. Il frontend non chiama direttamente endpoint dei modelli.

## Strumenti

Le modalità operative usano il sistema di permessi della CLI. Non viene passato `--dangerously-skip-permissions`. Un working directory non è un isolamento del filesystem: una sandbox forte richiederebbe un'implementazione e una validazione ulteriori.

In Attività/Codice, Claude Code può caricare istruzioni, hook, plugin e MCP dalle cartelle di lavoro e dalle sue impostazioni. Scegliere soltanto progetti fidati. La modalità Chat passa `--safe-mode` e disabilita gli strumenti; le policy amministrate della CLI possono comunque applicarsi. La compatibilità di questi flag va verificata sulla versione reale.

Su Windows un job object termina il processo e i discendenti quando viene chiuso il relativo handle. Un test controllato verifica realmente questo comportamento con Node e un suo processo figlio. Non equivale a limitare i diritti di accesso ai file o alla rete.

Le richieste non implementate vengono rifiutate. Il renderer non può inviare una stringa di shell arbitraria come comando di sistema attraverso un'API generica: sono esposti comandi applicativi specifici. File ed esportazioni sono accessibili tramite IPC dalla finestra locale; la protezione del renderer rimane quindi essenziale.

## Contenuti del modello

Markdown passa attraverso DOMPurify. Script, iframe, oggetti, immagini remote, form e stili arbitrari vengono esclusi. La CSP limita script agli asset locali e le connessioni del renderer all'IPC. Non è incluso un browser generale né un'anteprima di codice HTML con privilegi desktop.

I link HTTP/HTTPS sono aperti nel browser di sistema tramite un comando dedicato; protocolli eseguibili, percorsi file e URL con credenziali sono rifiutati. Il click del Markdown viene intercettato, senza navigare la WebView verso la pagina esterna.

I frame del processo e la memoria per turno hanno limiti espliciti. Le stringhe del modello non vengono interpolate in comandi di shell. Stderr è limitato in memoria; non vengono salvati log integrali del traffico o file di token.

## Prima di distribuire pubblicamente

Completare: prova con Claude Code reale e i suoi strumenti, collaudo dei link esterni nel desktop, firma dei binari, aggiornamenti firmati e revisione del percorso di autenticazione commerciale. Nessuna certificazione o equivalenza di sicurezza con Claude Desktop è dichiarata.
