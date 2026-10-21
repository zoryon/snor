# Specifica prodotto

## Obiettivo

Un workspace desktop Tauri per usare Claude Code con il proprio abbonamento, con un'esperienza di chat curata e rapida, cronologia locale e piena visibilità delle azioni. Windows x64 è la prima piattaforma di verifica. «Paragonabile a Claude Desktop» è la direzione di prodotto, non una dichiarazione di equivalenza della v0.1.

## Requisiti di esperienza

1. **Interfaccia:** sidebar, conversazioni e progetti, pannello contestuale, temi, tastiera e accessibilità.
2. **Chat:** streaming, Markdown, copia, modifica/reinvio, rigenerazione e rami, stop, errori e ripresa.
3. **Modalità:** conversazione senza strumenti, attività su file e lavoro sul codice.
4. **Account:** login ufficiale Claude Code, diagnostica, selezione dei modelli e limiti quando esposti.
5. **Archivio:** ricerca, preferiti, organizzazione, cronologia paginata, import/export e recupero.
6. **Progetti:** cartelle, istruzioni, conoscenza di riferimento e memoria controllabile.
7. **Allegati:** immagini, screenshot, testo, PDF e documenti con anteprime e limiti chiari.
8. **Artefatti:** documenti e codice affiancati, versioni, modifiche selettive, esportazione e preview isolata.
9. **Azioni:** tool card, risultati, approvazioni puntuali e regole, domande strutturate e stop.
10. **Codice:** file, editor, diff, terminale su richiesta, Git, worktree, test e ripristino.
11. **Estensioni:** MCP, skill, plugin e integrazioni browser/computer con confini espliciti.
12. **Attività:** coda, sessioni multiple, background, notifiche, pianificazione e limiti di concorrenza.
13. **Desktop:** richiamo rapido, tray, file esterni, dettatura come estensione.
14. **Affidabilità:** dati transazionali, backup, diagnostica, manutenzione dello spazio, aggiornamenti firmati.

## Differenze dai servizi Anthropic

La cronologia, memoria, progetti e artefatti di Snor sono dati propri dell'app. Non è previsto accesso non documentato alla cronologia claude.ai. I servizi ufficiali cloud, gli editor collaborativi e la pubblicazione degli artefatti non sono automaticamente disponibili tramite la CLI. Il modello può rispondere diversamente da Claude Chat, perché viene eseguito nel contesto e nelle istruzioni di Claude Code.

## Priorità

La base deve prima dimostrare un ciclo completo affidabile: aprire, trovare la CLI, autenticarsi, inviare due turni nello stesso processo, approvare/rifiutare un'azione, interrompere e riprendere, chiudere e ritrovare i dati. Le funzioni avanzate vengono aggiunte mantenendo il caricamento su richiesta e verificando il costo reale.

## Direzione visiva

Toni caldi, accento terracotta, titoli serif e testo di lettura sobrio. Asset locali e font di sistema; nessuna dipendenza da font remoti. L'interfaccia deve essere chiara anche quando la CLI manca, l'account non è connesso o la quota non è disponibile. Vietati risposte dimostrative presentate come vere e indicatori di connessione inventati.
