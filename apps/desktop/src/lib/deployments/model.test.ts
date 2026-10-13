import { test } from 'node:test';
import assert from 'node:assert/strict';
import { primitive, attach } from '../builder/model';
import { mapTelemetry, type Profile } from './model';
function fixture(): Profile {
  const robot = primitive('box'); attach(robot, primitive('cylinder'), robot.links[0].id, 'origin', 'origin');
  robot.joints[0].type = 'revolute'; robot.joints[0].id = 'joint';
  return { id: 'robot', name: 'Robot', robot, port: 9876, versionId: 'version', mappings: [{ jointId: 'joint', channel: 'motor', unit: 'deg', direction: -1, offset: 0.1 }] };
}
test('telemetry converts degrees, direction and calibrated offset', () => {
  assert.ok(Math.abs(mapTelemetry(fixture(), { motor: 90 }).joint - (-Math.PI / 2 + 0.1)) < 1e-10);
});
test('missing, non-finite and out-of-range telemetry cannot move the viewport', () => {
  for (const sample of ([{}, { motor: NaN }, { motor: Infinity }, { motor: 900 }] as Record<string, number>[])) assert.throws(() => mapTelemetry(fixture(), sample));
});
test('continuous angles are not clipped to revolute limits', () => {
  const profile = fixture(); profile.robot.joints[0].type = 'continuous';
  assert.ok(Number.isFinite(mapTelemetry(profile, { motor: 900 }).joint));
});
