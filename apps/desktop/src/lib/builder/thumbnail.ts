import * as THREE from 'three';
import { STLLoader } from 'three/addons/loaders/STLLoader.js';
import { OBJLoader } from 'three/addons/loaders/OBJLoader.js';
import { matrix, roots, type Robot } from './model';

// All catalog cards share one offscreen context; cards display static snapshots.
let renderer: THREE.WebGLRenderer | undefined;
export function thumbnail(robot: Robot): string {
  renderer ??= new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
  renderer.setSize(400, 240); renderer.setClearColor('#181818');
  const scene = new THREE.Scene(), assembly = new THREE.Group();
  scene.add(assembly, new THREE.HemisphereLight('#ffffff', '#65717f', 2));
  const light = new THREE.DirectionalLight('#ffffff', 3); light.position.set(3, -4, 6); scene.add(light);
  const seen = new Set<string>();
  function add(id: string, parent: THREE.Object3D) {
    if (seen.has(id)) return; seen.add(id);
    const link = robot.links.find(link => link.id === id); if (!link) return;
    for (const shape of link.visuals) {
      let object: THREE.Object3D;
      if (shape.kind === 'mesh') {
        const asset = robot.assets[shape.asset]; if (!asset) throw new Error('Missing mesh');
        const bytes = Uint8Array.from(atob(asset.data), c => c.charCodeAt(0));
        object = asset.format === 'stl' ? new THREE.Mesh(new STLLoader().parse(bytes.buffer)) : new OBJLoader().parse(new TextDecoder().decode(bytes));
        object.scale.set(...shape.size);
      } else {
        const geometry = shape.kind === 'box' ? new THREE.BoxGeometry(...shape.size) : shape.kind === 'sphere' ? new THREE.SphereGeometry(shape.size[0], 24, 16) : new THREE.CylinderGeometry(shape.size[0], shape.size[0], shape.size[1], 32);
        if (shape.kind === 'cylinder') geometry.rotateX(Math.PI / 2);
        object = new THREE.Mesh(geometry);
      }
      object.traverse(node => { if (node instanceof THREE.Mesh) {
        const materials = Array.isArray(node.material) ? node.material : [node.material]; materials.forEach(material => material.dispose());
        node.material = new THREE.MeshStandardMaterial({ color: shape.color, metalness: 0.15, roughness: 0.7, side: THREE.DoubleSide });
      } });
      const frame = new THREE.Group(); frame.applyMatrix4(matrix(shape.origin)); frame.add(object); parent.add(frame);
    }
    for (const joint of robot.joints.filter(joint => joint.parent === id)) {
      const frame = new THREE.Group(); frame.applyMatrix4(matrix(joint.origin)); parent.add(frame); add(joint.child, frame);
    }
  }
  try {
    for (const root of roots(robot)) add(root.id, assembly);
    const bounds = new THREE.Box3().setFromObject(assembly); if (bounds.isEmpty()) throw new Error('No visual geometry');
    const center = bounds.getCenter(new THREE.Vector3()), radius = Math.max(bounds.getSize(new THREE.Vector3()).length(), 0.001);
    const camera = new THREE.PerspectiveCamera(40, 400 / 240, radius / 1000, radius * 100);
    camera.up.set(0, 0, 1); camera.position.copy(center).add(new THREE.Vector3(1, -1.5, 1).normalize().multiplyScalar(radius * 1.6)); camera.lookAt(center);
    renderer.render(scene, camera); return renderer.domElement.toDataURL('image/png');
  } finally {
    assembly.traverse(node => { if (node instanceof THREE.Mesh) { node.geometry.dispose(); (Array.isArray(node.material) ? node.material : [node.material]).forEach(material => material.dispose()); } });
    renderer.renderLists.dispose();
  }
}
