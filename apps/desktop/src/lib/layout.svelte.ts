export type ViewId = "experiences" | "project" | "assembly" | "simulation";
export type BottomTab = "problems" | "output" | "debug" | "terminal";

export const layout = $state({
  activeView: "experiences" as ViewId,
  sidebarOpen: true,
  sidebarWidth: 240,
  rightPanelOpen: false,
  rightPanelWidth: 288,
  bottomPanelOpen: false,
  bottomPanelHeight: 192,
  bottomTab: "debug" as BottomTab,
  isImmersive3D: false,
});
