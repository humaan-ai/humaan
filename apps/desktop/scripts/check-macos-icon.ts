import { spawnSync } from "node:child_process";

// Tauri can silently skip the layered icon when actool is unavailable.
// Stop packaging so a successful release build always includes the native icon.
if (process.platform === "darwin") {
  const result = spawnSync("xcrun", ["actool", "--version", "--output-format=human-readable-text"], { encoding: "utf8" });
  if (result.error || result.status !== 0) {
    console.error("Native macOS icon packaging requires full Xcode with Icon Composer support. Install Xcode, open it to complete setup, and select its developer directory before bundling.");
    console.error(result.error?.message ?? result.stderr);
    process.exit(1);
  }
}
