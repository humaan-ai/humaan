import { primitive, pose, type Robot, type Shape } from '$lib/builder/model';
export type Item = { id: string; version: string; name: string; kind: 'part' | 'component' | 'module' | 'robot'; description: string; publisher: string; license: string; tags: string[]; compatibility: string[]; suppliers: { name: string }[] };
export type Entry = { key: string; item: Item; downloaded: boolean; robot: Robot };
function model(name: string, shapes: [Shape['kind'], number[], number[], string][]): Robot {
  const robot = primitive('box'); robot.name = name;
  robot.links[0].name = name;
  robot.links[0].visuals = shapes.map(([kind, size, xyz, color]) => ({ kind, size: size as Shape['size'], origin: { ...pose(), xyz: xyz as Shape['size'] }, color, asset: '' }));
  robot.links[0].collisions = []; return robot;
}
const gray = '#9ba8b6', dark = '#394654', blue = '#639fc4';
export function sampleCatalog(): Entry[] {
  const samples: [string, Item['kind'], string, string[], [Shape['kind'], number[], number[], string][]][] = [
    ['Compact robot arm', 'robot', 'A complete desktop robot arm concept with a gripper. Explore its assembly as a starting point for your robotics project.', ['Manipulation', 'Desktop'], [['cylinder',[.09,.03,1],[0,0,.015],dark],['box',[.06,.07,.2],[0,0,.13],gray],['box',[.19,.05,.05],[.075,0,.25],blue],['box',[.04,.09,.04],[.18,0,.25],dark],['box',[.07,.015,.025],[.22,.04,.25],gray],['box',[.07,.015,.025],[.22,-.04,.25],gray]]],
    ['Parallel gripper', 'module', 'A two-finger end effector concept with a compact mounting body and replaceable fingertips.', ['Gripper', 'Manipulation'], [['box',[.08,.1,.045],[0,0,.023],dark],['box',[.025,.025,.1],[0,.04,.095],gray],['box',[.025,.025,.1],[0,-.04,.095],gray],['box',[.035,.03,.025],[0,.027,.15],blue],['box',[.035,.03,.025],[0,-.027,.15],blue]]],
    ['Depth camera', 'component', 'A stereo sensor concept for perception. Preview the housing, mounting footprint, and connection information.', ['Perception', 'Camera'], [['box',[.14,.035,.035],[0,0,.025],gray],['box',[.023,.009,.023],[-.047,-.02,.025],dark],['box',[.023,.009,.023],[.047,-.02,.025],dark],['box',[.04,.03,.01],[0,0,.005],blue]]],
    ['Smart servo', 'component', 'An actuator concept with an output hub. Review its form factor alongside your assembly.', ['Actuator', 'Motor'], [['box',[.055,.04,.065],[0,0,.035],dark],['cylinder',[.02,.015,1],[0,0,.075],gray],['cylinder',[.008,.025,1],[0,0,.09],blue]]],
    ['Mounting bracket', 'part', 'An L-shaped mounting concept for connecting components to a robot frame.', ['Mount', 'Mechanical'], [['box',[.1,.07,.008],[0,0,.004],gray],['box',[.008,.07,.08],[-.046,0,.044],blue]]],
    ['Mobile base', 'module', 'A small wheeled platform concept for navigation and mobile manipulation projects.', ['Navigation', 'Mobile'], [['box',[.28,.2,.06],[0,0,.075],gray],['box',[.19,.14,.035],[0,0,.12],blue],['sphere',[.055,1,1],[-.08,-.11,.055],dark],['sphere',[.055,1,1],[.08,-.11,.055],dark],['sphere',[.055,1,1],[-.08,.11,.055],dark],['sphere',[.055,1,1],[.08,.11,.055],dark]]],
  ];
  return samples.map(([name, kind, description, tags, shapes], i) => ({ key: `sample-${i}`, downloaded: i === 4, robot: model(name, shapes), item: { id: `sample-${i}`, name, kind, description, version: '1.0.0', publisher: 'Humaan · Sample collection', license: 'Demo content · License information placeholder', tags, compatibility: ['Concept geometry only; dimensions and specifications are illustrative.', 'Hardware and simulation compatibility have not been verified.'], suppliers: [{ name: 'View supplier' }] } }));
}
export function filtered(entries: Entry[], query: string, kind: string, availability: string): Entry[] {
  const q = query.trim().toLowerCase();
  return entries.filter(e => (kind === 'all' || e.item.kind === kind) && (availability !== 'offline' || e.downloaded) && `${e.item.name} ${e.item.publisher} ${e.item.description} ${e.item.tags.join(' ')}`.toLowerCase().includes(q));
}
