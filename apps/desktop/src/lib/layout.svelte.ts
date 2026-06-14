export type ViewId = "project" | "assembly" | "models" | "data" | "simulation" | "compute";
export type BottomTab = "problems" | "output" | "debug" | "terminal";

export const layout = $state({
  activeView: "project" as ViewId,
  sidebarOpen: true,
  sidebarWidth: 240,
  rightPanelOpen: false,
  rightPanelWidth: 288,
  bottomPanelOpen: true,
  bottomPanelHeight: 192,
  bottomTab: "debug" as BottomTab,
});
