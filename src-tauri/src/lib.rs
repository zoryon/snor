mod agent;
mod links;
mod process_guard;
mod protocol;
mod startup;
mod store;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::{SystemTime, UNIX_EPOCH},
};
use store::{AppResult, Chat, Store};
use tauri::{Manager, State};

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[tauri::command]
fn open_external(url: String) -> AppResult<()> {
    links::open(&url)
}

#[tauri::command]
fn frontend_ready(startup: State<'_, startup::Startup>) -> Value {
    startup.ready()
}

/// Synchronous Tauri commands run on the main thread: SQLite work goes to the blocking
/// pool so a slow search or a busy write lock never freezes the window.
async fn blocking<T: Send + 'static>(
    db: &State<'_, Arc<Store>>,
    task: impl FnOnce(&Store) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || task(&db))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn bootstrap(db: State<'_, Arc<Store>>) -> AppResult<Value> {
    blocking(&db, |db| {
        Ok(json!({"chats":db.chats("")?,"projects":db.projects()?,"settings":db.settings()}))
    })
    .await
}
#[tauri::command]
async fn list_chats(db: State<'_, Arc<Store>>, search: String) -> AppResult<Vec<Chat>> {
    blocking(&db, move |db| db.chats(&search)).await
}
#[tauri::command]
async fn create_chat(
    db: State<'_, Arc<Store>>,
    model: String,
    mode: String,
    project_id: Option<String>,
) -> AppResult<Chat> {
    validate_model_mode(&model, &mode)?;
    blocking(&db, move |db| {
        let cwd = project_id
            .as_deref()
            .and_then(|id| db.project(id))
            .and_then(|p| p["path"].as_str().map(String::from));
        let chat = Chat {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Nuova conversazione".into(),
            model,
            mode,
            project_id,
            cwd,
            session_id: None,
            pinned: false,
            archived: false,
            created_at: now(),
            updated_at: now(),
        };
        db.put_chat(&chat)?;
        Ok(chat)
    })
    .await
}
fn validate_model_mode(model: &str, mode: &str) -> AppResult<()> {
    if !["sonnet", "opus", "haiku"].contains(&model) {
        return Err("Modello non valido".into());
    }
    if !["chat", "task", "code"].contains(&mode) {
        return Err("Modalità non valida".into());
    }
    Ok(())
}
#[tauri::command]
async fn update_chat(db: State<'_, Arc<Store>>, mut chat: Chat) -> AppResult<()> {
    validate_model_mode(&chat.model, &chat.mode)?;
    if chat.title.trim().is_empty() || chat.title.chars().count() > 200 {
        return Err("Titolo non valido".into());
    }
    blocking(&db, move |db| {
        let original = db.chat(&chat.id)?;
        chat.session_id = original.session_id;
        chat.cwd = original.cwd;
        chat.created_at = original.created_at;
        db.put_chat(&chat)
    })
    .await
}
#[tauri::command]
async fn get_messages(
    db: State<'_, Arc<Store>>,
    chat_id: String,
    before: Option<i64>,
) -> AppResult<Value> {
    blocking(&db, move |db| db.messages(&chat_id, before)).await
}
#[tauri::command]
async fn delete_chat(
    db: State<'_, Arc<Store>>,
    engine: State<'_, agent::Engine>,
    chat_id: String,
) -> AppResult<()> {
    let mut guard = engine.session.lock().await;
    if guard.as_ref().is_some_and(|s| s.chat_id == chat_id) {
        if guard
            .as_ref()
            .is_some_and(|s| s.busy.load(Ordering::Acquire))
        {
            return Err("Interrompi il turno prima di eliminare la chat.".into());
        }
        if let Some(s) = guard.take() {
            let _ = s.tx.send(agent::Input::Shutdown).await;
        }
    }
    blocking(&db, move |db| db.delete_chat(&chat_id)).await
}
#[tauri::command]
async fn save_settings(db: State<'_, Arc<Store>>, settings: Value) -> AppResult<()> {
    blocking(&db, move |db| db.put_settings(&settings)).await
}
#[tauri::command]
async fn save_project(db: State<'_, Arc<Store>>, mut project: Value) -> AppResult<Value> {
    let name = project["name"].as_str().unwrap_or("").trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err("Inserisci un nome di progetto valido.".into());
    }
    let path = PathBuf::from(project["path"].as_str().unwrap_or(""));
    if !path.is_absolute() || !path.is_dir() {
        return Err("Seleziona una cartella esistente.".into());
    }
    if project["instructions"].as_str().unwrap_or("").len() > 32000 {
        return Err("Istruzioni troppo lunghe.".into());
    }
    if project["id"].as_str().unwrap_or("").is_empty() {
        project["id"] = json!(uuid::Uuid::new_v4().to_string());
    }
    blocking(&db, move |db| {
        db.put_project(&project)?;
        Ok(project)
    })
    .await
}
#[tauri::command]
async fn diagnostics(
    db: State<'_, Arc<Store>>,
    engine: State<'_, agent::Engine>,
) -> AppResult<Value> {
    let settings = blocking(&db, |db| Ok(db.settings())).await?;
    let binary = match agent::resolve_binary(settings["claudePath"].as_str()) {
        Ok(p) => p,
        Err(e) => return Ok(json!({"installed":false,"authenticated":false,"error":e})),
    };
    // Both checks start the CLI: run them together instead of one after the other.
    let (version, auth) = tokio::join!(
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            agent::command(&binary).arg("--version").output(),
        ),
        agent::auth_status(&binary)
    );
    let version = match version {
        Ok(Ok(v)) if v.status.success() => String::from_utf8_lossy(&v.stdout)
            .trim()
            .chars()
            .take(160)
            .collect::<String>(),
        _ => {
            return Ok(
                json!({"installed":false,"authenticated":false,"path":binary,"error":"Il binario non risponde a --version."}),
            )
        }
    };
    let (authenticated, method) = match auth {
        Ok(status) => (status.subscription, status.method),
        Err(_) => (false, None),
    };
    if authenticated {
        engine.remember_subscription(&binary);
    } else {
        engine.forget_subscription();
    }
    Ok(
        json!({"installed":true,"authenticated":authenticated,"version":version,"path":binary,"authMethod":method,"error":if authenticated {None} else {Some("Accedi tramite abbonamento con claude auth login. L'autenticazione API o non identificata non viene usata da Snor.")}}),
    )
}
#[tauri::command]
async fn login(db: State<'_, Arc<Store>>, engine: State<'_, agent::Engine>) -> AppResult<()> {
    let settings = blocking(&db, |db| Ok(db.settings())).await?;
    let binary = agent::resolve_binary(settings["claudePath"].as_str())?;
    engine.forget_subscription();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        agent::command(&binary)
            .args(["auth", "login"])
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("ANTHROPIC_AUTH_TOKEN")
            .output(),
    )
    .await
    .map_err(|_| "Login scaduto. Ripeti l'accesso oppure usa claude auth login nel terminale.")?
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Accesso non completato. Prova claude auth login nel terminale.".into())
    }
}
#[tauri::command]
async fn send_prompt(
    app: tauri::AppHandle,
    db: State<'_, Arc<Store>>,
    engine: State<'_, agent::Engine>,
    chat_id: String,
    text: String,
    attachments: Vec<Value>,
) -> AppResult<Value> {
    if text.trim().is_empty() && attachments.is_empty() {
        return Err("Scrivi un messaggio.".into());
    }
    let payload: usize = attachments
        .iter()
        .map(|a| a["data"].as_str().map_or(0, str::len))
        .sum();
    if text.len() > 128 * 1024 || attachments.len() > 4 || payload > 8 * 1024 * 1024 {
        return Err("Messaggio o allegati troppo grandi.".into());
    }
    let mut chat = db.chat(&chat_id)?;
    let settings = db.settings();
    let mut guard = engine.session.lock().await;
    if guard
        .as_ref()
        .is_some_and(|s| s.busy.load(Ordering::Acquire))
    {
        return Err(
            "Attendi o interrompi il turno in corso. Snor mantiene un solo motore attivo.".into(),
        );
    }
    let signature = format!("{}:{}", chat.model, chat.mode);
    if guard
        .as_ref()
        .is_some_and(|s| s.chat_id != chat_id || s.tx.is_closed() || s.signature != signature)
    {
        if let Some(old) = guard.take() {
            let _ = old.tx.send(agent::Input::Shutdown).await;
        }
    }
    if guard.is_none() {
        let binary = agent::resolve_binary(settings["claudePath"].as_str())?;
        engine.require_subscription(&binary).await?;
        let workspace = if let Some(cwd) = &chat.cwd {
            PathBuf::from(cwd)
        } else {
            app.path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("workspace")
        };
        std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
        let instructions = chat
            .project_id
            .as_deref()
            .and_then(|id| db.project(id))
            .and_then(|p| p["instructions"].as_str().map(String::from))
            .unwrap_or_default();
        *guard =
            Some(agent::start(app.clone(), chat.clone(), binary, workspace, instructions).await?);
    }
    let session = guard.as_ref().ok_or("Sessione non disponibile")?;
    let user = json!({"id":uuid::Uuid::new_v4().to_string(),"role":"user","text":text,"attachments":attachments.iter().map(|a|json!({"id":a["id"],"name":a["name"],"kind":a["kind"],"size":a["size"]})).collect::<Vec<_>>(),"status":"complete","createdAt":now()});
    let assistant = json!({"id":uuid::Uuid::new_v4().to_string(),"role":"assistant","text":"","thinking":"","status":"streaming","createdAt":now()});
    db.put_message(&chat_id, &user)?;
    db.put_message(&chat_id, &assistant)?;
    if chat.title == "Nuova conversazione" {
        chat.title = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(64)
            .collect();
        if chat.title.is_empty() {
            chat.title = "Conversazione con allegati".into();
        }
    }
    chat.updated_at = now();
    db.put_chat(&chat)?;
    session.busy.store(true, Ordering::Release);
    if session
        .tx
        .send(agent::Input::Send {
            text,
            attachments,
            message_id: assistant["id"].as_str().unwrap().into(),
        })
        .await
        .is_err()
    {
        session.busy.store(false, Ordering::Release);
        let error = "Claude Code si è chiuso. Riprova il messaggio.";
        let mut failed = assistant;
        failed["status"] = json!("error");
        failed["error"] = json!(error);
        let _ = db.put_message(&chat_id, &failed);
        return Err(error.into());
    }
    Ok(json!({"chat":chat,"user":user,"assistant":assistant}))
}
#[tauri::command]
async fn interrupt(engine: State<'_, agent::Engine>, chat_id: String) -> AppResult<()> {
    let guard = engine.session.lock().await;
    let session = guard
        .as_ref()
        .filter(|s| s.chat_id == chat_id)
        .ok_or("Nessuna sessione attiva")?;
    session
        .tx
        .send(agent::Input::Interrupt)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn respond_permission(
    engine: State<'_, agent::Engine>,
    chat_id: String,
    request_id: String,
    allow: bool,
) -> AppResult<()> {
    let guard = engine.session.lock().await;
    let session = guard
        .as_ref()
        .filter(|s| s.chat_id == chat_id)
        .ok_or("Nessuna sessione attiva")?;
    session
        .tx
        .send(agent::Input::Permission {
            id: request_id,
            allow,
        })
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn read_attachment(path: String) -> AppResult<Value> {
    let path = PathBuf::from(path);
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > 4 * 1024 * 1024 {
        return Err("Ogni allegato deve essere inferiore a 4 MiB.".into());
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let image = match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    };
    if image.is_none() && size > 256 * 1024 {
        return Err("I file di testo devono essere inferiori a 256 KiB.".into());
    }
    let data = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    if data.len() > 4 * 1024 * 1024 {
        return Err("Allegato troppo grande.".into());
    }
    let (kind, mime, content) = if let Some(mime) = image {
        ("image", mime, STANDARD.encode(&data))
    } else {
        if data.len() > 256 * 1024 {
            return Err("I file di testo devono essere inferiori a 256 KiB.".into());
        }
        ("text","text/plain",String::from_utf8(data).map_err(|_|"Formato non supportato. Usa immagini o file di testo UTF-8; PDF e Office arriveranno in una fase successiva.")?)
    };
    Ok(
        json!({"id":uuid::Uuid::new_v4().to_string(),"name":path.file_name().unwrap_or_default().to_string_lossy(),"kind":kind,"mime":mime,"data":content,"size":size}),
    )
}
#[tauri::command]
async fn export_chat(
    db: State<'_, Arc<Store>>,
    chat_id: String,
    path: String,
    format: String,
) -> AppResult<()> {
    let data = blocking(&db, move |db| db.export(&chat_id)).await?;
    let content = if format == "markdown" {
        let mut text = format!(
            "# {}\n\n",
            data["chat"]["title"].as_str().unwrap_or("Conversazione")
        );
        if let Some(messages) = data["messages"].as_array() {
            for m in messages {
                text.push_str(&format!(
                    "## {}\n\n{}\n\n",
                    if m["role"] == "user" { "Tu" } else { "Claude" },
                    m["text"].as_str().unwrap_or("")
                ));
            }
        }
        text
    } else {
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
    };
    tokio::fs::write(path, content)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup = startup::Startup::new();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(agent::Engine::default())
        .manage(startup)
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Store::open(&dir.join("snor.sqlite3")).map_err(std::io::Error::other)?;
            app.manage(Arc::new(db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            list_chats,
            create_chat,
            update_chat,
            get_messages,
            delete_chat,
            save_settings,
            save_project,
            diagnostics,
            login,
            send_prompt,
            interrupt,
            respond_permission,
            read_attachment,
            export_chat,
            open_external,
            frontend_ready
        ])
        .run(tauri::generate_context!())
        .expect("Errore nell'avvio di Snor");
}
