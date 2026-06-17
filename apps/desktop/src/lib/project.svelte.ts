import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileEntry[];
}

export const projectState = $state({
  rootPath: null as string | null,
  files: [] as FileEntry[],
  openFiles: [] as string[],
  selectedFile: null as string | null,
  fileContent: "" as string,
  isLoading: false,
  isDirty: false,
  
  async selectFile(path: string | null) {
    if (path) {
      if (!this.openFiles.includes(path)) {
        this.openFiles.push(path);
      }
      if (this.selectedFile === path) return;
      this.selectedFile = path;
      await this.loadFile(path);
    } else {
      this.selectedFile = null;
      this.fileContent = "";
      this.isDirty = false;
    }
  },
  
  closeFile(path: string) {
    this.openFiles = this.openFiles.filter((p) => p !== path);
    if (this.selectedFile === path) {
      if (this.openFiles.length > 0) {
        this.selectFile(this.openFiles[this.openFiles.length - 1]);
      } else {
        this.selectFile(null);
      }
    }
  },
  
  async openFolder() {
    try {
      const selected = await invoke<string | null>("select_directory");
      if (selected) {
        this.rootPath = selected;
        this.selectedFile = null; // Clear active file
        this.openFiles = []; // Clear open tabs
        await this.refresh();
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  },
  
  async refresh() {
    if (!this.rootPath) return;
    try {
      const entries = await invoke<FileEntry[] | null>("read_directory", { path: this.rootPath });
      if (entries) {
        this.files = entries;
      }
    } catch (err) {
      console.error("Failed to read directory:", err);
    }
  },
  
  async loadFile(path: string) {
    this.isLoading = true;
    try {
      const content = await invoke<string>("read_file", { path });
      this.fileContent = content;
      this.isDirty = false;
    } catch (err) {
      console.error("Failed to read file:", err);
      this.fileContent = `Error reading file: ${err}`;
    } finally {
      this.isLoading = false;
    }
  },
  
  async saveFile() {
    if (!this.selectedFile) return;
    try {
      await invoke("write_file", { path: this.selectedFile, content: this.fileContent });
      this.isDirty = false;
    } catch (err) {
      console.error("Failed to save file:", err);
    }
  }
});
