//! Picker results stay native. Only duplicate decisions have backend-held tokens.
use crate::{
    ids,
    state::{DesktopState, PendingDuplicate},
    transport::*,
};
use filenym_app::{FileRegistryService, RegisterCanonicalFile};
use filenym_infra::registration::inspect_registration_candidate;
use std::{path::PathBuf, time::SystemTime};

pub fn selected(
    state: &DesktopState,
    epoch: String,
    path: PathBuf,
) -> CommandResult<SelectionOutcomeDto> {
    state.with(|s| state.check(s, &epoch, true))?;
    let facts = inspect_registration_candidate(&path)?;
    state.with(|s| {
        state.check(s, &epoch, true)?;
        let result = (|| {
            let mut service =
                FileRegistryService::new(s.repository.as_mut().expect("checked repository"));
            let mut candidates = Vec::new();
            for id in service.list_registered_file_ids()? {
                let stored = service.get_canonical_file(id.as_str())?;
                if stored.file().location().as_path().as_os_str() == path.as_os_str() {
                    candidates.push(id.as_str().to_owned());
                }
            }
            if !candidates.is_empty() {
                let token = ids::new_id()?;
                s.pending = Some(PendingDuplicate {
                    token: token.clone(),
                    path,
                    metadata: facts.metadata.clone(),
                    candidates: candidates.clone(),
                    reserved: false,
                });
                return Ok(SelectionOutcomeDto::DuplicateDecision {
                    selection_token: token,
                    filename: NativeTextDto::new(facts.location.filename())?,
                    duplicate_candidate_ids: candidates,
                });
            }
            let id = ids::new_id()?;
            let result = service
                .register_canonical_file(RegisterCanonicalFile {
                    canonical_id: id.clone(),
                    location: facts.location.clone(),
                    identity: None,
                    metadata: facts.metadata.clone(),
                    registered_at: SystemTime::now(),
                })
                .map_err(CommandErrorDto::from)
                .and_then(CanonicalFileDto::from_committed);
            result
                .map(|file| SelectionOutcomeDto::Added {
                    file: Box::new(file),
                })
                .map_err(|mut e| {
                    e.attempted = Some(Attempted {
                        canonical_id: Some(id),
                        alias_id: None,
                    });
                    e
                })
        })();
        let attempted = result.as_ref().err().and_then(|e| e.attempted.clone());
        state.finish(s, result, attempted)
    })
}

pub fn decide(
    state: &DesktopState,
    r: RegisterSelectionRequestDto,
) -> CommandResult<DuplicateDecisionOutcomeDto> {
    let pending = state.with(|s| {
        state.check(s, &r.storage_epoch, true)?;
        if !matches!(&s.pending,Some(p) if p.token==r.selection_token && !p.reserved) {
            return Err(CommandErrorDto::new(Code::SelectionInvalid));
        }
        let pending = s.pending.as_mut().expect("matched pending selection");
        pending.reserved = true;
        Ok(pending.clone())
    })?;
    if matches!(r.decision, Decision::Cancel) {
        return state.with(|s| {
            state.check(s, &r.storage_epoch, false)?;
            if !matches!(&s.pending,Some(p) if p.token==r.selection_token) {
                return Err(CommandErrorDto::new(Code::SelectionInvalid));
            }
            s.pending = None;
            state.finish(s, Ok(DuplicateDecisionOutcomeDto::Cancelled), None)
        });
    }
    if matches!(r.decision, Decision::ViewExisting) {
        state.with(|s| {
            state.check(s, &r.storage_epoch, false)?;
            if !matches!(&s.pending,Some(p) if p.token==r.selection_token) {
                return Err(CommandErrorDto::new(Code::SelectionInvalid));
            }
            s.pending = None;
            Ok(())
        })?;
        let id = r
            .candidate_id
            .filter(|id| pending.candidates.contains(id))
            .ok_or_else(|| CommandErrorDto::new(Code::SelectionInvalid))?;
        return crate::registry::get(
            state,
            GetFileRequestDto {
                storage_epoch: r.storage_epoch,
                canonical_id: id,
            },
        )
        .map(|reply| Reply {
            storage_epoch: reply.storage_epoch,
            value: DuplicateDecisionOutcomeDto::ViewExisting {
                file: Box::new(reply.value),
            },
        });
    }
    let facts = match inspect_registration_candidate(&pending.path) {
        Ok(facts) => facts,
        Err(error) => {
            let _ = state.with(|s| {
                if let Some(p) = s.pending.as_mut().filter(|p| p.token == r.selection_token) {
                    if matches!(
                        error,
                        filenym_infra::registration::RegistrationError::PermissionDenied
                            | filenym_infra::registration::RegistrationError::MetadataUnavailable
                    ) {
                        p.reserved = false;
                    } else {
                        s.pending = None;
                    }
                }
                Ok(())
            });
            return Err(error.into());
        }
    };
    if facts.metadata != pending.metadata {
        state.with(|s| {
            if matches!(&s.pending,Some(p) if p.token==r.selection_token) {
                s.pending = None;
            }
            Ok(())
        })?;
        return Err(CommandErrorDto::new(Code::SelectionChanged));
    }
    state.with(|s| {
        state.check(s, &r.storage_epoch, true)?;
        if !matches!(&s.pending,Some(p) if p.token==r.selection_token && p.reserved) {
            return Err(CommandErrorDto::new(Code::SelectionInvalid));
        }
        let id = match ids::new_id() {
            Ok(id) => id,
            Err(e) => {
                if let Some(p) = s.pending.as_mut() {
                    p.reserved = false;
                }
                return Err(e);
            }
        };
        let attempted = Some(Attempted {
            canonical_id: Some(id.clone()),
            alias_id: None,
        });
        let result = (|| {
            let mut service = FileRegistryService::new(s.repository.as_mut().expect("checked repository"));
            let mut candidates = Vec::new();
            for candidate in service.list_registered_file_ids()? {
                let file = service.get_canonical_file(candidate.as_str())?;
                if file.file().location().as_path().as_os_str() == pending.path.as_os_str() {
                    candidates.push(candidate.as_str().to_owned());
                }
            }
            if candidates != pending.candidates {
                return Err(CommandErrorDto::new(Code::SelectionChanged));
            }
            service.register_canonical_file(RegisterCanonicalFile {
                canonical_id: id,
                location: facts.location.clone(),
                identity: None,
                metadata: facts.metadata.clone(),
                registered_at: SystemTime::now(),
            })
            .map_err(CommandErrorDto::from)
            .and_then(CanonicalFileDto::from_committed)
            .map(|file| DuplicateDecisionOutcomeDto::Added { file: Box::new(file) })
        })();
        // Only a deterministic rejection can retain the already chosen candidate.
        if result.is_ok() || matches!(&result,Err(e) if matches!(e.code,Code::UncertainOutcome|Code::SelectionChanged)) {
            s.pending = None;
        } else if let Some(p) = s.pending.as_mut() {
            p.reserved = false;
        }
        state.finish(s, result, attempted)
    })
}
