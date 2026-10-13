import { roots, validate, type Robot } from '$lib/builder/model';
export type Finding = { severity: 'error' | 'warning'; message: string; linkId?: string };
export type RunConfig = { duration: number; gravity: number; targets: Record<string, number> };
export type Frame = { time: number; poses: Record<string, { position: [number, number, number]; quaternion: [number, number, number, number] }>; joints: Record<string, number> };
export type RunSummary = { id: string; versionId: string; name: string; createdAt: number; status: 'completed' | 'failed' };
export type SimulationRun = RunSummary & { robot: Robot; config: RunConfig; result: { floorHeight?: number; engine?: string; timestep?: number; frames?: Frame[]; error?: string; warnings?: string[]; maxContacts?: number; modelXml?: string } };
export function simulationChecks(robot: Robot): Finding[] {
  const findings: Finding[] = validate(robot).map(message => ({ severity: 'error', message, linkId: robot.links.find(l => message.startsWith(`${l.name}:`))?.id || robot.joints.find(j => message.startsWith(`${j.name}:`))?.child }));
  const rootIds = new Set(roots(robot).map(l => l.id));
  for (const link of robot.links) {
    if (!rootIds.has(link.id)) {
      if (!(link.mass > 0)) findings.push({ severity: 'error', linkId: link.id, message: `${link.name}: enter a positive mass for simulation.` });
      const [a, b, c, d, e, f] = link.inertia;
      const det = a*d*f + 2*b*c*e - a*e*e - d*c*c - f*b*b;
      // Both I and trace(I)/2 * identity - I must be positive (semi)definite.
      const trace = (a+d+f)/2, x = trace-a, y = trace-d, z = trace-f;
      const tolerance = Math.max(a,d,f, 1e-30) * 1e-8;
      const physical = x >= -tolerance && y >= -tolerance && z >= -tolerance && x*y-b*b >= -tolerance*trace && x*z-c*c >= -tolerance*trace && y*z-e*e >= -tolerance*trace && x*y*z-2*b*c*e-x*e*e-y*c*c-z*b*b >= -tolerance*trace*trace;
      if (!(a > 0 && a*d-b*b > 0 && det > 0 && physical)) findings.push({ severity: 'error', linkId: link.id, message: `${link.name}: enter a positive, physically valid inertia tensor.` });
    }
    if (!link.collisions.length) findings.push({ severity: 'warning', linkId: link.id, message: `${link.name}: no collision geometry; this link will not contact the environment.` });
    if (link.collisions.some(s => s.kind === 'mesh')) findings.push({ severity: 'warning', linkId: link.id, message: `${link.name}: mesh collisions use convex hulls; inspect concave parts carefully.` });
  }
  for (const joint of robot.joints) if (joint.type !== 'fixed') {
    if (joint.type !== 'continuous' && joint.lower >= joint.upper) findings.push({ severity: 'error', linkId: joint.child, message: `${joint.name}: lower limit must be below upper limit; use a fixed joint for locked motion.` });
    if (!(joint.effort > 0 && joint.velocity > 0)) findings.push({ severity: 'error', linkId: joint.child, message: `${joint.name}: set positive effort and velocity limits.` });
    if (joint.type !== 'continuous' && (joint.lower > 0 || joint.upper < 0)) findings.push({ severity: 'warning', linkId: joint.child, message: `${joint.name}: zero lies outside its limits; simulation starts at the nearest limit.` });
  }
  return findings;
}
