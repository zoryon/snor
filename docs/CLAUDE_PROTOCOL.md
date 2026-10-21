# Integrazione Claude Code

## Contratto e stato

Le opzioni CLI JSON sono documentate pubblicamente. Le forme dettagliate dei frame di controllo sono basate sui sorgenti dell'SDK ufficiale Anthropic; non vanno considerate un ABI stabile. Verificare la compatibilità con la versione CLI installata. Questa prima implementazione non afferma di aver superato una conversazione reale finché ciò non è riportato in `VERIFICATION.md`.

## Avvio

```text
claude -p
  --input-format stream-json
  --output-format stream-json
  --verbose
  --include-partial-messages
  --permission-mode default
  --permission-prompt-tool stdio
  --model sonnet
```

Gli argomenti sono passati come array a un processo, mai concatenati a una stringa di shell. Prompt e allegati transitano in JSON su stdin. Il modello usa alias della CLI, non nomi di versione hard-coded. Una sessione ripresa aggiunge `--resume ID`.

`--permission-prompt-tool stdio` appartiene al collegamento di controllo usato dall'SDK; il riferimento CLI pubblico descrive principalmente strumenti MCP per la stessa opzione. Serve una prova di compatibilità sulla CLI reale.

Modalità Chat: strumenti disabilitati con `--tools ""`, personalizzazioni disabilitate con `--safe-mode`, che a differenza di bare mode mantiene l'autenticazione. Modalità Attività/Codice: strumenti della CLI e richieste di permesso, senza bypass. L'app aggiunge istruzioni in italiano e quelle del progetto tramite `--append-system-prompt`. Le versioni CLI che non supportano questi flag non sono compatibili con questa prima implementazione.

Le variabili `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL` e `CLAUDECODE` vengono rimosse dall'ambiente del processo agente. Questo non sostituisce la verifica dell'autenticazione effettiva: configurazioni CLI e profili possono influenzarla.

Non usare `--bare` per questo percorso: disabilita la lettura delle credenziali OAuth dell'abbonamento.

## Handshake

```json
{
  "type": "control_request",
  "request_id": "init-id",
  "request": { "subtype": "initialize", "hooks": {} }
}
```

Attendere il `control_response` con lo stesso ID prima di inviare il primo messaggio. Timeout: 60 secondi. Il frame `system/init` fornisce session ID, modello e strumenti. Il session ID viene salvato nel database.

## Messaggi utente

```json
{
  "type": "user",
  "message": { "role": "user", "content": [{ "type": "text", "text": "Ciao" }] },
  "parent_tool_use_id": null
}
```

Immagini: blocchi `image` con sorgente base64 e MIME. File di testo: blocchi di testo aggiuntivi chiaramente etichettati. Il nome dell'allegato non equivale a un'istruzione di sistema attendibile.

## Eventi di risposta

- `stream_event`: `content_block_delta`, con delta testo o ragionamento.
- `assistant`: snapshot finale dei blocchi, usato anche se non sono arrivati delta; tool call con ID.
- `user`: risultati degli strumenti correlati dal tool ID.
- `result`: conclusione del turno, stato, consumo e possibili errori. Uno stream iniziato non è una risposta completata.
- `control_request`: approvazioni, gestite separatamente dai contenuti del modello.

Il testo dei subagenti non viene mescolato automaticamente con quello dell'assistente principale. Un `content_block_start` di testo o ragionamento quando il turno ha già del contenuto inserisce una riga vuota, così i messaggi successivi dello stesso turno non vengono concatenati. Le righe vuote su stdout vengono ignorate.

Un `result` con errore riporta il contenuto di `errors`, altrimenti `result`, altrimenti il sottotipo; non mostra `null`.

## Approvazioni

Una richiesta `can_use_tool` conserva in Rust l'input originale associato al request ID. Il frontend può rispondere con ID e allow/deny, senza modificare l'input dell'azione. La risposta ha il request ID **dentro** `response`:

```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "permission-id",
    "response": { "behavior": "allow", "updatedInput": { "command": "..." } }
  }
}
```

I rifiuti usano `behavior: deny` e una motivazione. Richieste non riconosciute ricevono un errore, non un'approvazione implicita. Dopo una richiesta di stop le nuove richieste `can_use_tool` vengono rifiutate. Una risposta dell'utente per un ID non più pendente (già risposto, annullato o interrotto) viene ignorata e la UI riceve `permission_cancelled`; non viene inoltrata alla CLI. Le approvazioni persistenti e il form di `AskUserQuestion` sono successivi alla v0.1.

## Limiti

- Frame massimo: 4 MiB, verificato durante la lettura prima di accumularne altri. Al massimo 8 frame decodificati in coda verso l'attore.
- stderr: svuotato in continuo, ultima riga conservata (massimo 2 KiB letti, 500 caratteri mostrati).
- Input di uno strumento oltre 32 KiB: sostituito da una nota nella UI e nel database; l'originale resta in Rust per la risposta di approvazione.
- Testo + ragionamento per turno: 2 MiB.
- Fino a 16 richieste di permesso pendenti.
- Quattro allegati, 4 MiB per file e 8 MiB di payload JSON totale; testo UTF-8 massimo 256 KiB per file.
- Scrittura su stdin limitata da timeout di 5 secondi.
- Un processo inattivo termina dopo 300 secondi.

I limiti locali non corrispondono ai limiti del modello o dell'abbonamento.

## Matrice di collaudo reale

Da eseguire con account dell'utente: login, primo turno, secondo turno nello stesso processo, ripresa dopo riavvio, immagine, testo allegato, permesso consentito/rifiutato, stop durante streaming, stop durante un tool, scadenza login, quota esaurita, modello non disponibile e CLI che esce prematuramente.
