export type LocalProject = {
  root: string;
  project: { id: string; name: string };
};

export type StudioView = "project" | "builder" | "simulation" | "experiences" | "loop" | "deployments" | "cloud" | "catalog";

export const studio = $state({
  project: null as LocalProject | null,
  activeView: "experiences" as StudioView,
  deploymentSection: "robots" as "robots" | "policies" | "sessions",
  deploymentBusy: false,
  policyBusy: false,
  policyRunId: "",
  policyRobotId: "",
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
  cloudExperimentId: '',
  cloudAccount: null as { accountId: string; gpus: { id: string; displayName: string; memoryInGb: number; secureCloud: boolean }[] } | null,
  trainingBusy: false,
  trainingDirty: false,
  loopSection: 'learning-sets' as 'learning-sets' | 'training',
  simulationBusy: false,
  simulationVersion: '',
  terminalOpen: false,
  terminalHeight: 200,
  bottomPanelTab: 'terminal',
});
