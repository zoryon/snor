# Changelog

## Non rilasciato

Correzioni:

- Lettura di stdout in un task dedicato: un frame grande ricevuto a pezzi non può più essere troncato quando un altro ramo dell'attore (checkpoint, stderr, input) si attiva a metà lettura.
- stderr svuotato sempre, anche se non UTF-8 o con righe molto lunghe: la CLI non può più bloccarsi su una pipe piena.
- Un doppio clic o un clic tardivo su un'approvazione non chiude più la sessione; i pulsanti si disattivano durante l'invio.
- Un errore a sessione inattiva non sovrascrive più come «errore» l'ultima risposta già completata; i messaggi accettati ma mai consegnati vengono marcati come falliti invece di lasciare la UI in attesa.
- Lo stop prima della fine dell'avvio annulla il messaggio in attesa; dopo lo stop le nuove richieste di strumenti vengono rifiutate.
- Gli errori di fine turno mostrano il messaggio della CLI invece di `null`.
- I testi di più messaggi nello stesso turno sono separati da una riga vuota.
- Il session ID viene salvato senza sovrascrivere una rinomina o un preferito fatti nel frattempo.
- Cambiare chat mentre un invio è in corso non attribuisce più la risposta alla conversazione sbagliata.
- Se la consegna alla CLI fallisce, la risposta vuota viene salvata come errore invece di restare «in corso».
- Le bozze vuote e quelle delle chat eliminate vengono rimosse da localStorage; i titoli automatici non contengono più a capo.

Prestazioni:

- Delta accorpati in Rust ogni 50 ms prima dell'IPC; nel frontend viene aggiornato solo il messaggio interessato.
- Card dei messaggi con rune Svelte 5: i messaggi invariati non vengono rivalutati durante lo streaming. Markdown della risposta in streaming rianalizzato al massimo ogni 90 ms; chunk Markdown caricato una volta e prelevato a UI inattiva.
- Strumenti e ragionamento entrano nel DOM solo quando aperti.
- Comandi SQLite fuori dal thread principale, connessione di lettura separata, `synchronous=NORMAL`, indice su `updated_at`, ricerca con `EXISTS`, statement in cache.
- Checkpoint solo se il contenuto è cambiato; buffer del turno liberati a fine risposta; allegati spostati invece di copiati; input degli strumenti oltre 32 KiB non duplicati.
- Diagnostica con `--version` e `auth status` in parallelo; verifica dell'abbonamento riutilizzata per 10 minuti nei riavvii di sessione.

## 0.1.0 — 21 ottobre 2026

Prima implementazione desktop: Tauri 2, Rust, Svelte 5, SQLite; UI italiana con temi, chat e progetti, bozze, ricerca, archivio ed esportazione. Collegamento diretto al processo Claude Code con streaming JSON, handshake, ripresa sessione, strumenti e approvazioni puntuali. Allegati immagini/testo e diagnostica del login. Su Windows, gestione del ciclo di vita dei processi attraverso un job object.

Documentazione di architettura, sviluppo, protocollo, dati, prestazioni e roadmap; test frontend, Rust e browser; script di misurazione e schermate dell'interfaccia.

Non ancora una replica completa di Claude Desktop. L'integrazione con un account Claude reale deve essere verificata separatamente, così come il budget di 200 MB comprendente il motore. Vedere `docs/VERIFICATION.md`.
