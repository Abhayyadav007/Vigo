import { useCategories, useSaveCategory } from "@vigo/api-client";
import type { AdminCategory, CategoryRequest } from "@vigo/types";
import { useState, type FormEvent } from "react";
import { ErrorText, Field } from "../components/ui";

type Draft = { id?: string; name: string; parentId: string; sortOrder: string; isActive: boolean; imageUrl?: string | null };
const EMPTY: Draft = { name: "", parentId: "", sortOrder: "0", isActive: true };

export function Categories() {
  const categories = useCategories();
  const save = useSaveCategory();
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const all = categories.data ?? [];
  const nameOf = (id: string | null) => all.find((c) => c.id === id)?.name ?? "—";

  const edit = (c: AdminCategory) =>
    setDraft({
      id: c.id,
      name: c.name,
      parentId: c.parentId ?? "",
      sortOrder: String(c.sortOrder),
      isActive: c.isActive,
      imageUrl: c.imageUrl,
    });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const body: CategoryRequest = {
      name: draft.name.trim(),
      sortOrder: Number(draft.sortOrder) || 0,
      isActive: draft.isActive,
      ...(draft.parentId ? { parentId: draft.parentId } : {}),
      ...(draft.imageUrl ? { imageUrl: draft.imageUrl } : {}),
    };
    save.mutate({ id: draft.id, body }, { onSuccess: () => setDraft(EMPTY) });
  };

  return (
    <div className="split">
      <section className="card">
        <h2>Categories</h2>
        <ErrorText error={categories.error} />
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Parent</th>
              <th>Products</th>
              <th>Order</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {all.map((c) => (
              <tr key={c.id}>
                <td>
                  <button type="button" className="link" onClick={() => edit(c)}>
                    {c.name}
                  </button>
                </td>
                <td className="muted">{nameOf(c.parentId)}</td>
                <td>{c.productCount}</td>
                <td className="muted">{c.sortOrder}</td>
                <td>
                  <span className={`pill ${c.isActive ? "pill-ok" : ""}`}>{c.isActive ? "active" : "hidden"}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <form className="card form" onSubmit={submit}>
        <h2>{draft.id ? "Edit category" : "New category"}</h2>
        <Field label="Name">
          {(id) => (
            <input id={id} required value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
          )}
        </Field>
        <Field label="Parent">
          {(id) => (
            <select id={id} value={draft.parentId} onChange={(e) => setDraft({ ...draft, parentId: e.target.value })}>
              <option value="">None (top level)</option>
              {all
                .filter((c) => c.id !== draft.id)
                .map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
            </select>
          )}
        </Field>
        <Field label="Sort order" hint="Lower shows first">
          {(id) => (
            <input
              id={id}
              inputMode="numeric"
              value={draft.sortOrder}
              onChange={(e) => setDraft({ ...draft, sortOrder: e.target.value })}
            />
          )}
        </Field>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.isActive}
            onChange={(e) => setDraft({ ...draft, isActive: e.target.checked })}
          />
          Visible to customers
        </label>
        <ErrorText error={save.error} />
        <div className="actions">
          <button className="btn" disabled={save.isPending}>
            {draft.id ? "Save" : "Add category"}
          </button>
          {draft.id ? (
            <button type="button" className="btn ghost" onClick={() => setDraft(EMPTY)}>
              Cancel
            </button>
          ) : null}
        </div>
      </form>
    </div>
  );
}
