export type LocalProject = {
  root: string;
  project: { id: string; name: string };
};

export type StudioView = "project" | "builder" | "simulation" | "experiences" | "loop" | "deployments" | "cloud";

export const studio = $state({
  project: null as LocalProject | null,
  activeView: "experiences" as StudioView,
  sidebarWidth: 260,
  chatOpen: false,
  chatExpanded: false,
  chatWidth: 280,
  editorDirty: false,
  editorBusy: false,
  builderDirty: false,
  builderBusy: false,
  simulationPython: '',
  simulationRuntime: null as { ready: boolean; installing: boolean; phase: string; error: string | null } | null,
  openTools: false,
  simulationBusy: false,
  simulationVersion: '',
  terminalOpen: false,
  terminalHeight: 200,
});
