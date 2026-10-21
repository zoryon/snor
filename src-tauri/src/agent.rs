use crate::{
    protocol,
    store::{AppResult, Chat, Store},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex,
    },
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{mpsc, Mutex},
    time::{timeout, Duration, Instant, MissedTickBehavior},
};

/// Text and reasoning deltas are coalesced before crossing IPC: one event per window
/// instead of one per token.
const DELTA_WINDOW: Duration = Duration::from_millis(50);
/// A successful subscription check is reused when a session restarts within this window.
const AUTH_TTL: Duration = Duration::from_secs(600);
const IDLE: Duration = Duration::from_secs(300);
/// Larger tool inputs stay in Rust for the permission round-trip but are not shown or stored.
const MAX_TOOL_INPUT: usize = 32 * 1024;
const MAX_TOOL_OUTPUT: usize = 16 * 1024;

#[derive(Default)]
pub struct Engine {
    pub session: Mutex<Option<Session>>,
    subscription: StdMutex<Option<(String, Instant)>>,
}
pub struct Session {
    pub chat_id: String,
    pub signature: String,
    pub busy: Arc<AtomicBool>,
    pub tx: mpsc::Sender<Input>,
}
pub enum Input {
    Send {
        text: String,
        attachments: Vec<Value>,
        message_id: String,
    },
    Permission {
        id: String,
        allow: bool,
    },
    Interrupt,
    Shutdown,
}

pub fn command(path: &str) -> Command {
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd.kill_on_drop(true);
    cmd
}

pub fn resolve_binary(custom: Option<&str>) -> AppResult<String> {
    if let Some(p) = custom.filter(|s| !s.trim().is_empty()) {
        let path = PathBuf::from(p);
        if !path.is_file() {
            return Err("Il percorso di Claude Code non è un file esistente.".into());
        }
        if cfg!(windows)
            && path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| !s.eq_ignore_ascii_case("exe"))
                .unwrap_or(true)
        {
            return Err("Seleziona il binario nativo claude.exe, non uno script .cmd/.ps1.".into());
        }
        return Ok(path.to_string_lossy().to_string());
    }
    let exe = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        dirs.push(PathBuf::from(&home).join(".local/bin"));
        dirs.push(PathBuf::from(home).join(".claude/bin"));
    }
    dirs.into_iter().map(|p| p.join(exe)).find(|p| p.is_file()).map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Claude Code non trovato. Installa il binario ufficiale e accedi, oppure seleziona claude.exe nelle impostazioni.".into())
}

pub struct AuthStatus {
    pub subscription: bool,
    pub method: Option<String>,
    parsed: bool,
}

pub async fn auth_status(binary: &str) -> AppResult<AuthStatus> {
    let output = timeout(
        Duration::from_secs(15),
        command(binary)
            .args(["auth", "status"])
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("ANTHROPIC_AUTH_TOKEN")
            .env_remove("ANTHROPIC_BASE_URL")
            .output(),
    )
    .await
    .map_err(|_| "Verifica dell'abbonamento scaduta.")?
    .map_err(|e| e.to_string())?;
    let status = serde_json::from_slice::<Value>(&output.stdout).ok();
    let method = status
        .as_ref()
        .and_then(|v| v["authMethod"].as_str())
        .map(String::from);
    Ok(AuthStatus {
        subscription: output.status.success()
            && matches!(method.as_deref(), Some("claude.ai" | "oauth_token")),
        parsed: status.is_some(),
        method,
    })
}

impl Engine {
    pub fn remember_subscription(&self, binary: &str) {
        if let Ok(mut cached) = self.subscription.lock() {
            *cached = Some((binary.to_string(), Instant::now()));
        }
    }

    pub fn forget_subscription(&self) {
        if let Ok(mut cached) = self.subscription.lock() {
            *cached = None;
        }
    }

