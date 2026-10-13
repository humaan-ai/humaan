<script lang="ts">
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import BuilderVector from './BuilderVector.svelte';
  import { pose, clone, type Robot, type Link } from '$lib/builder/model';
  let { robot = $bindable(), link = $bindable(), onremove }: { robot: Robot; link: Link; onremove: () => void } = $props();
  const joint = $derived(robot.joints.find(j => j.child === link.id));
</script>
<div class="flex w-full min-w-0 flex-col gap-4 p-3 text-xs [overflow-wrap:anywhere] [&_label]:min-w-0 [&_details]:min-w-0 [&_button]:h-auto [&_button]:min-h-7 [&_button]:max-w-full [&_button]:whitespace-normal">
  <label>Link name<Input class="mt-1" bind:value={link.name} /></label>
  <details open><summary class="mb-3 cursor-pointer font-medium text-title">Physical properties</summary><div class="flex flex-col gap-3">
    <label>Mass (kg)<Input type="number" min="0" step="any" class="mt-1" bind:value={link.mass} /></label>
    <BuilderVector label="Center of mass (m)" bind:value={link.inertial.xyz} />
    <BuilderVector label="Inertial rotation (rad)" bind:value={link.inertial.rpy} />
    <BuilderVector label="Inertia (kg·m²)" axes={['Ixx', 'Ixy', 'Ixz', 'Iyy', 'Iyz', 'Izz']} bind:value={link.inertia} />
  </div></details>
  {#if joint}
    <details open><summary class="mb-3 cursor-pointer font-medium text-title">Joint · {joint.name}</summary><div class="flex flex-col gap-3">
      <p class="text-muted">Parent: {robot.links.find(l => l.id === joint.parent)?.name}</p>
      <label>Joint name<Input class="mt-1" bind:value={joint.name} /></label>
      <label>Type<NativeSelect class="mt-1 w-full" bind:value={joint.type}><option value="fixed">Fixed</option><option value="revolute">Revolute</option><option value="continuous">Continuous</option><option value="prismatic">Prismatic</option></NativeSelect></label>
      <BuilderVector label="Joint position (m)" bind:value={joint.origin.xyz} />
      <BuilderVector label="Joint rotation (rad)" bind:value={joint.origin.rpy} />
      {#if joint.type !== 'fixed'}
        <BuilderVector label="Joint axis" bind:value={joint.axis} />
        {#if joint.type !== 'continuous'}<div class="grid grid-cols-2 gap-2"><label>Lower ({joint.type === 'prismatic' ? 'm' : 'rad'})<Input type="number" step="any" bind:value={joint.lower} /></label><label>Upper<Input type="number" step="any" bind:value={joint.upper} /></label></div>{/if}
        <label>Effort limit ({joint.type === 'prismatic' ? 'N' : 'N·m'})<Input type="number" min="0" step="any" bind:value={joint.effort} /></label>
        <label>Velocity limit ({joint.type === 'prismatic' ? 'm/s' : 'rad/s'})<Input type="number" min="0" step="any" bind:value={joint.velocity} /></label>
      {/if}
      <label>Hardware mapping<Input class="mt-1" placeholder="e.g. can0/motor_1" bind:value={joint.hardware} /></label>
    </div></details>
  {/if}
  <details><summary class="mb-3 cursor-pointer font-medium text-title">Geometry</summary>
    {#each ['visuals', 'collisions'] as const as kind}
      <h3 class="my-3 text-muted">{kind === 'visuals' ? 'Visual' : 'Collision'}</h3>
      {#each link[kind] as shape, i}<div class="mb-3 flex flex-col gap-2 rounded border border-border p-2">
        <p>{shape.kind}{shape.kind === 'mesh' ? ` · ${shape.asset.split('/').at(-1)}` : ''}</p>
        <BuilderVector label={shape.kind === 'mesh' ? 'Scale' : shape.kind === 'cylinder' ? 'Radius / length / unused (m)' : shape.kind === 'sphere' ? 'Radius / unused / unused (m)' : 'Dimensions (m)'} bind:value={shape.size} />
        <BuilderVector label="Geometry position (m)" bind:value={shape.origin.xyz} />
        <BuilderVector label="Geometry rotation (rad)" bind:value={shape.origin.rpy} />
        <label>Color<Input type="color" bind:value={shape.color} /></label>
        <Button size="sm" onclick={() => link[kind].splice(i, 1)}>Remove geometry</Button>
      </div>{/each}
      <Button size="sm" onclick={() => link[kind].push({ kind: 'box', size: [0.1, 0.1, 0.1], asset: '', origin: pose(), color: '#8ca8c3' })}>Add box {kind === 'visuals' ? 'visual' : 'collision'}</Button>
    {/each}
    <p class="mt-3 text-muted">Geometry edits do not recalculate mass or inertia.</p>
    <Button class="mt-3" size="sm" onclick={() => link.collisions = clone(link.visuals)}>Copy visuals to collision</Button>
  </details>
  <details open><summary class="mb-3 cursor-pointer font-medium text-title">Mounting points</summary>
    {#each link.mounts as mount, i}<div class="mb-3 flex flex-col gap-2 rounded border border-border p-2">
      <label>Name<Input bind:value={mount.name} /></label>
      <BuilderVector label="Mount position (m)" bind:value={mount.origin.xyz} />
      <BuilderVector label="Mount rotation (rad)" bind:value={mount.origin.rpy} />
      <Button size="sm" onclick={() => link.mounts.splice(i, 1)}>Remove mount</Button>
    </div>{/each}
    <Button size="sm" onclick={() => { let n = link.mounts.length + 1; while (link.mounts.some(m => m.name === `mount_${n}`)) n++; link.mounts.push({ name: `mount_${n}`, origin: pose() }); }}>Add mounting point</Button>
  </details>
  <details><summary class="mb-3 cursor-pointer font-medium text-title">Sensors ({link.sensors.length})</summary>
    {#each link.sensors as sensor, i}<div class="mb-3 flex flex-col gap-2 rounded border border-border p-2">
      <label>Name<Input bind:value={sensor.name} /></label>
      <label>Type<NativeSelect class="w-full" bind:value={sensor.type}><option value="camera">Camera</option><option value="imu">IMU</option><option value="lidar">LiDAR</option><option value="force">Force / torque</option><option value="encoder">Encoder</option></NativeSelect></label>
      <label>Topic / interface<Input bind:value={sensor.topic} /></label>
      <label>Rate (Hz)<Input type="number" min="0.01" step="any" bind:value={sensor.rate} /></label>
      <BuilderVector label="Sensor position (m)" bind:value={sensor.origin.xyz} />
      <BuilderVector label="Sensor rotation (rad)" bind:value={sensor.origin.rpy} />
      <Button size="sm" onclick={() => link.sensors.splice(i, 1)}>Remove sensor</Button>
    </div>{/each}
    <Button size="sm" onclick={() => link.sensors.push({ name: `sensor_${link.sensors.length + 1}`, type: 'camera', topic: '', rate: 30, origin: pose() })}>Add sensor</Button>
  </details>
  <Button variant="destructive" onclick={onremove}>Remove link and descendants</Button>
</div>
