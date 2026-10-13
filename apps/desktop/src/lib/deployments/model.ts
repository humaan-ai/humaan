import type { Robot } from '$lib/builder/model';
export type Mapping = { jointId: string; channel: string; unit: 'rad' | 'deg' | 'm'; direction: number; offset: number };
export type Profile = { id: string; name: string; versionId: string; robot: Robot; port: number; mappings: Mapping[] };
export type Frame = { time: number; positions: Record<string, number> };
export type Session = { id: string; name: string; createdAt: number; profile: Profile; frames: Frame[]; reason: string };
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
