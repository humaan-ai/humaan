import type { Robot } from '$lib/builder/model';
import type { Frame } from '$lib/simulation/model';
export type Definition = { id: string; name: string; createdAt: number };
export type Environment = Definition & { kind: string; gravity: number; duration: number };
export type Task = Definition & { kind: string; targetFraction: number; tolerance: number };
export type Loop = Definition & { experienceId: string };
export type Experiment = Definition & { experience: Definition; loop: Loop; environment: Environment; task: Task; versionId: string; robot?: Robot; robotName?: string; config: { algorithm: string; iterations: number; population: number; seed: number } };
export type Evaluation = { successRate: number; meanError: number; meanLoss: number; seeds: number[] };
export type TrainingRun = Definition & { experimentId: string; versionId: string; robotName: string; experienceId: string; loopId: string; status: string; error?: string; execution: { kind: string; host?: string; python?: string }; experiment: Experiment & { robot: Robot }; evaluation?: Evaluation; comparisonKey?: string; result?: { error?: string; engine: string; evaluation: Evaluation; baseline: Evaluation; comparisonKey: string; policy: { format: string; jointIds: string[]; gain: number[]; damping: number[]; target: number[] }; history: { iteration: number; loss: number }[]; frames: Frame[]; warnings: string[] } };
export type Catalog = { experiences: Definition[]; loops: Loop[]; environments: Environment[]; tasks: Task[]; experiments: Experiment[]; runs: TrainingRun[] };
export function comparableRuns(runs: TrainingRun[], selected: TrainingRun | null): TrainingRun[] {
  const key = selected?.result?.comparisonKey ?? selected?.comparisonKey;
  return key ? runs.filter(run => run.status === 'completed' && run.comparisonKey === key) : [];
}