    /// Every session requires a verified subscription login. The check spawns the CLI,
    /// so a recent positive result is reused instead of paying for it on each restart.
    pub async fn require_subscription(&self, binary: &str) -> AppResult<()> {
        let fresh = self.subscription.lock().is_ok_and(|cached| {
            cached
                .as_ref()
                .is_some_and(|(path, at)| path == binary && at.elapsed() < AUTH_TTL)
        });
        if fresh {
            return Ok(());
        }
        let status = auth_status(binary).await?;
        if !status.parsed {
            return Err("La CLI non espone uno stato di autenticazione JSON compatibile.".into());
        }
        if status.subscription {
            self.remember_subscription(binary);
            Ok(())
        } else {
            self.forget_subscription();
            Err("Snor richiede l'accesso con abbonamento. Esegui claude auth login e scegli il tuo account Claude.".into())
        }
    }
}

async fn write(stdin: &mut ChildStdin, frame: &Value) -> AppResult<()> {
    let mut data = serde_json::to_vec(frame).map_err(|e| e.to_string())?;
    data.push(b'\n');
    timeout(Duration::from_secs(5), async {
        stdin.write_all(&data).await?;
        stdin.flush().await
    })
    .await
    .map_err(|_| "Claude Code non legge l'input: scrittura scaduta.".to_string())?
    .map_err(|e| e.to_string())
}

pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> AppResult<Option<String>> {
    let mut bytes = Vec::new();
    loop {
        let buf = reader.fill_buf().await.map_err(|e| e.to_string())?;
        if buf.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                String::from_utf8(bytes)
                    .map(Some)
                    .map_err(|e| e.to_string())
            };
        }
        let end = buf
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(buf.len());
        if bytes.len() + end > protocol::MAX_FRAME {
            return Err("Output Claude Code oltre il limite di 4 MiB per evento.".into());
        }
        let done = buf[end - 1] == b'\n';
        bytes.extend_from_slice(&buf[..end]);
        reader.consume(end);
        if done {
            return String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| e.to_string());
        }
    }
}

/// Owns stdout. `read_frame` keeps partial lines in a local buffer, so it must not race
/// in a `select!`: a winning branch would drop the bytes already consumed.
async fn read_frames<R: AsyncRead + Unpin>(reader: R, tx: mpsc::Sender<AppResult<Value>>) {
    let mut reader = BufReader::with_capacity(64 * 1024, reader);
    loop {
        let item = match read_frame(&mut reader).await {
            Ok(None) => return,
            Ok(Some(line)) if line.trim().is_empty() => continue,
            Ok(Some(line)) => serde_json::from_str(&line)
                .map_err(|e| format!("Protocollo Claude Code non valido: {e}")),
            Err(e) => Err(e),
        };
        let failed = item.is_err();
        if tx.send(item).await.is_err() || failed {
            return;
        }
    }
}

/// Drains stderr until EOF so the CLI never blocks on a full pipe. Keeps only the last
/// non-empty line, bounded and decoded lossily, for error messages.
async fn drain_stderr<R: AsyncRead + Unpin>(reader: R, last: Arc<StdMutex<String>>) {
    const KEEP: usize = 2048;
    let mut reader = BufReader::new(reader);
    let mut line = Vec::with_capacity(256);
    let keep = |line: &[u8]| {
        let text = String::from_utf8_lossy(line);
        let text = text.trim();
        if !text.is_empty() {
            if let Ok(mut last) = last.lock() {
                *last = text.chars().take(500).collect();
            }
        }
    };
    loop {
        let consumed = match reader.fill_buf().await {
            Ok(buf) if !buf.is_empty() => {
                for &byte in buf {
                    if byte == b'\n' {
                        keep(&line);
                        line.clear();
                    } else if line.len() < KEEP {
                        line.push(byte);
                    }
                }
                buf.len()
            }
            _ => break,
        };
        reader.consume(consumed);
    }
    keep(&line);
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentEvent<'a> {
    chat_id: &'a str,
    message_id: &'a str,
    kind: &'a str,
    data: Value,
}

fn emit(app: &AppHandle, chat_id: &str, message_id: &str, kind: &str, data: Value) {
    let _ = app.emit(
        "agent:event",
        AgentEvent {
            chat_id,
            message_id,
            kind,
            data,
        },
    );
}

/// Coalesces streamed text and reasoning. Every other event flushes the pending deltas
/// first, so the UI receives them in the order Claude Code produced them.
struct Outbox {
    app: AppHandle,
    chat_id: String,
    text: String,
    thinking: String,
}

