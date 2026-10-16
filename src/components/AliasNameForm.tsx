import { useEffect, useRef, useState } from "react";
import { aliasIssue } from "../lib/aliasValidation";
export default function AliasNameForm({
  initialName,
  initialRevision,
  disabled,
  needsReview,
  onReview,
  onSave,
  onCancel,
}: {
  initialName: string;
  initialRevision: string;
  disabled: boolean;
  needsReview: boolean;
  onReview: () => Promise<string | undefined>;
  onSave: (name: string, revision: string) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initialName),
    [revision, setRevision] = useState(initialRevision),
    [issue, setIssue] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (!disabled) input.current?.focus();
  }, [disabled]);
  async function submit() {
    const issue = aliasIssue(name);
    setIssue(issue);
    if (issue) {
      input.current?.focus();
      return;
    }
    if (await onSave(name, revision)) onCancel();
    else input.current?.focus();
  }
  return (
    <form
      className="alias-form"
      aria-label={initialName ? "Rename alias" : "Add alias"}
      onSubmit={(e) => {
        e.preventDefault();
        if (!disabled && !needsReview) void submit();
      }}
    >
      <label className="sr-only" htmlFor="alias-name">
        Alias name
      </label>
      <input
        autoFocus
        ref={input}
        id="alias-name"
        value={name}
        disabled={disabled}
        onChange={(e) => {
          setName(e.target.value);
          setIssue(null);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && e.nativeEvent.isComposing) e.preventDefault();
          if (e.key === "Escape" && !disabled) {
            e.preventDefault();
            onCancel();
          }
        }}
        aria-invalid={Boolean(issue)}
        aria-describedby="alias-help"
      />
      <span id="alias-help" className={issue ? "field-error" : "muted"}>
        {issue ??
          `${new TextEncoder().encode(name).length}/255 UTF-8 bytes · extensions stay as entered`}
      </span>
      <div className="inline-actions">
        <button disabled={disabled || needsReview} type="submit">
          Save
        </button>
        <button disabled={disabled} type="button" onClick={onCancel}>
          Cancel
        </button>
        {needsReview && (
          <button
            disabled={disabled}
            type="button"
            onClick={() =>
              void onReview().then((value) => {
                if (value) {
                  setRevision(value);
                  input.current?.focus();
                }
              })
            }
          >
            Reload and review draft
          </button>
        )}
      </div>
    </form>
  );
}
