import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attach, clone, matrix, primitive, removeLink, validate } from './model';

test('attachment aligns named mount frames including their rotations', () => {
  const robot = primitive('box'), module = primitive('cylinder');
  robot.links[0].mounts[0].origin = { xyz: [1, 2, 3], rpy: [0.2, 0.4, 0.6] };
  module.links[0].mounts[0].origin = { xyz: [0, 0, 0.5], rpy: [0, 0, Math.PI / 2] };
  attach(robot, module, robot.links[0].id, 'origin', 'origin');
  const actual = matrix(robot.joints[0].origin).multiply(matrix(module.links[0].mounts[0].origin));
  const expected = matrix(robot.links[0].mounts[0].origin);
  actual.elements.forEach((v, i) => assert.ok(Math.abs(v - expected.elements[i]) < 1e-10));
  assert.deepEqual(validate(robot), []);
});

test('reusing an assembly remaps identities and retains properties without mutating its source', () => {
  const module = primitive('box');
  const child = attach(module, primitive('sphere'), module.links[0].id, 'origin', 'origin');
  module.links.find(l => l.id === child)!.sensors.push({ name: 'camera', type: 'camera', topic: '/camera', rate: 30, origin: { xyz: [0, 0, 1], rpy: [0, 0, 0] } });
  const before = structuredClone(module), robot = primitive('box');
  attach(robot, module, robot.links[0].id, 'origin', 'origin');
  attach(robot, module, robot.links[0].id, 'origin', 'origin');
  assert.deepEqual(module, before);
  assert.equal(robot.links.length, 5);
  assert.equal(robot.links.filter(l => l.sensors.length).length, 2);
  assert.deepEqual(validate(robot), []);
});

test('cycles and invalid limits are rejected; subtree removal leaves siblings intact', () => {
  const robot = primitive('box'), base = robot.links[0].id;
  const arm = attach(robot, primitive('cylinder'), base, 'origin', 'origin');
  const finger = attach(robot, primitive('sphere'), arm, 'origin', 'origin');
  const sibling = attach(robot, primitive('box'), base, 'origin', 'origin');
  const invalid = clone(robot); invalid.joints[0].parent = finger;
  assert.ok(validate(invalid).some(s => s.includes('cycle')));
  robot.joints[0].lower = 10; robot.joints[0].upper = -10;
  assert.ok(validate(robot).some(s => s.includes('limits')));
  removeLink(robot, arm);
  assert.deepEqual(new Set(robot.links.map(l => l.id)), new Set([base, sibling]));
  assert.deepEqual(validate(robot), []);
});
