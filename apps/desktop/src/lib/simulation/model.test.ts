import { test } from 'node:test';
import assert from 'node:assert/strict';
import { primitive, attach } from '../builder/model';
import { simulationChecks } from './model';

test('simulation checks locate missing mass and inertia without blocking draft storage', () => {
  const robot = primitive('box');
  const id = attach(robot, primitive('sphere'), robot.links[0].id, 'origin', 'origin');
  const link = robot.links.find(l => l.id === id)!;
  assert.equal(simulationChecks(robot).filter(c => c.severity === 'error').length, 0);
  link.mass = 0; link.inertia = [0,0,0,0,0,0];
  const errors = simulationChecks(robot).filter(c => c.severity === 'error');
  assert.equal(errors.length, 2); assert.ok(errors.every(c => c.linkId === id));
});
test('nonphysical inertia and missing collision geometry are distinguished', () => {
  const robot = primitive('box');
  const id = attach(robot, primitive('sphere'), robot.links[0].id, 'origin', 'origin');
  const link = robot.links.find(l => l.id === id)!;
  link.inertia = [10,0,0,1,0,1]; link.collisions = [];
  assert.ok(simulationChecks(robot).some(c => c.severity === 'error' && c.message.includes('inertia')));
  assert.ok(simulationChecks(robot).some(c => c.severity === 'warning' && c.message.includes('collision')));
});
