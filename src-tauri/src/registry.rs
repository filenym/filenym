//! Desktop coordination; all mutations delegate to the unchanged application service.
use crate::{ids, state::DesktopState, transport::*};
use filenym_app::{AddAlias, DeleteAlias, FileRegistryService, RenameAlias};
use std::time::SystemTime;

pub fn get(state: &DesktopState, request: GetFileRequestDto) -> CommandResult<CanonicalFileDto> {
    state.with(|s| {
        state.check(s, &request.storage_epoch, false)?;
        let result = FileRegistryService::new(s.repository.as_mut().expect("checked repository"))
            .get_canonical_file(&request.canonical_id)
            .map_err(CommandErrorDto::from)
            .and_then(CanonicalFileDto::from_stored);
        if result.is_ok() || matches!(&result,Err(e) if e.code==Code::CanonicalNotFound) {
            state.reviewed(s, &request.canonical_id);
        }
        state.finish(s, result, None)
    })
}

pub fn list(state: &DesktopState, request: SessionRequestDto) -> CommandResult<RegistryDto> {
    state.with(|s| {
        state.check(s, &request.storage_epoch, false)?;
        let result = (|| {
            let mut service =
                FileRegistryService::new(s.repository.as_mut().expect("checked repository"));
            let ids = service.list_registered_file_ids()?;
            let mut entries = Vec::new();
            for id in ids {
                let id = id.as_str().to_owned();
                match service
                    .get_canonical_file(&id)
                    .map_err(CommandErrorDto::from)
                    .and_then(CanonicalFileDto::from_stored)
                {
                    Ok(file) => entries.push(RegistryEntryDto::File {
                        file: Box::new(file),
                    }),
                    Err(e) if e.code == Code::CanonicalNotFound => {
                        entries.push(RegistryEntryDto::Missing { id })
                    }
                    Err(e)
                        if matches!(
                            e.code,
                            Code::InvalidStoredData
                                | Code::UnsupportedPathEncoding
                                | Code::UnrepresentableValue
                        ) =>
                    {
                        entries.push(RegistryEntryDto::Unreadable { id, code: e.code })
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(RegistryDto { entries })
        })();
        if let Ok(inventory) = &result {
            if s.inventory_review {
                for entry in &inventory.entries {
                    if let RegistryEntryDto::Unreadable { id, .. } = entry {
                        s.review.insert(id.clone());
                    }
                }
            }
            s.inventory_review = false;
            for entry in &inventory.entries {
                match entry {
                    RegistryEntryDto::File { file } => {
                        s.review.remove(&file.id);
                    }
                    RegistryEntryDto::Missing { id } => {
                        s.review.remove(id);
                    }
                    _ => {}
                }
            }
            // IDs absent from a successfully read inventory are definitively absent.
            s.review.retain(|id| {
                inventory
                    .entries
                    .iter()
                    .any(|e| matches!(e,RegistryEntryDto::Unreadable{id:other,..} if other==id))
            });
            if s.review.is_empty() {
                state.publish(s, Phase::Ready, None);
            }
        }
        state.finish(s, result, None)
    })
}

pub fn add(state: &DesktopState, r: AddAliasRequestDto) -> CommandResult<CanonicalFileDto> {
    state.with(|s| {
        state.check(s, &r.storage_epoch, true)?;
        let revision = revision(&r.expected_revision)?;
        let alias_id = ids::new_id()?;
        let attempted = Some(Attempted {
            canonical_id: Some(r.canonical_id.clone()),
            alias_id: Some(alias_id.clone()),
        });
        let result = FileRegistryService::new(s.repository.as_mut().expect("checked repository"))
            .add_alias(AddAlias {
                canonical_id: r.canonical_id,
                expected_revision: revision,
                alias_id,
                name: r.name,
                created_at: SystemTime::now(),
            })
            .map_err(CommandErrorDto::from)
            .and_then(CanonicalFileDto::from_committed);
        state.finish(s, result, attempted)
    })
}
pub fn rename(state: &DesktopState, r: RenameAliasRequestDto) -> CommandResult<CanonicalFileDto> {
    state.with(|s| {
        state.check(s, &r.storage_epoch, true)?;
        let revision = revision(&r.expected_revision)?;
        let attempted = Some(Attempted {
            canonical_id: Some(r.canonical_id.clone()),
            alias_id: Some(r.alias_id.clone()),
        });
        let result = FileRegistryService::new(s.repository.as_mut().expect("checked repository"))
            .rename_alias(RenameAlias {
                canonical_id: r.canonical_id,
                expected_revision: revision,
                alias_id: r.alias_id,
                name: r.name,
                updated_at: SystemTime::now(),
            })
            .map_err(CommandErrorDto::from)
            .and_then(CanonicalFileDto::from_committed);
        state.finish(s, result, attempted)
    })
}
pub fn delete(state: &DesktopState, r: DeleteAliasRequestDto) -> CommandResult<CanonicalFileDto> {
    state.with(|s| {
        state.check(s, &r.storage_epoch, true)?;
        let revision = revision(&r.expected_revision)?;
        let attempted = Some(Attempted {
            canonical_id: Some(r.canonical_id.clone()),
            alias_id: Some(r.alias_id.clone()),
        });
        let result = FileRegistryService::new(s.repository.as_mut().expect("checked repository"))
            .delete_alias(DeleteAlias {
                canonical_id: r.canonical_id,
                expected_revision: revision,
                alias_id: r.alias_id,
            })
            .map_err(CommandErrorDto::from)
            .and_then(CanonicalFileDto::from_committed);
        state.finish(s, result, attempted)
    })
}
