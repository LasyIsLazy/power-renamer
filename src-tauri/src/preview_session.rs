use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Preview,
    Execute,
}

impl OperationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Execute => "execute",
        }
    }

    fn progress_event(self) -> String {
        format!("{}-progress", self.label())
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub completed: usize,
    pub total: usize,
    pub current_path: Option<String>,
    pub phase: String,
}

#[derive(Serialize, Clone)]
pub struct PreviewChunkPayload {
    pub mappings: Vec<(String, String)>,
    pub logs: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCompletePayload {
    pub md5_preview_placeholder: bool,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

struct SessionInner {
    app: AppHandle,
    kind: OperationKind,
    cancel: AtomicBool,
    files_done: AtomicUsize,
    total_files: usize,
}

#[derive(Clone, Default)]
pub struct PreviewSession {
    inner: Arc<Mutex<Option<SessionInner>>>,
}

fn session_log(kind: OperationKind, message: impl AsRef<str>) {
    eprintln!("[{}] {}", kind.label(), message.as_ref());
}

impl PreviewSession {
    pub fn try_begin(
        &self,
        app: AppHandle,
        total_files: usize,
        kind: OperationKind,
    ) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| format!("操作状态锁定失败: {}", e))?;
        if guard.is_some() {
            return Err("已有任务正在进行中".to_string());
        }
        session_log(kind, format!("session begin, total_files={}", total_files));
        *guard = Some(SessionInner {
            app,
            kind,
            cancel: AtomicBool::new(false),
            files_done: AtomicUsize::new(0),
            total_files,
        });
        Ok(())
    }

    pub fn cancel(&self) {
        if let Ok(guard) = self.inner.lock() {
            if let Some(s) = guard.as_ref() {
                session_log(s.kind, "cancel requested");
                s.cancel.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.cancel.load(Ordering::SeqCst)))
            .unwrap_or(false)
    }

    pub fn files_done(&self) -> usize {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.files_done.load(Ordering::SeqCst)))
            .unwrap_or(0)
    }

    pub fn finish(&self) {
        if let Ok(guard) = self.inner.lock() {
            if let Some(s) = guard.as_ref() {
                session_log(s.kind, "session finish");
            }
        }
        if let Ok(mut guard) = self.inner.lock() {
            *guard = None;
        }
    }

    fn with_session<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&SessionInner) -> R,
    {
        let guard = self.inner.lock().ok()?;
        let inner = guard.as_ref()?;
        Some(f(inner))
    }

    fn emit_raw<T: Serialize + Clone>(&self, event: &str, payload: T) {
        match self.with_session(|s| s.app.emit(event, payload)) {
            None => eprintln!("[progress] emit {} skipped: no active session", event),
            Some(Err(e)) => eprintln!("[progress] emit {} failed: {}", event, e),
            Some(Ok(())) => {}
        }
    }

    pub fn emit_progress(
        &self,
        completed: usize,
        total: usize,
        current_path: Option<String>,
        phase: &str,
    ) {
        if let Some((kind, event)) = self.with_session(|s| (s.kind, s.kind.progress_event())) {
            let path_log = current_path.as_deref().unwrap_or("-");
            session_log(
                kind,
                format!(
                    "emit progress: {}/{} phase={} path={}",
                    completed, total, phase, path_log
                ),
            );
            self.emit_raw(
                &event,
                ProgressPayload {
                    completed,
                    total,
                    current_path,
                    phase: phase.to_string(),
                },
            );
        }
    }

    pub fn on_file_processed(&self, file_path: &str, phase: &str) {
        let emit_info = {
            let guard = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let inner = match guard.as_ref() {
                Some(s) => s,
                None => return,
            };
            let done = inner.files_done.fetch_add(1, Ordering::SeqCst) + 1;
            Some((
                inner.kind,
                inner.app.clone(),
                inner.kind.progress_event(),
                done,
                inner.total_files,
                file_path.to_string(),
                phase.to_string(),
            ))
        };

        if let Some((kind, app, event, done, total, path, phase)) = emit_info {
            session_log(
                kind,
                format!("emit {} progress: {}/{} path={}", phase, done, total, path),
            );
            let payload = ProgressPayload {
                completed: done,
                total,
                current_path: Some(path),
                phase,
            };
            if let Err(e) = app.emit(&event, payload) {
                session_log(kind, format!("emit progress failed: {}", e));
            }
        }
    }

    pub fn emit_chunk(&self, mappings: Vec<(String, String)>, logs: Vec<String>) {
        if let Some(kind) = self.with_session(|s| s.kind) {
            session_log(kind, format!("emit chunk: {} mappings", mappings.len()));
        }
        self.emit_raw(
            "preview-chunk",
            PreviewChunkPayload { mappings, logs },
        );
    }

    pub fn emit_preview_complete(&self, payload: PreviewCompletePayload) {
        if let Some(kind) = self.with_session(|s| s.kind) {
            session_log(
                kind,
                format!(
                    "emit complete: cancelled={} errors={}",
                    payload.cancelled,
                    payload.errors.len()
                ),
            );
        }
        self.emit_raw("preview-complete", payload);
    }

    pub fn emit_preview_complete_via_app(app: &AppHandle, payload: PreviewCompletePayload) {
        eprintln!(
            "[preview] emit complete (direct): errors={}",
            payload.errors.len()
        );
        if let Err(e) = app.emit("preview-complete", payload) {
            eprintln!("[preview] emit complete failed: {}", e);
        }
    }

    pub fn emit_execute_complete<T: Serialize + Clone>(&self, payload: T) {
        session_log(OperationKind::Execute, "emit execute complete");
        self.emit_raw("execute-complete", payload);
    }
}
