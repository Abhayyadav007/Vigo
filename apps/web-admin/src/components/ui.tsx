import { ApiError, rupeesToPaise } from "@vigo/api-client";
import { useId, useState, type ReactNode } from "react";

export function errorText(err: unknown): string | undefined {
  if (!err) return undefined;
  if (err instanceof ApiError) return err.message;
  return err instanceof Error ? err.message : "Something went wrong";
}

export function ErrorText({ error }: { error: unknown }) {
  const text = errorText(error);
  return text ? (
    <p className="error" role="alert">
      {text}
    </p>
  ) : null;
}

export function Field({ label, children, hint }: { label: string; children: (id: string) => ReactNode; hint?: string }) {
  const id = useId();
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      {children(id)}
      {hint ? <small className="muted">{hint}</small> : null}
    </div>
  );
}

/** Rupee input backed by integer paise. Shows an error while the text isn't a valid amount. */
export function MoneyInput({
  id,
  paise,
  onChange,
  required = true,
}: {
  id: string;
  paise: number | null;
  onChange: (paise: number | null) => void;
  required?: boolean;
}) {
  const [text, setText] = useState(paise === null ? "" : (paise / 100).toString());
  const parsed = text.trim() === "" ? null : rupeesToPaise(text);
  const invalid = text.trim() !== "" && parsed === null;
  return (
    <div className="input-group">
      <span>₹</span>
      <input
        id={id}
        inputMode="decimal"
        required={required}
        value={text}
        aria-invalid={invalid}
        onChange={(e) => {
          setText(e.target.value);
          const v = e.target.value.trim() === "" ? null : rupeesToPaise(e.target.value);
          if (v !== null || e.target.value.trim() === "") onChange(v);
        }}
      />
    </div>
  );
}

export function Pager({
  total,
  limit,
  offset,
  onChange,
}: {
  total: number;
  limit: number;
  offset: number;
  onChange: (offset: number) => void;
}) {
  return (
    <div className="pager">
      <span className="muted">
        {total === 0 ? "0" : `${offset + 1}–${Math.min(offset + limit, total)}`} of {total}
      </span>
      <button type="button" className="btn secondary" disabled={offset === 0} onClick={() => onChange(offset - limit)}>
        Previous
      </button>
      <button
        type="button"
        className="btn secondary"
        disabled={offset + limit >= total}
        onClick={() => onChange(offset + limit)}
      >
        Next
      </button>
    </div>
  );
}

