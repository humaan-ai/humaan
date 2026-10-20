import { Euler, Matrix4, Quaternion, Vector3 } from 'three';

export type Vec3 = [number, number, number];
export type Pose = { xyz: Vec3; rpy: Vec3 };
export type Shape = { kind: 'box' | 'cylinder' | 'sphere' | 'mesh'; size: Vec3; asset: string; origin: Pose; color: string };
export type Mount = { name: string; origin: Pose };
export type Sensor = { name: string; type: string; topic: string; rate: number; origin: Pose };
export type Link = { id: string; name: string; visuals: Shape[]; collisions: Shape[]; mass: number; inertial: Pose; inertia: [number, number, number, number, number, number]; mounts: Mount[]; sensors: Sensor[] };
export type Joint = { id: string; name: string; parent: string; child: string; type: 'fixed' | 'revolute' | 'continuous' | 'prismatic'; origin: Pose; axis: Vec3; lower: number; upper: number; effort: number; velocity: number; hardware: string; mimic?: { joint: string; multiplier: number; offset: number } };
export type Robot = { placement?: Vec3; schema: 1; name: string; links: Link[]; joints: Joint[]; assets: Record<string, { format: 'stl' | 'obj'; data: string }>; source?: string };
export type SavedVersion = { id: string; name: string; savedAt: number; links: number };
export const pose = (): Pose => ({ xyz: [0, 0, 0], rpy: [0, 0, 0] });
export const emptyRobot = (): Robot => ({ schema: 1, name: 'Untitled robot', links: [], joints: [], assets: {} });
export const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
export function jointPosition(robot: Robot, id: string, positions: Record<string, number>, seen = new Set<string>()): number {
  const joint = robot.joints.find(j => j.id === id);
  if (!joint || seen.has(id)) return 0;
  seen.add(id);
  if (joint.mimic) return jointPosition(robot, joint.mimic.joint, positions, seen) * joint.mimic.multiplier + joint.mimic.offset;
  return positions[id] ?? 0;
}
export function primitive(kind: 'box' | 'cylinder' | 'sphere'): Robot {
  const size: Vec3 = kind === 'box' ? [0.4, 0.3, 0.15] : kind === 'cylinder' ? [0.1, 0.08, 1] : [0.1, 1, 1];
  const shape: Shape = { kind, size, asset: '', origin: pose(), color: '#8ca8c3' };
  const [x, y, z] = size;
  const diagonal = kind === 'box' ? [(y*y+z*z)/12, (x*x+z*z)/12, (x*x+y*y)/12] : kind === 'sphere' ? [0.4*x*x, 0.4*x*x, 0.4*x*x] : [(3*x*x+y*y)/12, (3*x*x+y*y)/12, x*x/2];
  return { ...emptyRobot(), name: kind, links: [{ id: crypto.randomUUID(), name: kind, visuals: [shape], collisions: [clone(shape)], mass: 1, inertial: pose(), inertia: [diagonal[0], 0, 0, diagonal[1], 0, diagonal[2]], mounts: [{ name: 'origin', origin: pose() }], sensors: [] }] };
}
export function roots(robot: Robot) { return robot.links.filter(link => !robot.joints.some(j => j.child === link.id)); }
export function matrix(p: Pose) { return new Matrix4().compose(new Vector3(...p.xyz), new Quaternion().setFromEuler(new Euler(...p.rpy, 'ZYX')), new Vector3(1, 1, 1)); }
function fromMatrix(m: Matrix4): Pose {
  const p = new Vector3(), q = new Quaternion(), s = new Vector3(); m.decompose(p, q, s);
  const e = new Euler().setFromQuaternion(q, 'ZYX');
  return { xyz: p.toArray() as Vec3, rpy: [e.x, e.y, e.z] };
}
export function attach(robot: Robot, source: Robot, parentId: string, parentMount: string, childMount: string): string {
  const problems = validate(source); if (problems.length) throw new Error(problems.join('\n'));
  if (roots(source).length !== 1) throw new Error('A reusable module must have exactly one root link.');
  const module = clone(source), ids = new Map(module.links.map(l => [l.id, crypto.randomUUID()]));
  const prefix = crypto.randomUUID();
  for (const [key, asset] of Object.entries(module.assets)) robot.assets[`${prefix}/${key}`] = asset;
  const usedNames = new Set(robot.links.map(l => l.name));
  for (const l of module.links) {
    l.id = ids.get(l.id)!;
    let name = l.name, n = 2; while (usedNames.has(name)) name = `${l.name}_${n++}`; l.name = name; usedNames.add(name);
    for (const shape of [...l.visuals, ...l.collisions]) if (shape.kind === 'mesh') shape.asset = `${prefix}/${shape.asset}`;
  }
  const jointIds = new Map(module.joints.map(j => [j.id, crypto.randomUUID()]));
  for (const j of module.joints) { j.id = jointIds.get(j.id)!; if (j.mimic) j.mimic.joint = jointIds.get(j.mimic.joint)!; j.parent = ids.get(j.parent)!; j.child = ids.get(j.child)!; j.name = `${module.name}/${j.name}/${j.id.slice(0, 4)}`; }
  const root = roots(module)[0];
  if (robot.links.length) {
    const parent = robot.links.find(l => l.id === parentId);
    if (!parent) throw new Error('Select the parent link before adding a module.');
    const target = parent.mounts.find(m => m.name === parentMount), mount = root.mounts.find(m => m.name === childMount);
    if (!target || !mount) throw new Error('Select valid parent and module mounting points.');
    robot.joints.push({ id: crypto.randomUUID(), name: `${parent.name}_to_${root.name}`, parent: parent.id, child: root.id, type: 'fixed', origin: fromMatrix(matrix(target.origin).multiply(matrix(mount.origin).invert())), axis: [0, 0, 1], lower: -Math.PI, upper: Math.PI, effort: 10, velocity: 1, hardware: '' });
  }
  robot.links.push(...module.links); robot.joints.push(...module.joints);
  return root.id;
}
export function removeLink(robot: Robot, id: string) {
  const removed = new Set([id]);
  let changed = true; while (changed) { changed = false; for (const j of robot.joints) if (removed.has(j.parent) && !removed.has(j.child)) { removed.add(j.child); changed = true; } }
  robot.links = robot.links.filter(l => !removed.has(l.id)); robot.joints = robot.joints.filter(j => !removed.has(j.parent) && !removed.has(j.child));
}
export function validate(robot: Robot): string[] {
  const issues: string[] = [];
  if (robot.schema !== 1 || !robot.name.trim()) issues.push('Enter a robot name.');
  if (robot.placement && (robot.placement.length !== 3 || !robot.placement.every(Number.isFinite))) issues.push('Robot placement must contain three finite coordinates.');
  if (!robot.links.length) issues.push('Add at least one link.');
  const ids = new Set(robot.links.map(l => l.id));
  if (ids.size !== robot.links.length) issues.push('Link IDs must be unique.');
  if (new Set(robot.links.map(l => l.name)).size !== robot.links.length) issues.push('Link names must be unique.');
  if (new Set(robot.joints.map(j => j.id)).size !== robot.joints.length) issues.push('Joint IDs must be unique.');
  const finite = (values: number[]) => values.every(Number.isFinite);
  const validPose = (p: Pose) => p.xyz.length === 3 && p.rpy.length === 3 && finite([...p.xyz, ...p.rpy]);
  for (const l of robot.links) {
    if (!l.name.trim() || !Number.isFinite(l.mass) || l.mass < 0 || !validPose(l.inertial) || l.inertia.length !== 6 || !finite(l.inertia)) issues.push(`${l.name}: invalid physical properties.`);
    if (l.inertia[0] < 0 || l.inertia[3] < 0 || l.inertia[5] < 0) issues.push(`${l.name}: inertia diagonal cannot be negative.`);
    if (new Set(l.mounts.map(m => m.name)).size !== l.mounts.length || l.mounts.some(m => !m.name.trim() || !validPose(m.origin))) issues.push(`${l.name}: mounting points need unique names and finite poses.`);
    if (l.sensors.some(s => !s.name.trim() || !s.type.trim() || !Number.isFinite(s.rate) || s.rate <= 0 || !validPose(s.origin))) issues.push(`${l.name}: sensors need a name, type, positive rate, and finite pose.`);
    for (const g of [...l.visuals, ...l.collisions]) {
      const dimensions = g.kind === 'box' || g.kind === 'mesh' ? g.size : g.kind === 'sphere' ? g.size.slice(0, 1) : g.size.slice(0, 2);
      if (!finite(g.size) || dimensions.some(v => v <= 0) || !validPose(g.origin)) issues.push(`${l.name}: geometry dimensions must be positive and poses finite.`);
      if (g.kind === 'mesh' && !robot.assets[g.asset]) issues.push(`${l.name}: missing mesh ${g.asset}.`);
    }
  }
  for (const j of robot.joints) {
    if (j.mimic) {
      const source = robot.joints.find(other => other.id === j.mimic!.joint);
      if (!source || source.id === j.id || source.type === 'fixed' || j.type === 'fixed' || (source.type === 'prismatic') !== (j.type === 'prismatic')) issues.push(`${j.name}: mimic must reference another movable joint with compatible units.`);
      if (!finite([j.mimic.multiplier, j.mimic.offset])) issues.push(`${j.name}: mimic multiplier and offset must be finite.`);
      const chain = new Set<string>([j.id]);
      let next = source;
      while (next) {
        if (chain.has(next.id)) { issues.push(`${j.name}: mimic joints cannot form a cycle.`); break; }
        chain.add(next.id);
        next = next.mimic ? robot.joints.find(other => other.id === next!.mimic!.joint) : undefined;
      }
    }
    if (!ids.has(j.parent) || !ids.has(j.child) || j.parent === j.child) issues.push(`${j.name}: invalid connection.`);
    if (!validPose(j.origin) || !finite([...j.axis, j.lower, j.upper, j.effort, j.velocity]) || j.lower > j.upper || j.effort < 0 || j.velocity < 0 || (j.type !== 'fixed' && Math.hypot(...j.axis) < 1e-9)) issues.push(`${j.name}: invalid joint limits, axis, or origin.`);
    if (robot.joints.filter(other => other.child === j.child).length > 1) issues.push(`${j.name}: a link can have only one parent.`);
    const seen = new Set<string>(); let current: string | undefined = j.child;
    while (current) { if (seen.has(current)) { issues.push('Connections cannot form a cycle.'); break; } seen.add(current); current = robot.joints.find(other => other.child === current)?.parent; }
  }
  if (robot.links.length && roots(robot).length !== 1) issues.push('Connect all links into one assembly.');
  return [...new Set(issues)];
}
