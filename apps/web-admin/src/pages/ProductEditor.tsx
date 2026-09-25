import { discountPercent, formatPaise, useCategories, useProduct, useSaveProduct } from "@vigo/api-client";
import type { ProductRequest } from "@vigo/types";
import { useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { ImageUploader } from "../components/ImageUploader";
import { ErrorText, Field, MoneyInput } from "../components/ui";

interface Draft {
  categoryId: string;
  name: string;
  brand: string;
  unitLabel: string;
  barcode: string;
  description: string;
  mrpPaise: number | null;
  pricePaise: number | null;
  imageUrls: string[];
  isActive: boolean;
}

const EMPTY: Draft = {
  categoryId: "",
  name: "",
  brand: "",
  unitLabel: "",
  barcode: "",
  description: "",
  mrpPaise: null,
  pricePaise: null,
  imageUrls: [],
  isActive: true,
};

export function ProductEditor() {
  const { id } = useParams();
  const existing = useProduct(id);
  if (!id) return <ProductForm key="new" initial={EMPTY} />;
  if (existing.error) return <ErrorText error={existing.error} />;
  if (!existing.data) return <p className="muted">Loading…</p>;
  const p = existing.data;
  return (
    <ProductForm
      key={id}
      id={id}
      initial={{
        categoryId: p.categoryId,
        name: p.name,
        brand: p.brand ?? "",
        unitLabel: p.unitLabel,
        barcode: p.barcode ?? "",
        description: p.description ?? "",
        mrpPaise: p.mrpPaise,
        pricePaise: p.pricePaise,
        imageUrls: p.imageUrls,
        isActive: p.isActive,
      }}
    />
  );
}

function ProductForm({ id, initial }: { id?: string; initial: Draft }) {
  const navigate = useNavigate();
  const categories = useCategories();
  const save = useSaveProduct();
  const [draft, setDraft] = useState<Draft>(initial);
  const [formError, setFormError] = useState<string>();
  const set = <K extends keyof Draft>(key: K, value: Draft[K]) => setDraft({ ...draft, [key]: value });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setFormError(undefined);
    if (draft.mrpPaise === null || draft.pricePaise === null) return setFormError("Enter MRP and selling price.");
    if (draft.pricePaise > draft.mrpPaise) return setFormError("Selling price can't be above MRP.");
    const optional = (v: string) => (v.trim() ? v.trim() : undefined);
    const body: ProductRequest = {
      categoryId: draft.categoryId,
      name: draft.name.trim(),
      unitLabel: draft.unitLabel.trim(),
      mrpPaise: draft.mrpPaise,
      pricePaise: draft.pricePaise,
      imageUrls: draft.imageUrls,
      isActive: draft.isActive,
      ...(optional(draft.brand) ? { brand: optional(draft.brand) } : {}),
      ...(optional(draft.barcode) ? { barcode: optional(draft.barcode) } : {}),
      ...(optional(draft.description) ? { description: optional(draft.description) } : {}),
    };
    save.mutate({ id, body }, { onSuccess: () => void navigate("/products") });
  };

  const off = draft.mrpPaise && draft.pricePaise ? discountPercent(draft.mrpPaise, draft.pricePaise) : 0;

  return (
    <form className="card form" onSubmit={submit}>
      <div className="card-head">
        <h2>{id ? "Edit product" : "New product"}</h2>
        <Link to="/products">Back to products</Link>
      </div>
      <div className="grid-2">
        <Field label="Name">
          {(fid) => <input id={fid} required value={draft.name} onChange={(e) => set("name", e.target.value)} />}
        </Field>
        <Field label="Brand">
          {(fid) => <input id={fid} value={draft.brand} onChange={(e) => set("brand", e.target.value)} />}
        </Field>
        <Field label="Category">
          {(fid) => (
            <select id={fid} required value={draft.categoryId} onChange={(e) => set("categoryId", e.target.value)}>
              <option value="" disabled>
                Choose…
              </option>
              {categories.data?.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          )}
        </Field>
        <Field label="Pack size" hint='e.g. "500 g", "1 L", "6 pcs"'>
          {(fid) => (
            <input id={fid} required value={draft.unitLabel} onChange={(e) => set("unitLabel", e.target.value)} />
          )}
        </Field>
        <Field label="MRP">
          {(fid) => <MoneyInput id={fid} paise={draft.mrpPaise} onChange={(v) => set("mrpPaise", v)} />}
        </Field>
        <Field
          label="Selling price"
          hint={off ? `${off}% off · customer pays ${formatPaise(draft.pricePaise ?? 0)}` : "Stores can override this"}
        >
          {(fid) => <MoneyInput id={fid} paise={draft.pricePaise} onChange={(v) => set("pricePaise", v)} />}
        </Field>
        <Field label="Barcode (EAN/UPC)" hint="Scanned by pickers">
          {(fid) => (
            <input
              id={fid}
              inputMode="numeric"
              value={draft.barcode}
              onChange={(e) => set("barcode", e.target.value.replace(/\D/g, ""))}
            />
          )}
        </Field>
      </div>
      <Field label="Description">
        {(fid) => (
          <textarea id={fid} rows={3} value={draft.description} onChange={(e) => set("description", e.target.value)} />
        )}
      </Field>
      <Field label="Images" hint="JPEG, PNG or WebP, up to 5 MB each">
        {() => <ImageUploader urls={draft.imageUrls} onChange={(urls) => set("imageUrls", urls)} />}
      </Field>
      <label className="checkbox">
        <input type="checkbox" checked={draft.isActive} onChange={(e) => set("isActive", e.target.checked)} />
        Active (sellable)
      </label>
      {formError ? <p className="error">{formError}</p> : null}
      <ErrorText error={save.error} />
      <div className="actions">
        <button className="btn" disabled={save.isPending}>
          {save.isPending ? "Saving…" : "Save product"}
        </button>
      </div>
    </form>
  );
}
