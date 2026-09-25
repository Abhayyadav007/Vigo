/**
 * Local uploads come back as `/media/...`; resolve them against the API origin
 * (which may be a LAN IP on devices). Absolute URLs pass through.
 */
export function resolveMediaUrl(url: string | null | undefined, apiBaseUrl: string): string | undefined {
  if (!url) return undefined;
  return url.startsWith("/") ? `${apiBaseUrl.replace(/\/$/, "")}${url}` : url;
}
