export function statusRank(key: "outcome" | "review_state" | "assistance", value: string): number | null {
  const order = {
    outcome: ["success", "failure", "timeout", "aborted"],
    review_state: ["unreviewed", "reviewed"],
    assistance: ["yes", "no"],
  };
  const index = order[key].indexOf(value);
  return index < 0 ? null : index;
}
