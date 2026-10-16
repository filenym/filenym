use crate::{ids, storage, transport::*};
use filenym_core::FileMetadata;
use filenym_infra::SqliteRepository;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone)]
pub struct PendingDuplicate {
    pub token: String,
    pub path: PathBuf,
    pub metadata: FileMetadata,
    pub candidates: Vec<String>,
    pub reserved: bool,
}
pub struct Session {
    pub root: Option<PathBuf>,
    pub repository: Option<SqliteRepository>,
    pub epoch: Option<String>,
    pub review: BTreeSet<String>,
    pub inventory_review: bool,
    pub pending: Option<PendingDuplicate>,
    pub initialized: bool,
}
#[derive(Clone)]
pub struct DesktopState {
    pub inner: Arc<Mutex<Session>>,
    pub status: Arc<Mutex<Reply<RegistryStatusDto>>>,
    pub picker: Arc<AtomicBool>,
    pub closing: Arc<AtomicBool>,
}
impl DesktopState {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Session {
                root,
                repository: None,
                epoch: None,
                review: BTreeSet::new(),
                inventory_review: false,
                pending: None,
                initialized: false,
            })),
            status: Arc::new(Mutex::new(Reply {
                storage_epoch: None,
                value: RegistryStatusDto {
                    phase: Phase::Initializing,
                    code: None,
                    affected_canonical_ids: vec![],
                },
            })),
            picker: Arc::new(AtomicBool::new(false)),
            closing: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn publish(&self, s: &Session, phase: Phase, code: Option<Code>) {
        let value = Reply {
            storage_epoch: s.epoch.clone(),
            value: RegistryStatusDto {
                phase,
                code,
                affected_canonical_ids: s.review.iter().cloned().collect(),
            },
        };
        match self.status.lock() {
            Ok(mut status) => *status = value,
            Err(poison) => {
                *poison.into_inner() = value;
                self.status.clear_poison();
            }
        }
    }
    pub fn initialize(&self, reopen: bool) -> Result<Reply<RegistryStatusDto>, CommandErrorDto> {
        self.with(|s| {
            if self.closing.load(Ordering::SeqCst) {
                return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
            }
            self.publish(
                s,
                if reopen {
                    Phase::Reopening
                } else {
                    Phase::Initializing
                },
                None,
            );
            s.pending = None;
            s.repository = None;
            let Some(root) = s.root.as_ref() else {
                self.publish(s, Phase::Blocked, Some(Code::DatabaseUnavailable));
                return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
            };
            // Recovery never creates a replacement registry. Initial failed artifacts are preserved.
            let result = storage::open(root, !reopen && !s.initialized);
            s.initialized = true;
            match result {
                Ok(repo) => {
                    let epoch = match ids::new_id() {
                        Ok(e) => e,
                        Err(e) => {
                            self.publish(s, Phase::Blocked, Some(e.code));
                            return Err(e);
                        }
                    };
                    s.repository = Some(repo);
                    s.epoch = Some(epoch);
                    self.publish(
                        s,
                        if s.review.is_empty() && !s.inventory_review {
                            Phase::Ready
                        } else {
                            Phase::NeedsReload
                        },
                        None,
                    );
                }
                Err(error) => {
                    self.publish(s, Phase::Blocked, Some(error.code));
                    return Err(error);
                }
            }
            self.status
                .lock()
                .map(|v| v.clone())
                .map_err(|_| CommandErrorDto::new(Code::InternalFailure))
        })
    }
    pub fn with<T>(
        &self,
        job: impl FnOnce(&mut Session) -> Result<T, CommandErrorDto>,
    ) -> Result<T, CommandErrorDto> {
        match self.inner.lock() {
            Ok(mut s) => job(&mut s),
            Err(poison) => {
                let mut s = poison.into_inner();
                s.repository = None;
                s.pending = None;
                s.inventory_review = true;
                self.publish(&s, Phase::NeedsReopen, Some(Code::UncertainOutcome));
                self.inner.clear_poison();
                Err(CommandErrorDto::new(Code::UncertainOutcome))
            }
        }
    }
    pub fn check(&self, s: &Session, epoch: &str, write: bool) -> Result<(), CommandErrorDto> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
        }
        if s.epoch.as_deref() != Some(epoch) {
            return Err(CommandErrorDto::new(Code::StaleSession));
        }
        if s.repository.is_none() {
            return Err(CommandErrorDto::new(Code::DatabaseUnavailable));
        }
        if write && (!s.review.is_empty() || s.inventory_review) {
            return Err(CommandErrorDto::new(Code::UncertainOutcome));
        }
        Ok(())
    }
    pub fn finish<T>(
        &self,
        s: &mut Session,
        result: Result<T, CommandErrorDto>,
        attempted: Option<Attempted>,
    ) -> CommandResult<T> {
        match result {
            Ok(value) => Ok(Reply {
                storage_epoch: s.epoch.clone(),
                value,
            }),
            Err(mut e) => {
                e.storage_epoch = s.epoch.clone();
                e.attempted = attempted;
                if e.code == Code::UncertainOutcome {
                    if let Some(id) = e.attempted.as_ref().and_then(|a| a.canonical_id.as_ref()) {
                        s.review.insert(id.clone());
                    } else {
                        s.inventory_review = true;
                    }
                    s.repository = None;
                    s.pending = None;
                    self.publish(s, Phase::NeedsReopen, Some(e.code));
                }
                Err(e)
            }
        }
    }
    pub fn reviewed(&self, s: &mut Session, id: &str) {
        s.review.remove(id);
        if s.review.is_empty() && !s.inventory_review {
            self.publish(s, Phase::Ready, None);
        }
    }
}

pub async fn blocking<T: Send + 'static>(
    state: DesktopState,
    job: impl FnOnce(&DesktopState) -> Result<T, CommandErrorDto> + Send + 'static,
) -> Result<T, CommandErrorDto> {
    let failed = state.clone();
    match tauri::async_runtime::spawn_blocking(move || job(&state)).await {
        Ok(result) => result,
        Err(_) => {
            let _ = failed.with::<()>(|s| {
                s.repository = None;
                s.pending = None;
                s.inventory_review = true;
                failed.publish(s, Phase::NeedsReopen, Some(Code::UncertainOutcome));
                Ok(())
            });
            Err(CommandErrorDto::new(Code::UncertainOutcome))
        }
    }
}