impl Outbox {
    fn has_pending(&self) -> bool {
        !self.text.is_empty() || !self.thinking.is_empty()
    }

    fn flush(&mut self, message_id: &str) {
        if !self.thinking.is_empty() {
            let data = Value::String(std::mem::take(&mut self.thinking));
            emit(&self.app, &self.chat_id, message_id, "thinking", data);
        }
        if !self.text.is_empty() {
            let data = Value::String(std::mem::take(&mut self.text));
            emit(&self.app, &self.chat_id, message_id, "delta", data);
        }
    }

    fn emit(&mut self, message_id: &str, kind: &str, data: Value) {
        self.flush(message_id);
        emit(&self.app, &self.chat_id, message_id, kind, data);
    }
}

/// State of the assistant reply being produced.
#[derive(Default)]
struct Turn {
    message_id: String,
    active: bool,
    interrupted: bool,
    /// Changed since the last checkpoint.
    dirty: bool,
    text: String,
    thinking: String,
    /// Text blocks of final snapshots, used only when no deltas arrived.
    final_text: Vec<String>,
    tools: Vec<Value>,
}

impl Turn {
    fn begin(&mut self, message_id: String) {
        *self = Turn {
            message_id,
            active: true,
            ..Default::default()
        };
    }

    /// Keeps the ID for the final `closed` event and releases the buffers of the reply.
    fn finish(&mut self) {
        let message_id = std::mem::take(&mut self.message_id);
        *self = Turn {
            message_id,
            ..Default::default()
        };
    }

    fn message(&self, status: &str) -> Value {
        json!({"id":self.message_id,"role":"assistant","text":self.text,"thinking":self.thinking,"tools":self.tools,"status":status,"createdAt":crate::now()})
    }
}

fn needs_break(text: &str) -> bool {
    !text.is_empty() && !text.ends_with("\n\n")
}

fn ui_tool(block: &Value) -> Value {
    let mut tool = block.clone();
    let size = tool["input"].to_string().len();
    if size > MAX_TOOL_INPUT {
        tool["input"] = json!({"nota": format!("Input di {} KiB omesso dalla UI.", size / 1024)});
    }
    tool["status"] = json!("running");
    tool
}

