/**
 * Update checks.
 *
 * By default the app reads the newest GitHub Release for this repository, so a
 * tag pushed by `.github/workflows/release.yml` becomes the update offer. Users
 * may override the source in Settings with any URL returning `{ "version": ... }`.
 */

export const GITHUB_REPO = "hn1318/AutoClicker";
export const GITHUB_LATEST_API = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
export const GITHUB_RELEASES_PAGE = `https://github.com/${GITHUB_REPO}/releases/latest`;

export interface UpdateCandidate {
  /** Normalized to `x.y.z` (no leading `v`). */
  version: string;
  /** Page or asset to open when the user accepts the update. */
  url: string;
  /** Release notes / manifest description, may be empty. */
  notes: string;
}

function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** Parse either a GitHub Release payload or a minimal `{ version }` manifest. */
function parseCandidate(data: unknown, fromGitHub: boolean): UpdateCandidate {
  if (typeof data !== "object" || data === null) throw new Error("invalid payload");
  const record = data as Record<string, unknown>;

  const rawVersion = fromGitHub
    ? text(record.tag_name) || text(record.name)
    : text(record.version) || text(record.tag_name);
  const version = rawVersion.trim().replace(/^v/i, "");
  if (version === "") throw new Error("missing version");

  return {
    version,
    url: text(record.html_url) || text(record.url) || GITHUB_RELEASES_PAGE,
    notes: text(record.body) || text(record.notes),
  };
}

export async function fetchLatestRelease(source: string): Promise<UpdateCandidate> {
  const custom = source.trim();
  const fromGitHub = custom === "";

  const response = await fetch(fromGitHub ? GITHUB_LATEST_API : custom, {
    cache: "no-store",
    headers: fromGitHub ? { Accept: "application/vnd.github+json" } : undefined,
  });

  if (!response.ok) {
    // 404 means "no release published yet" — surface it as a plain failure.
    throw new Error(`HTTP ${response.status}`);
  }

  return parseCandidate(await response.json(), fromGitHub);
}
