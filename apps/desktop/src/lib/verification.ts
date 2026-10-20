export type VerificationReport = {
  id: string; valid: boolean; error?: string | null; checked_at_unix_ms: number;
  status: "verified" | "failed" | "needs_recheck";
};
