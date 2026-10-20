const rerunPathKey = "humaan.tools.rerunPath";

export function readRerunPath(): string {
  return localStorage.getItem(rerunPathKey) ?? "";
}

export function saveRerunPath(path: string): void {
  if (path) localStorage.setItem(rerunPathKey, path);
  else localStorage.removeItem(rerunPathKey);
}