fn result_error(frame: &Value) -> String {
    let listed = frame["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .map(|e| {
                    e.as_str()
                        .map(String::from)
                        .unwrap_or_else(|| e.to_string())
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.trim().is_empty());
    let message = listed
        .or_else(|| {
            frame["result"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
        })
        .unwrap_or_else(|| {
            format!(
                "Claude Code ha concluso il turno con un errore ({}).",
                frame["subtype"].as_str().unwrap_or("sconosciuto")
            )
        });
    message.chars().take(2000).collect()
}

/// Marks a message that never reached Claude Code as failed.
fn reject(store: &Store, out: &mut Outbox, chat_id: &str, message_id: &str, error: &str) {
    let message = json!({"id":message_id,"role":"assistant","text":"","status":"error","createdAt":crate::now(),"error":error});
    let _ = store.put_message(chat_id, &message);
    out.emit(message_id, "failure", json!(error));
}

async fn begin_turn(
    turn: &mut Turn,
    out: &mut Outbox,
    stdin: &mut ChildStdin,
    message_id: String,
    prompt: String,
    attachments: Vec<Value>,
) -> AppResult<()> {
    turn.begin(message_id);
    out.emit(&turn.message_id, "status", json!("running"));
    write(stdin, &protocol::user_message(&prompt, attachments)).await
}

pub async fn start(
    app: AppHandle,
    chat: Chat,
    binary: String,
    safe_cwd: PathBuf,
    instructions: String,
) -> AppResult<Session> {
    let mut cmd = command(&binary);
    cmd.args([
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--permission-mode",
        "default",
        "--permission-prompt-tool",
        "stdio",
        "--model",
        &chat.model,
    ]);
    if chat.mode == "chat" {
        cmd.args(["--tools", "", "--safe-mode"]);
    }
    if let Some(session) = &chat.session_id {
        cmd.args(["--resume", session]);
    }
    let prompt = if chat.mode == "chat" {
        format!("Sei l'assistente di Snor. Rispondi nella lingua dell'utente. Questa è una conversazione senza strumenti. Non dichiarare azioni mai eseguite. {instructions}")
    } else {
        format!("Sei l'assistente di Snor. Rispondi nella lingua dell'utente. Richiedi le approvazioni per le operazioni che lo richiedono. {instructions}")
    };
    cmd.args(["--append-system-prompt", &prompt]);
    // Subscription is the explicit product choice. Do not inherit API billing overrides.
    for key in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "CLAUDECODE",
    ] {
        cmd.env_remove(key);
    }
    let mut child = cmd
        .current_dir(safe_cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Avvio Claude Code fallito: {e}"))?;
    let process_guard = crate::process_guard::ProcessGuard::attach(&child)?;
    let mut stdin = child.stdin.take().ok_or("stdin non disponibile")?;
    let stdout = child.stdout.take().ok_or("stdout non disponibile")?;
    let stderr = child.stderr.take().ok_or("stderr non disponibile")?;
    let last_stderr = Arc::new(StdMutex::new(String::new()));
    tauri::async_runtime::spawn(drain_stderr(stderr, last_stderr.clone()));
    let (frame_tx, mut frames) = mpsc::channel(8);
    tauri::async_runtime::spawn(read_frames(stdout, frame_tx));
    let init_id = uuid::Uuid::new_v4().to_string();
    write(&mut stdin, &protocol::initialize(&init_id)).await?;
    let (tx, mut rx) = mpsc::channel::<Input>(16);
    let chat_id = chat.id.clone();
    let busy_flag = Arc::new(AtomicBool::new(false));
    let session = Session {
        chat_id: chat_id.clone(),
        signature: format!("{}:{}", chat.model, chat.mode),
        busy: busy_flag.clone(),
        tx,
    };
    tauri::async_runtime::spawn(async move {
        let store = app.state::<Arc<Store>>().inner().clone();
        let mut out = Outbox {
            app: app.clone(),
            chat_id: chat_id.clone(),
            text: String::new(),
            thinking: String::new(),
        };
        let mut turn = Turn::default();
        let mut initialized = false;
        // A prompt received before the handshake: (message ID, text, attachments).
        let mut pending: Option<(String, String, Vec<Value>)> = None;
        let mut permissions = HashMap::<String, Value>::new();
        let init_timeout = tokio::time::sleep(Duration::from_secs(60));
        tokio::pin!(init_timeout);
        let idle = tokio::time::sleep(IDLE);
        tokio::pin!(idle);
        let interrupt_deadline = tokio::time::sleep(Duration::from_secs(8));
        tokio::pin!(interrupt_deadline);
        let flush_deadline = tokio::time::sleep(DELTA_WINDOW);
        tokio::pin!(flush_deadline);
        let mut checkpoint = tokio::time::interval(Duration::from_secs(2));
        checkpoint.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let stderr_tail = || last_stderr.lock().map(|s| s.clone()).unwrap_or_default();
        let outcome: AppResult<()> = async {
            loop {
                tokio::select! {
                    _ = &mut init_timeout, if !initialized => return Err("Claude Code non ha completato l'inizializzazione entro 60 secondi.".into()),
                    _ = &mut interrupt_deadline, if turn.interrupted && turn.active => return Err("Processo arrestato dopo la richiesta di interruzione.".into()),
                    _ = &mut idle, if initialized && !turn.active => break,
                    _ = &mut flush_deadline, if out.has_pending() => out.flush(&turn.message_id),
                    _ = checkpoint.tick(), if turn.active && turn.dirty => {
                        store.put_message(&chat_id, &turn.message("streaming"))?;
                        turn.dirty = false;
                    },
                    input = rx.recv() => match input {
                        Some(Input::Shutdown) | None => break,
                        Some(Input::Interrupt) => {
                            if let Some((id, _, _)) = pending.take() {
                                let message = json!({"id":id,"role":"assistant","text":"","status":"interrupted","createdAt":crate::now()});
                                store.put_message(&chat_id, &message)?;
                                busy_flag.store(false, Ordering::Release);
                                out.emit(&id, "done", message);
                            } else if turn.active && !turn.interrupted {
                                turn.interrupted = true;
                                interrupt_deadline.as_mut().reset(Instant::now() + Duration::from_secs(8));
                                write(&mut stdin, &protocol::control(&uuid::Uuid::new_v4().to_string(), "interrupt")).await?;
                                for (id, original) in permissions.drain() {
                                    write(&mut stdin, &protocol::permission_response(&id, original, false)).await?;
                                }
                            }
                        },
                        Some(Input::Permission { id, allow }) => match permissions.remove(&id) {
                            Some(original) => write(&mut stdin, &protocol::permission_response(&id, original, allow)).await?,
                            // Already answered, cancelled or interrupted: a late click must not end the session.
                            None => out.emit(&turn.message_id, "permission_cancelled", json!({"id": id})),
                        },
                        Some(Input::Send { text, attachments, message_id }) => {
                            if turn.active || pending.is_some() {
                                reject(&store, &mut out, &chat_id, &message_id, "Un turno è già in corso.");
                            } else if !initialized {
                                pending = Some((message_id, text, attachments));
                            } else {
                                begin_turn(&mut turn, &mut out, &mut stdin, message_id, text, attachments).await?;
                            }
                        },
                    },
                    frame = frames.recv() => {
                        let Some(frame) = frame else {
                            let status = child.wait().await.map_err(|e| e.to_string())?;
                            if turn.active || pending.is_some() || !initialized {
                                return Err(format!("Claude Code si è chiuso ({status}). {}", stderr_tail()));
                            }
                            break;
                        };
                        let frame = frame?;
                        let was_pending = out.has_pending();
                        match frame["type"].as_str().unwrap_or("") {
                            "control_response" => {
                                if frame["response"]["request_id"] == init_id {
                                    if frame["response"]["subtype"] == "error" {
                                        return Err(frame["response"]["error"].as_str().unwrap_or("Inizializzazione rifiutata").into());
                                    }
                                    initialized = true;
                                    if let Some((id, text, attachments)) = pending.take() {
                                        begin_turn(&mut turn, &mut out, &mut stdin, id, text, attachments).await?;
                                    }
                                }
                            },
                            "system" if frame["subtype"] == "init" => {
                                if let Some(id) = frame["session_id"].as_str() {
                                    let _ = store.set_session(&chat_id, id);
                                }
                                out.emit(&turn.message_id, "init", json!({"model":frame["model"],"sessionId":frame["session_id"],"tools":frame["tools"]}));
                            },
                            "control_request" => {
                                let id = frame["request_id"].as_str().unwrap_or("");
                                if turn.active && !turn.interrupted && frame["request"]["subtype"] == "can_use_tool" {
                                    if permissions.len() >= 16 {
                                        return Err("Troppe richieste di approvazione simultanee.".into());
                                    }
                                    let input = frame["request"]["input"].clone();
                                    out.emit(&turn.message_id, "permission", json!({"id":id,"tool":frame["request"]["tool_name"],"input":input}));
                                    permissions.insert(id.to_string(), input);
                                } else if turn.interrupted && frame["request"]["subtype"] == "can_use_tool" {
                                    write(&mut stdin, &protocol::permission_response(id, Value::Null, false)).await?;
                                } else {
                                    write(&mut stdin, &protocol::unsupported_response(id)).await?;
                                }
                            },
                            "control_cancel_request" => {
                                if let Some(id) = frame["request_id"].as_str() {
                                    permissions.remove(id);
                                    out.emit(&turn.message_id, "permission_cancelled", json!({"id":id}));
                                }
                            },
                            "stream_event" if turn.active && frame["parent_tool_use_id"].is_null() => {
                                let event = &frame["event"];
                                match event["type"].as_str() {
                                    // Successive messages of one turn would otherwise run together.
                                    Some("content_block_start") => match event["content_block"]["type"].as_str() {
                                        Some("text") if needs_break(&turn.text) => {
                                            turn.text.push_str("\n\n");
                                            out.text.push_str("\n\n");
                                        },
                                        Some("thinking") if needs_break(&turn.thinking) => {
                                            turn.thinking.push_str("\n\n");
                                            out.thinking.push_str("\n\n");
                                        },
                                        _ => {}
                                    },
                                    Some("content_block_delta") => {
                                        if let Some(delta) = event["delta"]["text"].as_str() {
                                            turn.text.push_str(delta);
                                            out.text.push_str(delta);
                                            turn.dirty = true;
                                        }
                                        if let Some(delta) = event["delta"]["thinking"].as_str() {
                                            turn.thinking.push_str(delta);
                                            out.thinking.push_str(delta);
                                            turn.dirty = true;
                                        }
                                    },
                                    _ => {}
                                }
                            },
                            "assistant" if turn.active && frame["parent_tool_use_id"].is_null() => {
                                if let Some(blocks) = frame["message"]["content"].as_array() {
                                    for block in blocks {
                                        if block["type"] == "text" && turn.text.is_empty() {
                                            if let Some(text) = block["text"].as_str() {
                                                turn.final_text.push(text.to_string());
                                            }
                                        }
                                        if block["type"] == "tool_use" {
                                            let tool = ui_tool(block);
                                            out.emit(&turn.message_id, "tool", tool.clone());
                                            if turn.tools.len() >= 50 {
                                                turn.tools.remove(0);
                                            }
                                            turn.tools.push(tool);
                                            turn.dirty = true;
                                        }
                                    }
                                }
                            },
                            "user" if turn.active => {
                                if let Some(blocks) = frame["message"]["content"].as_array() {
                                    for block in blocks.iter().filter(|b| b["type"] == "tool_result") {
                                        let mut b = block.clone();
                                        if b.to_string().len() > MAX_TOOL_OUTPUT {
                                            b["content"] = json!("Output esteso omesso dalla UI.");
                                        }
                                        if let Some(tool) = turn.tools.iter_mut().find(|t| t["id"] == b["tool_use_id"]) {
                                            tool["status"] = json!(if b["is_error"].as_bool().unwrap_or(false) { "error" } else { "complete" });
                                            tool["output"] = json!(b["content"].as_str().map(String::from).unwrap_or_else(|| b["content"].to_string()));
                                        }
                                        out.emit(&turn.message_id, "tool_result", b);
                                        turn.dirty = true;
                                    }
                                }
                            },
                            "result" if turn.active => {
                                let failed = frame["is_error"].as_bool().unwrap_or(false)
                                    || frame["subtype"].as_str().is_some_and(|s| s.starts_with("error"));
                                if turn.text.is_empty() {
                                    turn.text = turn.final_text.join("\n\n");
                                }
                                if turn.text.is_empty() && !failed {
                                    turn.text = frame["result"].as_str().unwrap_or("").into();
                                }
                                let status = if turn.interrupted { "interrupted" } else if failed { "error" } else { "complete" };
                                let mut message = turn.message(status);
                                if failed && !turn.interrupted {
                                    message["error"] = json!(result_error(&frame));
                                }
                                message["usage"] = frame["usage"].clone();
                                store.put_message(&chat_id, &message)?;
                                busy_flag.store(false, Ordering::Release);
                                out.emit(&turn.message_id, "done", message);
                                turn.finish();
                                permissions.clear();
                                idle.as_mut().reset(Instant::now() + IDLE);
                            },
                            _ => {}
                        }
                        if turn.text.len() + turn.thinking.len() > protocol::MAX_OUTPUT {
                            return Err("Risposta oltre il limite locale di 2 MiB. Il processo è stato interrotto.".into());
                        }
                        if !was_pending && out.has_pending() {
                            flush_deadline.as_mut().reset(Instant::now() + DELTA_WINDOW);
                        }
                    },
                }
            }
            Ok(())
        }
        .await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        drop(process_guard);
        busy_flag.store(false, Ordering::Release);
        // Prompts accepted by the channel but never handed to Claude Code.
        rx.close();
        let mut orphans: Vec<String> = pending.take().map(|(id, _, _)| id).into_iter().collect();
        while let Ok(input) = rx.try_recv() {
            if let Input::Send { message_id, .. } = input {
                orphans.push(message_id);
            }
        }
        if outcome.is_err() {
            app.state::<Engine>().forget_subscription();
        }
        // Only the reply in progress is finalised: an error while idle must not rewrite
        // a message that already completed.
        if turn.active {
            out.flush(&turn.message_id);
            let error = match &outcome {
                Err(e) if !turn.interrupted => Some(e.as_str()),
                _ => None,
            };
            let mut message = turn.message(if error.is_some() {
                "error"
            } else {
                "interrupted"
            });
            if let Some(error) = error {
                message["error"] = json!(error);
            }
            let _ = store.put_message(&chat_id, &message);
            match error {
                Some(error) => out.emit(&turn.message_id, "failure", json!(error)),
                None => out.emit(&turn.message_id, "done", message),
            }
        }
        let reason = outcome
            .as_ref()
            .err()
            .map(String::as_str)
            .unwrap_or("Claude Code si è chiuso prima di ricevere il messaggio. Riprova.");
        for id in orphans {
            reject(&store, &mut out, &chat_id, &id, reason);
        }
        out.emit(&turn.message_id, "closed", Value::Null);
    });
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn framing_handles_partial_writes_and_rejects_oversized_frames() {
        let (mut tx, rx) = tokio::io::duplex(1024);
        tokio::spawn(async move {
            tx.write_all(b"{\"type\":").await.unwrap();
            tx.write_all(b"\"result\"}\nnext\n").await.unwrap();
        });
        let mut reader = BufReader::new(rx);
        assert_eq!(
            read_frame(&mut reader).await.unwrap().unwrap(),
            "{\"type\":\"result\"}\n"
        );
        assert_eq!(read_frame(&mut reader).await.unwrap().unwrap(), "next\n");
        assert!(read_frame(&mut reader).await.unwrap().is_none());
        let big = vec![b'x'; protocol::MAX_FRAME + 1];
        assert!(read_frame(&mut BufReader::new(big.as_slice()))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn frames_survive_slow_partial_writes_and_skip_blank_lines() {
        let (mut writer, reader) = tokio::io::duplex(64);
        tokio::spawn(async move {
            for chunk in [
                &b"{\"type\":\"a"[..],
                b"ssistant\"}\n\r\n",
                b"\n{\"type\":",
                b"\"result\"}\nnot json\n",
            ] {
                writer.write_all(chunk).await.unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        let (tx, mut frames) = mpsc::channel(8);
        tokio::spawn(read_frames(reader, tx));
        assert_eq!(frames.recv().await.unwrap().unwrap()["type"], "assistant");
        assert_eq!(frames.recv().await.unwrap().unwrap()["type"], "result");
        assert!(frames.recv().await.unwrap().is_err());
        assert!(frames.recv().await.is_none());
    }

    #[tokio::test]
    async fn stderr_is_drained_lossily_with_bounded_lines() {
        let mut data = b"prima riga\n".to_vec();
        data.extend(vec![b'x'; 100_000]);
        data.extend(b"\nerrore \xe8 finale\n\n");
        let last = Arc::new(StdMutex::new(String::new()));
        drain_stderr(data.as_slice(), last.clone()).await;
        assert_eq!(*last.lock().unwrap(), "errore \u{fffd} finale");
    }

    #[test]
    fn result_errors_are_readable() {
        assert_eq!(
            result_error(&json!({"subtype":"error_during_execution","errors":["quota"]})),
            "quota"
        );
        assert_eq!(
            result_error(&json!({"is_error":true,"result":"Modello non disponibile"})),
            "Modello non disponibile"
        );
        assert!(result_error(&json!({"subtype":"error_max_turns"})).contains("error_max_turns"));
    }

    #[test]
    fn large_tool_inputs_are_not_copied_to_the_ui() {
        let tool = ui_tool(
            &json!({"id":"t","name":"Write","input":{"content":"x".repeat(MAX_TOOL_INPUT)}}),
        );
        assert!(tool["input"]["content"].is_null());
        assert_eq!(tool["status"], "running");
        assert_eq!(ui_tool(&json!({"id":"t","input":{"a":1}}))["input"]["a"], 1);
    }
}
