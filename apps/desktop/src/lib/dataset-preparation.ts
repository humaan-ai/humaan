export type PreparationToolStatus = { ready: boolean; custom: boolean; installing: boolean; phase: string; error: string | null };
export function eligibilityReason(review: { review_state: string; outcome: string; assistance: string }): string | null {
  if (review.review_state !== "reviewed") return "The saved review is unreviewed.";
  if (review.outcome !== "success") return `The saved outcome is ${review.outcome}; this exporter requires success.`;
  if (review.assistance !== "no") return `Saved human assistance is ${review.assistance}; this exporter requires No.`;
  return null;
}
export function preparationBlockers(tools: PreparationToolStatus | null, eligible: number, train: number, validation: number, approved: boolean): string[] {
  const reasons: string[] = [];
  if (!tools) reasons.push("Check dataset tool status using Refresh tool status.");
  else if (tools.installing) reasons.push("Wait for dataset tool setup to finish.");
  else if (!tools.ready) reasons.push(tools.custom ? "Fix the configured custom Python path, or unset it to use managed tools." : "Click Set up dataset tools and wait until setup finishes.");
  if (eligible < 2) reasons.push("This saved set needs at least two reviewed, successful experiences with human assistance set to No. Create a new set after correcting reviews; existing sets keep their saved reviews.");
  else {
    if (!train) reasons.push("Change at least one episode from Exclude to Train.");
    if (!validation) reasons.push("Assign a different episode to Validation.");
  }
  if (!approved) reasons.push("Check the confirmation that the selected whole trajectories are suitable for imitation.");
  return reasons;
}
