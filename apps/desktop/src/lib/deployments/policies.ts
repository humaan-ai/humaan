import type { TrainingRun } from '$lib/training/model';
export type PolicySummary = { id: string; name: string; createdAt: number; runId: string; versionId: string; robotName: string; evaluation?: { successRate: number; meanError: number } };
export type Policy = PolicySummary & { run: TrainingRun };
export type AssignmentEvent = { id: string; policyId: string | null; policyName: string | null; createdAt: number; action: 'assign' | 'unassign' | 'rollback'; restoredFrom: string | null };
export type Assignment = { robotId: string; history: AssignmentEvent[] };
export type Compatibility = { compatible: boolean; issues: string[]; runtime: 'unavailable' };
