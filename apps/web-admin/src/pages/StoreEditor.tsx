import { useSaveStore, useStore } from "@vigo/api-client";
import type { GeoJsonPolygon, LatLng, StoreRequest } from "@vigo/types";
import { useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { hexagon, PolygonEditor } from "../components/PolygonEditor";
import { ErrorText, Field } from "../components/ui";

interface Draft {
  code: string;
  name: string;
  address: string;
  location: LatLng | null;
  serviceArea: GeoJsonPolygon | null;
  isActive: boolean;
}

const EMPTY: Draft = { code: "", name: "", address: "", location: null, serviceArea: null, isActive: true };

export function StoreEditor() {
  const { id } = useParams();
  const existing = useStore(id);
  if (!id) return <StoreForm key="new" initial={EMPTY} />;
  if (existing.error) return <ErrorText error={existing.error} />;
  if (!existing.data) return <p className="muted">Loading…</p>;
  const { code, name, address, location, serviceArea, isActive } = existing.data;
  return <StoreForm key={id} id={id} initial={{ code, name, address, location, serviceArea, isActive }} />;
}

function StoreForm({ id, initial }: { id?: string; initial: Draft }) {
  const navigate = useNavigate();
  const save = useSaveStore();
  const [draft, setDraft] = useState<Draft>(initial);
  const [radiusKm, setRadiusKm] = useState("3");
  const [formError, setFormError] = useState<string>();

  const set = <K extends keyof Draft>(key: K, value: Draft[K]) => setDraft((d) => ({ ...d, [key]: value }));

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setFormError(undefined);
    if (!draft.location) return setFormError("Click the map (or enter coordinates) to place the store.");
    if (!draft.serviceArea) return setFormError("Draw the delivery area, or generate one around the store.");
    const body: StoreRequest = {
      code: draft.code.trim().toUpperCase(),
      name: draft.name.trim(),
      address: draft.address.trim(),
      location: draft.location,
      serviceArea: draft.serviceArea,
      isActive: draft.isActive,
    };
    save.mutate({ id, body }, { onSuccess: () => void navigate("/stores") });
  };

  return (
    <form className="card form" onSubmit={submit}>
      <div className="card-head">
        <h2>{id ? `Edit ${initial.code}` : "New dark store"}</h2>
        <Link to="/stores">Back to stores</Link>
      </div>

      <div className="grid-2">
        <Field label="Code" hint="e.g. BLR-IND-01">
          {(fid) => (
            <input id={fid} required value={draft.code} onChange={(e) => set("code", e.target.value.toUpperCase())} />
          )}
        </Field>
        <Field label="Name">
          {(fid) => <input id={fid} required value={draft.name} onChange={(e) => set("name", e.target.value)} />}
        </Field>
      </div>
      <Field label="Address">
        {(fid) => <input id={fid} required value={draft.address} onChange={(e) => set("address", e.target.value)} />}
      </Field>

      <div className="grid-3">
        <Field label="Latitude">
          {(fid) => (
            <input
              id={fid}
              inputMode="decimal"
              value={draft.location?.lat ?? ""}
              onChange={(e) => set("location", { lat: Number(e.target.value), lng: draft.location?.lng ?? 0 })}
            />
          )}
        </Field>
        <Field label="Longitude">
          {(fid) => (
            <input
              id={fid}
              inputMode="decimal"
              value={draft.location?.lng ?? ""}
              onChange={(e) => set("location", { lat: draft.location?.lat ?? 0, lng: Number(e.target.value) })}
            />
          )}
        </Field>
        <Field label="Quick area (km radius)">
          {(fid) => (
            <div className="inline">
              <input id={fid} inputMode="decimal" value={radiusKm} onChange={(e) => setRadiusKm(e.target.value)} />
              <button
                type="button"
                className="btn secondary"
                disabled={!draft.location || !(Number(radiusKm) > 0)}
                onClick={() => draft.location && set("serviceArea", hexagon(draft.location, Number(radiusKm)))}
              >
                Generate area
              </button>
            </div>
          )}
        </Field>
      </div>

      <p className="muted small">
        Click the map to place the store. Use the polygon tool (top-left) to draw the delivery area, or generate a
        hexagon and adjust it with the edit tool.
      </p>
      <PolygonEditor
        location={draft.location}
        area={draft.serviceArea}
        onLocationChange={(l) => set("location", l)}
        onAreaChange={(a) => set("serviceArea", a)}
      />

      <label className="checkbox">
        <input type="checkbox" checked={draft.isActive} onChange={(e) => set("isActive", e.target.checked)} />
        Active (serving customers)
      </label>

      {formError ? <p className="error">{formError}</p> : null}
      <ErrorText error={save.error} />
      <div className="actions">
        <button className="btn" disabled={save.isPending}>
          {save.isPending ? "Saving…" : "Save store"}
        </button>
      </div>
    </form>
  );
}
