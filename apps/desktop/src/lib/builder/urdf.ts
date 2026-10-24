import { emptyRobot, pose, validate, type Robot, type Pose, type Vec3, type Shape, type Joint } from './model';
const extensionImportNote = 'Some simulator or hardware settings are preserved in your URDF but aren’t used by Humaan yet. You can continue editing the robot.';
export function formatImportNote(note: string) {
  // Drafts may contain the previous extension notice; update its wording without reimporting.
  return note.startsWith('Gazebo, transmission, and ros2_control extensions are retained') ? extensionImportNote : note;
}
const child = (e: Element, tag: string) => Array.from(e.children).find(c => c.tagName === tag);
const children = (e: Element, tag: string) => Array.from(e.children).filter(c => c.tagName === tag);
function number(value: string | null | undefined, fallback = 0) { const n = value == null ? fallback : Number(value); if (!Number.isFinite(n)) throw new Error(`Invalid number: ${value}`); return n; }
function vector(value: string | null | undefined, fallback: Vec3 = [0, 0, 0]): Vec3 {
  if (!value) return [...fallback]; const parts = value.trim().split(/\s+/).map(v => number(v));
  if (parts.length !== 3) throw new Error(`Expected three coordinates: ${value}`); return parts as Vec3;
}
function origin(e: Element): Pose { const o = child(e, 'origin'); return { xyz: vector(o?.getAttribute('xyz')), rpy: vector(o?.getAttribute('rpy')) }; }
function normalize(path: string) { const stack: string[] = []; for (const part of path.replaceAll('\\', '/').split('/')) { if (part === '..') stack.pop(); else if (part && part !== '.') stack.push(part); } return stack.join('/'); }
function bytesToBase64(bytes: Uint8Array) { let binary = ''; for (let i = 0; i < bytes.length; i += 8192) binary += String.fromCharCode(...bytes.subarray(i, i + 8192)); return btoa(binary); }
export type UrdfFile = Pick<File, 'name' | 'webkitRelativePath' | 'size' | 'text' | 'arrayBuffer'>;
export function resolveUrdfMesh(files: UrdfFile[], base: string, name: string): UrdfFile {
  const resource = name.replace(/^(package|file):\/\//, '');
  const paths = [normalize(`${base}/${resource}`), normalize(resource)];
  let matches = files.filter(f => paths.includes(normalize(f.webkitRelativePath || f.name)));
  if (!matches.length) matches = files.filter(f => (f.webkitRelativePath || f.name).endsWith('/' + resource) || f.name === resource.split('/').at(-1));
  if (matches.length !== 1) throw new Error(`${matches.length ? 'Ambiguous' : 'Missing'} mesh: ${name}. Include its mesh files in the import.`);
  return matches[0];
}
export async function importUrdf(files: UrdfFile[], filename?: string): Promise<{ robot: Robot; warnings: string[] }> {
  const candidates = files.filter(f => /\.urdf$/i.test(f.name));
  const file = filename ? candidates.find(f => (f.webkitRelativePath || f.name) === filename) : candidates.length === 1 ? candidates[0] : undefined;
  if (!file) throw new Error(candidates.length > 1 ? 'Choose one URDF from the folder.' : 'Choose a URDF file with its STL or OBJ meshes. Expand Xacro to URDF before importing.');
  let loadedSize = file.size;
  if (loadedSize > 64 * 1024 * 1024) throw new Error('Import up to 64 MB of robot files at a time.');
  const text = await file.text();
  const doc = new DOMParser().parseFromString(text, 'application/xml');
  if (doc.querySelector('parsererror') || doc.documentElement.tagName !== 'robot') throw new Error('The file is not valid URDF XML.');
  if (Array.from(doc.getElementsByTagName('*')).some(e => e.tagName.startsWith('xacro:'))) throw new Error('Expand Xacro to URDF before importing.');
  const element = doc.documentElement, robot = emptyRobot(), warnings: string[] = [];
  const meshLoads = new Map<string, Promise<void>>();
  robot.name = element.getAttribute('name') || file.name.replace(/\.urdf$/i, ''); robot.source = text;
  const base = (file.webkitRelativePath || file.name).split('/').slice(0, -1).join('/');
  async function geometry(e: Element): Promise<Shape> {
    const g = child(e, 'geometry')?.firstElementChild;
    if (!g || !['box', 'sphere', 'cylinder', 'mesh'].includes(g.tagName)) throw new Error('Unsupported or missing URDF geometry.');
    for (const attribute of g.tagName === 'box' ? ['size'] : g.tagName === 'sphere' ? ['radius'] : g.tagName === 'cylinder' ? ['radius', 'length'] : ['filename']) { if (!g.getAttribute(attribute)?.trim()) throw new Error(`${g.tagName} geometry requires ${attribute}.`); }
    const shape: Shape = { kind: g.tagName as Shape['kind'], size: [1, 1, 1], asset: '', origin: origin(e), color: '#8ca8c3' };
    if (shape.kind === 'box') shape.size = vector(g.getAttribute('size'), [1, 1, 1]);
    if (shape.kind === 'sphere') shape.size = [number(g.getAttribute('radius'), 0.1), 1, 1];
    if (shape.kind === 'cylinder') shape.size = [number(g.getAttribute('radius'), 0.1), number(g.getAttribute('length'), 0.1), 1];
    const material = child(e, 'material');
    const definition = material && (child(material, 'color') ? material : children(element, 'material').find(m => m.getAttribute('name') === material.getAttribute('name')));
    const rgba = definition && child(definition, 'color')?.getAttribute('rgba')?.split(/\s+/).map(Number);
    if (rgba && rgba.length >= 3) shape.color = '#' + rgba.slice(0, 3).map(v => Math.round(Math.max(0, Math.min(1, v)) * 255).toString(16).padStart(2, '0')).join('');
    if (shape.kind === 'mesh') {
      const mesh = resolveUrdfMesh(files, base, g.getAttribute('filename') || ''), format = mesh.name.split('.').at(-1)?.toLowerCase();
      if (format !== 'stl' && format !== 'obj') throw new Error(`Mesh format .${format} is not supported yet. Convert ${mesh.name} to STL or OBJ.`);
      shape.asset = mesh.webkitRelativePath || mesh.name; shape.size = vector(g.getAttribute('scale'), [1, 1, 1]);
      if (!meshLoads.has(shape.asset)) {
        meshLoads.set(shape.asset, (async () => {
          loadedSize += mesh.size;
          if (loadedSize > 64 * 1024 * 1024) throw new Error('Import up to 64 MB of referenced robot files at a time.');
          robot.assets[shape.asset] = { format, data: bytesToBase64(new Uint8Array(await mesh.arrayBuffer())) };
        })());
      }
      await meshLoads.get(shape.asset);
    }
    return shape;
  }
  for (const e of children(element, 'link')) {
    const name = e.getAttribute('name'); if (!name) throw new Error('Every link needs a name.');
    const inertial = child(e, 'inertial'), inertia = inertial && child(inertial, 'inertia');
    if (!inertial) warnings.push(`${name}: no inertial data; mass and inertia are zero. Configure them before simulation.`);
    robot.links.push({ id: name, name, visuals: await Promise.all(children(e, 'visual').map(geometry)), collisions: await Promise.all(children(e, 'collision').map(geometry)), mass: inertial ? number(child(inertial, 'mass')?.getAttribute('value')) : 0, inertial: inertial ? origin(inertial) : pose(), inertia: ['ixx', 'ixy', 'ixz', 'iyy', 'iyz', 'izz'].map(key => number(inertia?.getAttribute(key))) as [number, number, number, number, number, number], mounts: [{ name: 'origin', origin: pose() }], sensors: [] });
  }
  for (const e of children(element, 'joint')) {
    const type = e.getAttribute('type') as Joint['type'];
    if (!['fixed', 'revolute', 'continuous', 'prismatic'].includes(type)) throw new Error(`Unsupported joint type: ${type}.`);
    const limit = child(e, 'limit');
    const mimic = child(e, 'mimic');
    robot.joints.push({ id: e.getAttribute('name') || crypto.randomUUID(), name: e.getAttribute('name') || 'Joint', type, parent: child(e, 'parent')?.getAttribute('link') || '', child: child(e, 'child')?.getAttribute('link') || '', origin: origin(e), axis: vector(child(e, 'axis')?.getAttribute('xyz'), [1, 0, 0]), lower: number(limit?.getAttribute('lower'), -Math.PI), upper: number(limit?.getAttribute('upper'), Math.PI), effort: number(limit?.getAttribute('effort')), velocity: number(limit?.getAttribute('velocity')), hardware: '' });
    if (mimic) robot.joints.at(-1)!.mimic = { joint: mimic.getAttribute('joint') || '', multiplier: number(mimic.getAttribute('multiplier'), 1), offset: number(mimic.getAttribute('offset')) };
  }
  if (robot.joints.some(j => j.mimic)) warnings.push('Mimic joints are preserved for Builder preview. Simulation and training with mimic joints are not supported yet.');
  if (children(element, 'gazebo').length || children(element, 'transmission').length || children(element, 'ros2_control').length) warnings.push(extensionImportNote);
  const issues = validate(robot); if (issues.length) throw new Error(issues.join('\n'));
  return { robot, warnings };
}
