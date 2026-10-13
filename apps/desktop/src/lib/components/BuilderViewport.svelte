<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Maximize from '@lucide/svelte/icons/maximize';
  import * as THREE from 'three';
  import { TransformControls } from 'three/addons/controls/TransformControls.js';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { STLLoader } from 'three/addons/loaders/STLLoader.js';
  import { OBJLoader } from 'three/addons/loaders/OBJLoader.js';
  import { matrix, roots, type Robot, type Shape } from '$lib/builder/model';
  import { Button } from '$lib/components/ui/button/index.js';

  let { robot, selected, onselect, collision = false, positions = {}, explode = false, poses, floorHeight = 0, allowGroundPlacement = false, onplacement }: { robot: Robot; selected: string; onselect: (id: string) => void; collision?: boolean; positions?: Record<string, number>; explode?: boolean; poses?: Record<string, { position: [number, number, number]; quaternion: [number, number, number, number] }>; floorHeight?: number; allowGroundPlacement?: boolean; onplacement?: (position: [number, number, number]) => void } = $props();
  let host: HTMLDivElement;
  let error = $state('');
  let ready = $state(false);
  let scene: THREE.Scene, camera: THREE.PerspectiveCamera, renderer: THREE.WebGLRenderer, controls: OrbitControls, assembly: THREE.Group;
  let transform: TransformControls;
  let guides: THREE.Group;
  let grid: THREE.GridHelper;
  $effect(() => { if (ready) grid.position.z = floorHeight; });
  let parts = new Map<string, THREE.Group>();
  let origins = new Map<string, THREE.Vector3>();
  let offsets = new Map<string, THREE.Vector3>();
  let robotKey = '';
  let extent = 1;
  let moved = $state(0);
  let selectedMoved = $state(false);
  let withChildren = $state(false);
  let dragging = false;
  let suppressPick = false;
  let dragStart = new THREE.Vector3();
  let dragOffsets = new Map<string, THREE.Vector3>();
  const joint = $derived(robot.joints.find(j => j.child === selected));
  function affected() {
    const ids = new Set([selected]);
    if (withChildren) {
      let changed = true;
      while (changed) { changed = false; for (const j of robot.joints) if (ids.has(j.parent) && !ids.has(j.child)) { ids.add(j.child); changed = true; } }
    }
    return ids;
  }
  function updatePresentation() {
    for (const [id, part] of parts) part.position.copy(origins.get(id)!).add(explode ? offsets.get(id) || new THREE.Vector3() : new THREE.Vector3());
    moved = [...offsets.values()].filter(v => v.lengthSq() > 1e-12).length;
    selectedMoved = (offsets.get(selected)?.lengthSq() || 0) > 1e-12;
    dispose(guides); guides.clear();
    if (!explode) return;
    for (const j of robot.joints) {
      const origin = origins.get(j.child); if (!origin || !parts.has(j.parent)) continue;
      const a = origin.clone().add(offsets.get(j.parent) || new THREE.Vector3());
      const b = origin.clone().add(offsets.get(j.child) || new THREE.Vector3());
      const active = j.child === selected || j.parent === selected;
      const color = active ? '#e6c078' : '#667788';
      const line = new THREE.Line(new THREE.BufferGeometry().setFromPoints([a, b]), new THREE.LineDashedMaterial({ color, dashSize: extent * .025, gapSize: extent * .015, depthTest: false, transparent: true, opacity: active ? .95 : .45 }));
      line.computeLineDistances(); line.renderOrder = 2; guides.add(line);
      if (active) for (const point of [a, b]) {
        const marker = new THREE.Mesh(new THREE.SphereGeometry(extent * .009, 12, 8), new THREE.MeshBasicMaterial({ color, depthTest: false }));
        marker.position.copy(point); marker.renderOrder = 3; guides.add(marker);
      }
    }
  }
  function restore(all = false) {
    if (all) offsets.clear(); else for (const id of affected()) offsets.delete(id);
    updatePresentation();
  }
  const groundOffset = $derived(robot.placement?.[2] || 0);
  let worldLabel: HTMLSpanElement;
  function placeOnGround() {
    assembly.updateMatrixWorld(true);
    const bounds = new THREE.Box3();
    assembly.traverse(object => { if (object instanceof THREE.Mesh) bounds.union(new THREE.Box3().setFromObject(object)); });
    if (bounds.isEmpty()) return;
    const position: [number, number, number] = [...(robot.placement || [0, 0, 0])] as [number, number, number];
    position[2] += floorHeight - bounds.min.z;
    onplacement?.(position);
  }
  function restoreOrigin() { onplacement?.([0, 0, 0]); }
  let fittedRoot = '';
  let fitted = false;
  function dispose(group: THREE.Object3D) {
    group.traverse(object => { if (object instanceof THREE.Mesh || object instanceof THREE.Line) { object.geometry.dispose(); const materials = Array.isArray(object.material) ? object.material : [object.material]; materials.forEach(m => m.dispose()); } });
  }
  function fit() {
    if (!assembly || !camera) return;
    const box = new THREE.Box3().setFromObject(assembly);
    const center = box.isEmpty() ? new THREE.Vector3() : box.getCenter(new THREE.Vector3());
    const radius = box.isEmpty() ? 1 : Math.max(box.getSize(new THREE.Vector3()).length(), 0.3);
    controls.target.copy(center); camera.position.copy(center).add(new THREE.Vector3(1, -1.5, 1).normalize().multiplyScalar(radius * 1.6));
    camera.near = Math.max(0.0001, radius / 10000); camera.far = Math.max(100, radius * 100); camera.updateProjectionMatrix(); controls.update();
  }
  function geometry(shape: Shape): THREE.Object3D {
    let object: THREE.Object3D;
    if (shape.kind === 'mesh') {
      const asset = robot.assets[shape.asset]; if (!asset) throw new Error(`Missing mesh: ${shape.asset}`);
      const bytes = Uint8Array.from(atob(asset.data), c => c.charCodeAt(0));
      object = asset.format === 'stl' ? new THREE.Mesh(new STLLoader().parse(bytes.buffer)) : new OBJLoader().parse(new TextDecoder().decode(bytes));
      object.scale.set(...shape.size);
    } else {
      const g = shape.kind === 'box' ? new THREE.BoxGeometry(...shape.size) : shape.kind === 'sphere' ? new THREE.SphereGeometry(shape.size[0], 24, 16) : new THREE.CylinderGeometry(shape.size[0], shape.size[0], shape.size[1], 32);
      if (shape.kind === 'cylinder') g.rotateX(Math.PI / 2);
      object = new THREE.Mesh(g);
    }
    object.traverse(o => { if (o instanceof THREE.Mesh) {
      const old = Array.isArray(o.material) ? o.material : [o.material]; old.forEach(m => m.dispose());
      o.material = new THREE.MeshStandardMaterial({ color: shape.color, metalness: 0.15, roughness: 0.7, wireframe: collision, side: THREE.DoubleSide });
    } });
    const frame = new THREE.Group(); frame.applyMatrix4(matrix(shape.origin)); frame.add(object); return frame;
  }
  $effect(() => {
    // Serialize to track nested configuration changes from the property inspector.
    const key = JSON.stringify(robot); JSON.stringify(positions); selected; collision; explode;
    if (!ready) return;
    if (key !== robotKey || !explode) { offsets.clear(); robotKey = key; }
    transform.detach(); parts = new Map(); origins = new Map();
    error = ''; dispose(assembly); scene.remove(assembly); assembly = new THREE.Group(); scene.add(assembly);
    const seen = new Set<string>();
    function add(id: string, parent: THREE.Object3D) {
      if (seen.has(id)) return; seen.add(id);
      const link = robot.links.find(l => l.id === id); if (!link) return;
      const frameGroup = new THREE.Group(); parent.add(frameGroup);
      const group = new THREE.Group(); frameGroup.add(group); parts.set(id, group);
      for (const shape of collision ? link.collisions : link.visuals) {
        try { const object = geometry(shape); object.traverse(o => { o.userData.linkId = id; if (o instanceof THREE.Mesh && id === selected) (o.material as THREE.MeshStandardMaterial).emissive.set('#163957'); }); group.add(object); }
        catch (cause) { error = String(cause); }
      }
      if (id === selected) {
        const axes = new THREE.AxesHelper(0.15);
        // Selection axes are an overlay: coplanar grid/mesh surfaces must not
        // compete with them in the depth buffer as the camera moves.
        const materials = Array.isArray(axes.material) ? axes.material : [axes.material];
        for (const material of materials) {
          material.depthTest = false;
          material.depthWrite = false;
          material.toneMapped = false;
        }
        axes.renderOrder = 1;
        group.add(axes);
      }
      for (const j of robot.joints.filter(j => j.parent === id)) {
        const frame = new THREE.Group(); frame.applyMatrix4(matrix(j.origin)); frameGroup.add(frame);
        const motion = new THREE.Group(), axis = new THREE.Vector3(...j.axis).normalize(), value = positions[j.id] || 0;
        if (j.type === 'prismatic') motion.position.copy(axis.multiplyScalar(value));
        if (j.type === 'revolute' || j.type === 'continuous') motion.quaternion.setFromAxisAngle(axis, value);
        frame.add(motion); add(j.child, motion);
      }
    }
    for (const link of roots(robot)) add(link.id, assembly);
    assembly.updateMatrixWorld(true);
    for (const [id, part] of parts) { assembly.attach(part); origins.set(id, part.position.clone()); }
    extent = Math.max(new THREE.Box3().setFromObject(assembly).getSize(new THREE.Vector3()).length(), .1);
    untrack(() => {
      assembly.position.fromArray(poses ? [0, 0, 0] : robot.placement || [0, 0, 0]); guides.position.copy(assembly.position);
      updatePresentation();
      if (poses) for (const [id, pose] of Object.entries(poses)) {
        const part = parts.get(id); if (part) { part.position.set(...pose.position); part.quaternion.set(...pose.quaternion); }
      }
    });
    if (explode && parts.has(selected)) transform.attach(parts.get(selected)!);
    const rootKey = roots(robot).map(l => l.id).join('/');
    if (!robot.links.length) fitted = false;
    if (robot.links.length && (!fitted || rootKey !== fittedRoot)) { fit(); fitted = true; fittedRoot = rootKey; }
  });
  $effect(() => {
    const current = poses;
    if (!ready || !current) return;
    assembly.position.set(0, 0, 0);
    for (const [id, pose] of Object.entries(current)) {
      const part = parts.get(id); if (part) { part.position.set(...pose.position); part.quaternion.set(...pose.quaternion); }
    }
  });
  onMount(() => {
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true }); renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
      renderer.setClearColor('#181818'); host.prepend(renderer.domElement);
      renderer.domElement.setAttribute('aria-label', 'Robot 3D viewport');
      scene = new THREE.Scene(); camera = new THREE.PerspectiveCamera(45, 1, 0.001, 1000); camera.up.set(0, 0, 1); camera.position.set(2, -3, 2);
      controls = new OrbitControls(camera, renderer.domElement); controls.enableDamping = true;
      scene.add(new THREE.HemisphereLight('#ffffff', '#65717f', 2));
      const light = new THREE.DirectionalLight('#ffffff', 3); light.position.set(3, -4, 6); scene.add(light);
      grid = new THREE.GridHelper(10, 50, '#414141', '#292929'); grid.rotation.x = Math.PI / 2; grid.position.z = floorHeight; scene.add(grid);
      const worldOrigin = new THREE.AxesHelper(.08);
      worldOrigin.renderOrder = 4;
      for (const material of Array.isArray(worldOrigin.material) ? worldOrigin.material : [worldOrigin.material]) { material.depthTest = false; material.depthWrite = false; material.toneMapped = false; }
      scene.add(worldOrigin);
      assembly = new THREE.Group(); scene.add(assembly);
      guides = new THREE.Group(); scene.add(guides);
      transform = new TransformControls(camera, renderer.domElement); transform.setMode('translate'); transform.setSpace('world'); transform.setSize(.85); scene.add(transform.getHelper());
      transform.addEventListener('dragging-changed', event => {
        dragging = Boolean(event.value); controls.enabled = !dragging;
        if (dragging) { suppressPick = true; dragStart.copy(parts.get(selected)!.position); dragOffsets = new Map([...offsets].map(([id, value]) => [id, value.clone()])); }
        else {
          const part = parts.get(selected), origin = origins.get(selected);
          const parentOffset = joint ? offsets.get(joint.parent) || new THREE.Vector3() : new THREE.Vector3();
          if (part && origin && part.position.distanceTo(origin.clone().add(parentOffset)) < extent * .035) {
            const adjustment = origin.clone().add(parentOffset).sub(part.position);
            for (const id of affected()) offsets.set(id, (offsets.get(id) || new THREE.Vector3()).clone().add(adjustment));
          }
          updatePresentation();
        }
      });
      transform.addEventListener('objectChange', () => {
        if (!dragging) return;
        const delta = parts.get(selected)!.position.clone().sub(dragStart);
        for (const id of affected()) offsets.set(id, (dragOffsets.get(id) || new THREE.Vector3()).clone().add(delta));
        updatePresentation();
      });
      const observer = new ResizeObserver(() => { const { width, height } = host.getBoundingClientRect(); if (width && height) { renderer.setSize(width, height); camera.aspect = width / height; camera.updateProjectionMatrix(); } }); observer.observe(host);
      const raycaster = new THREE.Raycaster(); let down = [0, 0];
      const pointerDown = (event: PointerEvent) => { down = [event.clientX, event.clientY]; if (!dragging) suppressPick = false; };
      const pointerUp = (event: PointerEvent) => {
        if (suppressPick || transform.axis !== null || event.button !== 0 || Math.hypot(event.clientX - down[0], event.clientY - down[1]) > 4) return;
        const rect = renderer.domElement.getBoundingClientRect(); raycaster.setFromCamera(new THREE.Vector2((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1), camera);
        const hit = raycaster.intersectObject(assembly, true).find(h => h.object.userData.linkId); if (hit) onselect(hit.object.userData.linkId);
      };
      renderer.domElement.addEventListener('pointerdown', pointerDown); renderer.domElement.addEventListener('pointerup', pointerUp);
      renderer.setAnimationLoop(() => { if (host.clientWidth && host.clientHeight) { controls.update(); renderer.render(scene, camera);
        const origin = new THREE.Vector3().project(camera);
        worldLabel.style.display = origin.z >= -1 && origin.z <= 1 && Math.abs(origin.x) <= 1 && Math.abs(origin.y) <= 1 ? '' : 'none';
        worldLabel.style.left = `${(origin.x + 1) * host.clientWidth / 2}px`;
        worldLabel.style.top = `${(1 - origin.y) * host.clientHeight / 2}px`; } }); ready = true;
      return () => { observer.disconnect(); renderer.setAnimationLoop(null); controls.dispose(); transform.detach(); scene.remove(transform.getHelper()); transform.dispose(); dispose(scene); renderer.dispose(); renderer.domElement.remove(); };
    } catch (cause) { error = `3D viewport unavailable: ${String(cause)}`; }
  });
