import { test } from 'node:test';
import assert from 'node:assert/strict';
import { primitive, attach } from '../builder/model';
import { mapTelemetry, replayPose, type Profile } from './model';
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
test('replay interpolates irregular timestamps and preserves recorded samples', () => {
  const frames = [
    { time: 0.1, positions: { shoulder: 0, slide: 1 } },
    { time: 0.3, positions: { shoulder: 2, slide: 3 } },
    { time: 1.1, positions: { shoulder: -2, slide: 7 } },
  ];
  const original = JSON.stringify(frames);
  assert.deepEqual(replayPose(frames, 0), frames[0].positions);
  assert.ok(Math.abs(replayPose(frames, 0.2).shoulder - 1) < 1e-10);
  assert.ok(Math.abs(replayPose(frames, 0.7).shoulder) < 1e-10);
  assert.ok(Math.abs(replayPose(frames, 0.7).slide - 5) < 1e-10);
  assert.deepEqual(replayPose(frames, 2), frames[2].positions);
  assert.equal(JSON.stringify(frames), original);
  assert.deepEqual(replayPose([], 0), {});
  assert.deepEqual(replayPose([frames[0]], 4), frames[0].positions);
});
