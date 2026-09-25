import { resolveMediaUrl, useApiBaseUrl, useUploadImage } from "@vigo/api-client";
import { ErrorText } from "./ui";

const MAX = 8;

export function ImageUploader({ urls, onChange }: { urls: string[]; onChange: (urls: string[]) => void }) {
  const upload = useUploadImage();
  const base = useApiBaseUrl();

  return (
    <div className="images">
      {urls.map((url, i) => (
        <figure key={url} className="image-tile">
          <img src={resolveMediaUrl(url, base)} alt={`Product image ${i + 1}`} />
          <figcaption>
            {i === 0 ? <span className="muted small">Main</span> : (
              <button type="button" className="link small" onClick={() => onChange([url, ...urls.filter((u) => u !== url)])}>
                Make main
              </button>
            )}
            <button type="button" className="link small danger" onClick={() => onChange(urls.filter((u) => u !== url))}>
              Remove
            </button>
          </figcaption>
        </figure>
      ))}
      {urls.length < MAX ? (
        <label className="image-tile add">
          <input
            type="file"
            accept="image/jpeg,image/png,image/webp"
            aria-label="Upload image"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              e.target.value = "";
              if (file) upload.mutate(file, { onSuccess: (url) => onChange([...urls, url]) });
            }}
          />
          {upload.isPending ? "Uploading…" : "+ Add image"}
        </label>
      ) : null}
      <ErrorText error={upload.error} />
    </div>
  );
}