</script>

<div bind:this={host} class="relative min-h-0 flex-1 overflow-hidden">
  <div class="absolute left-3 top-3 z-10 flex max-w-[calc(100%-24px)] flex-wrap items-center gap-2">
    <Button size="icon-sm" onclick={fit} disabled={!ready} aria-label="Zoom to fit" title="Zoom to fit"><Maximize size={15} /></Button>
    {#if allowGroundPlacement}
      <Button size="sm" onclick={placeOnGround} disabled={!ready || !robot.links.length || explode} title="Set the whole robot’s placement on the ground; save a version to use it in simulation">Place on ground</Button>
      {#if Math.abs(groundOffset) > 1e-8}<Button size="sm" onclick={restoreOrigin}>Restore origin</Button><span class="rounded bg-surface px-2 py-1 text-[10px] text-muted">Ground placement · Save to simulate</span>{/if}
    {/if}
    {#if explode}
      <Button size="sm" onclick={() => restore()} disabled={!ready || !selectedMoved}>Restore part</Button>
      <Button size="sm" onclick={() => restore(true)} disabled={!ready || !moved}>Reset all</Button>
      <label class="flex items-center gap-1.5 rounded bg-surface px-2 py-1 text-xs"><input type="checkbox" bind:checked={withChildren} /> Move with children</label>
      <div class="basis-full rounded border border-border bg-surface/95 px-3 py-2 text-xs" aria-label="Explode connection">
        <p>{robot.links.find(l => l.id === selected)?.name || 'Select a part'}{#if joint}{' → '}{robot.links.find(l => l.id === joint.parent)?.name} · {joint.name} · {joint.type}{:else if selected} · Root link{/if}</p>
        <p class="mt-1 text-muted">Drag an axis handle to separate. Matching dots show the joint connection. Move near the parent marker to snap back.</p>
        <p class="mt-1 text-muted" role="status">{moved} displaced part{moved === 1 ? '' : 's'} · View only</p>
      </div>
    {/if}
  </div>
  <span bind:this={worldLabel} class="pointer-events-none absolute ml-2 mt-2 rounded bg-surface/90 px-1.5 py-0.5 text-[10px] text-muted">World (0, 0, 0)</span>
  {#if !robot.links.length}<p class="pointer-events-none absolute inset-0 flex items-center justify-center px-8 text-center text-sm text-muted">Add a module or import a robot to begin.</p>{/if}
  {#if error}<p class="absolute bottom-10 left-3 right-3 rounded bg-surface p-3 text-xs text-error" role="alert">{error}</p>{/if}
  <p class="pointer-events-none absolute bottom-3 left-3 text-xs text-[#666666]">{explode ? 'Select a part · Drag handles to move · Drag background to orbit' : 'Drag to orbit'} · Scroll to zoom · Right-drag to pan · Metres</p>
</div>
