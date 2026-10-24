import type { Robot } from '$lib/builder/model';
export type Mapping = { jointId: string; channel: string; unit: 'rad' | 'deg' | 'm'; direction: number; offset: number };
export type Profile = { id: string; name: string; versionId: string; robot: Robot; port: number; mappings: Mapping[] };
export type Frame = { time: number; positions: Record<string, number> };
export type Session = { id: string; name: string; createdAt: number; profile: Profile; frames: Frame[]; reason: string };
// Interpolation is only a replay presentation; recorded measurements stay intact.
export function replayPose(frames: Frame[], time: number): Record<string, number> {
  if (!frames.length) return {};
  if (time <= frames[0].time) return frames[0].positions;
  if (time >= frames[frames.length - 1].time) return frames[frames.length - 1].positions;
  let low = 0, high = frames.length - 1;
  while (low + 1 < high) {
    const mid = (low + high) >>> 1;
    if (frames[mid].time <= time) low = mid; else high = mid;
  }
  const before = frames[low], after = frames[high];
  const fraction = (time - before.time) / (after.time - before.time);
  return Object.fromEntries(Object.entries(before.positions).map(([joint, value]) =>
    [joint, value + ((after.positions[joint] ?? value) - value) * fraction]));
}
export function mapTelemetry(profile: Profile, channels: Record<string, number>): Record<string, number> {
  return Object.fromEntries(profile.mappings.map(mapping => {
    const value = channels[mapping.channel];
    if (!Number.isFinite(value)) throw new Error(`Missing or invalid channel: ${mapping.channel}`);
    const position = value * (mapping.unit === 'deg' ? Math.PI / 180 : 1) * mapping.direction + mapping.offset;
    const joint = profile.robot.joints.find(j => j.id === mapping.jointId);
    if (!joint || !Number.isFinite(position)) throw new Error('Invalid joint mapping.');
    if (joint.type !== 'continuous' && (position < joint.lower - 0.01 || position > joint.upper + 0.01)) throw new Error(`${joint.name}: telemetry exceeds saved joint limits. Check calibration.`);
    return [mapping.jointId, position];
  }));
}
