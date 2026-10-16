import { useRef, useState } from "react";
import { useRegistry } from "../hooks/useRegistry";
import { errorMessage } from "../lib/registry";
import { size, time } from "../lib/format";
import type { AliasDto } from "../lib/transport";
import AliasNameForm from "./AliasNameForm";
import Confirmation from "./Confirmation";
import ThemeControl from "./ThemeControl";
export default function AppShell({ version }: { version: string | null }) {
  const r = useRegistry();
  const [editor, setEditor] = useState<{
      owner: string;
      alias: AliasDto | null;
      revision: string;
    } | null>(null),
    [deleting, setDeleting] = useState<{ owner: string; alias: AliasDto; revision: string } | null>(
      null,
    );
  const addAlias = useRef<HTMLButtonElement>(null);
  const file = r.selected;
  const review =
    r.error?.code === "conflict" || r.error?.code === "staleSession" || r.error?.code === "busy";
  const closeEditor = () => {
    setEditor(null);
    requestAnimationFrame(() => addAlias.current?.focus());
  };
  const statusLabel =
    file?.status.kind === "unavailable"
      ? `Unavailable (${{ notFound: "not found", permissionDenied: "permission denied", unknown: "unknown reason" }[file.status.reason]})`
      : file?.status.kind === "possibleReplacement"
        ? "Possible replacement"
        : file?.status.kind === "modified"
          ? "Modified observation"
          : "Available";
  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="app-brand">
          <img className="app-logo" src="/filenym-logo.png" alt="FileNym logo" />
          <h1>FileNym</h1>
          <span className="tagline">One file. Many names.</span>
        </div>
        <div className="header-actions">
          {version && <span className="app-version">v{version}</span>}
          <ThemeControl />
        </div>
      </header>
      <div className="announcements" role="status" aria-live="polite">
        {r.pending ? "Working…" : r.notice}
      </div>
      {r.error && (
        <div className="feedback" role="alert">
          {errorMessage(r.error)}
        </div>
      )}
      {!["ready", "initializing"].includes(r.status.phase) && (
        <div className="feedback">
          <p>
            {r.status.code === "unsupportedPlatform"
              ? "Desktop features are available in the FileNym Windows app. This browser preview does not save records."
              : `Registry ${r.status.phase}. Saved records will not be recreated or reset.`}
          </p>
          {r.status.code !== "unsupportedPlatform" && (
            <button disabled={r.pending} onClick={() => void r.reopen()}>
              Reopen and review saved registry
            </button>
          )}
        </div>
      )}
      <main className={`registry-layout ${r.selectedId ? "has-selection" : ""}`}>
        <aside className="file-panel" aria-labelledby="files-title">
          <div className="panel-heading">
            <h2 id="files-title">My Files</h2>
            <button disabled={!r.enabled || Boolean(editor)} onClick={() => void r.addFile()}>
              + Add file
            </button>
          </div>
          <div className="list-tools">
            <span>
              {r.status.phase === "blocked" ? "Not loaded" : `${r.entries.length} registered`}
            </span>
            <button disabled={!r.enabled || Boolean(editor)} onClick={() => void r.refresh()}>
              Refresh
            </button>
          </div>
          {r.status.phase === "initializing" ? (
            <p className="empty">Opening registry…</p>
          ) : r.entries.length === 0 ? (
            <p className="empty">
              {r.status.phase === "ready"
                ? "Select an existing file to give it more names."
                : "Saved records have not been loaded."}
            </p>
          ) : (
            <ul className="file-list">
              {r.entries.map((entry) =>
                entry.kind === "file" ? (
                  <li key={entry.file.id}>
                    <button
                      className="file-item"
                      aria-current={r.selectedId === entry.file.id ? "true" : undefined}
                      title={entry.file.filename.display}
                      disabled={r.pending || Boolean(editor) || Boolean(deleting)}
                      onClick={() => void r.select(entry.file.id)}
                    >
                      <strong>{entry.file.filename.display}</strong>
                      <span>
                        {entry.file.aliases.length}{" "}
                        {entry.file.aliases.length === 1 ? "alias" : "aliases"} · recorded{" "}
                        {entry.file.status.kind}
                      </span>
                    </button>
                  </li>
                ) : (
                  <li key={entry.id} className="unreadable">
                    {entry.kind === "unreadable"
                      ? "Unreadable saved record"
                      : "Missing saved record"}
                    <small>{entry.id}</small>
                  </li>
                ),
              )}
            </ul>
          )}
        </aside>
        <section className="detail-panel" aria-label="File details">
          {file ? (
            <>
              <button className="back" disabled={r.pending || Boolean(editor)} onClick={r.back}>
                ← My Files
              </button>
              <div className="file-heading">
                <h2 title={file.filename.display}>{file.filename.display}</h2>
                <span className="status-badge">Recorded: {statusLabel}</span>
              </div>
              <dl className="file-facts">
                <dt>Saved location</dt>
                <dd className="saved-path">
                  {file.savedPath.display}
                  {file.savedPath.escaped && (
                    <small>Escaped native characters shown literally.</small>
                  )}
                </dd>
                <dt>Recorded size</dt>
                <dd>{size(file.sizeBytes)}</dd>
                <dt>Recorded modified</dt>
                <dd
                  title={`${file.modifiedAt?.seconds ?? ""} seconds; ${file.modifiedAt?.nanoseconds ?? ""} nanoseconds`}
                >
                  {time(file.modifiedAt)}
                </dd>
                <dt>Registered</dt>
                <dd>{time(file.registeredAt)}</dd>
                <dt>Last recorded observation</dt>
                <dd>{time(file.lastObservedAt)}</dd>
              </dl>
              <p className="recorded-note">
                Saved metadata, not live file status. FileNym does not track changes automatically
                yet.
              </p>
              <div className="panel-heading alias-heading">
                <h2>
                  Aliases <span className="count">{file.aliases.length}</span>
                </h2>
                <button
                  ref={addAlias}
                  disabled={!r.enabled || Boolean(editor) || Boolean(deleting)}
                  onClick={() =>
                    setEditor({ owner: file.id, alias: null, revision: file.revision })
                  }
                >
                  + Add
                </button>
              </div>
              {file.aliases.length === 0 && !editor && (
                <p className="empty">No aliases yet. Add a name for this context.</p>
              )}
              <ul className="alias-list">
                {file.aliases.map((alias) => (
                  <li key={alias.id}>
                    {editor?.alias?.id === alias.id && editor.owner === file.id ? null : (
                      <div className="alias-row">
                        <span title={alias.name}>{alias.name}</span>
                        <details>
                          <summary aria-label={`Actions for ${alias.name}`}>···</summary>
                          <div className="context-actions">
                            <button
                              disabled={!r.enabled || Boolean(editor)}
                              onClick={() =>
                                setEditor({ owner: file.id, alias, revision: file.revision })
                              }
                            >
                              Rename
                            </button>
                            <button
                              disabled={!r.enabled || Boolean(editor)}
                              onClick={() =>
                                setDeleting({ owner: file.id, alias, revision: file.revision })
                              }
                            >
                              Delete
                            </button>
                          </div>
                        </details>
                      </div>
                    )}
                  </li>
                ))}
              </ul>
              {editor && editor.owner === file.id && (
                <AliasNameForm
                  key={`${editor.owner}:${editor.alias?.id ?? "new"}`}
                  initialName={editor.alias?.name ?? ""}
                  initialRevision={editor.revision}
                  disabled={!r.enabled}
                  needsReview={Boolean(review)}
                  onCancel={closeEditor}
                  onSave={async (name, revision) =>
                    Boolean(
                      await r.mutate(
                        editor.alias ? "rename" : "add",
                        file,
                        revision,
                        editor.alias?.id ?? null,
                        name,
                      ),
                    )
                  }
                  onReview={async () => {
                    const result = await r.refresh();
                    const entry = result?.entries.find(
                      (e) => e.kind === "file" && e.file.id === file.id,
                    );
                    return entry?.kind === "file" ? entry.file.revision : undefined;
                  }}
                />
              )}
              <p className="alias-note">
                Aliases are local names only. Your original file stays unchanged.
              </p>
            </>
          ) : (
            <div className="detail-empty">
              <h2>Your files, with room for more names.</h2>
              <p>Select a file to view its saved details and aliases.</p>
            </div>
          )}
        </section>
      </main>
      {deleting && (
        <Confirmation title="Delete alias?" disabled={r.pending} onCancel={() => setDeleting(null)}>
          <p>Delete “{deleting.alias.name}” from FileNym? The original file will stay unchanged.</p>
          <div className="dialog-actions">
            <button
              disabled={!r.enabled || Boolean(review)}
              onClick={() => {
                const owner = r.entries.find(
                  (e) => e.kind === "file" && e.file.id === deleting.owner,
                );
                if (owner?.kind === "file")
                  void r
                    .mutate("delete", owner.file, deleting.revision, deleting.alias.id, "")
                    .then((saved) => {
                      if (saved) {
                        setDeleting(null);
                        requestAnimationFrame(() => addAlias.current?.focus());
                      }
                    });
              }}
            >
              Delete alias
            </button>
          </div>
          {review && <p>Cancel and reload the record to review changes before deleting.</p>}
        </Confirmation>
      )}
      {r.duplicate && (
        <Confirmation
          title="Already registered location"
          disabled={r.pending}
          onCancel={() => void r.decide("cancel", null)}
        >
          <p>
            “{r.duplicate.filename.display}” has an existing saved location. A matching path does
            not prove it is the same physical file.
          </p>
          <div className="duplicate-candidates">
            {r.duplicate.duplicateCandidateIds.map((id, i) => (
              <button
                key={id}
                disabled={r.pending}
                onClick={() => void r.decide("viewExisting", id)}
              >
                View existing {i + 1}
              </button>
            ))}
          </div>
          <button disabled={r.pending} onClick={() => void r.decide("addSeparately", null)}>
            Add separately
          </button>
        </Confirmation>
      )}
    </div>
  );
}
