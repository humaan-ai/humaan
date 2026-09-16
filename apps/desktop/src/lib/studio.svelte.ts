export type LocalProject = {
  root: string;
  project: { id: string; name: string };
};

export const studio = $state({
  project: null as LocalProject | null,
  activeView: "experiences" as "experiences" | "loop",
  sidebarWidth: 260,
});
