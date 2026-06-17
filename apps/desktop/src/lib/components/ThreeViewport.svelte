<script lang="ts">
  import { onMount } from "svelte";
  import { projectState } from "$lib/project.svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import * as THREE from "three";
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
  import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
  import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
  import URDFLoader from "urdf-loader";

  let containerElement: HTMLDivElement | null = null;
  
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let renderer: THREE.WebGLRenderer;
  let controls: OrbitControls;
  let currentModel: THREE.Object3D | null = null;
  let animationFrameId: number;

  let filePath = $derived(projectState.selectedFile);

  // Re-run loading whenever the selected file changes
  $effect(() => {
    if (filePath) {
      load3DFile(filePath);
    }
  });

  async function load3DFile(path: string) {
    if (!scene) return;
    
    // Clear previous model
    if (currentModel) {
      scene.remove(currentModel);
      currentModel = null;
    }

    const ext = path.split('.').pop()?.toLowerCase();
    const resolvedUrl = convertFileSrc(path);

    try {
      if (ext === 'urdf') {
        let robot: THREE.Object3D | null = null;
        
        const manager = new THREE.LoadingManager();
        manager.onLoad = () => {
          if (robot) fitCameraToModel(robot);
        };
        manager.onProgress = () => {
          if (robot) fitCameraToModel(robot);
        };

        const loader = new URDFLoader(manager);
        loader.fetchOptions = { mode: 'cors' };
        
        const parentDir = path.substring(0, path.lastIndexOf('/'));
        loader.workingPath = convertFileSrc(parentDir) + "/";
        
        // Dynamically build package mapping from all parent directories in the filepath
        const packagesMap: Record<string, string> = {};
        const parts = path.split(/[/\\]/);
        let currentPath = "";
        for (let i = 0; i < parts.length - 1; i++) {
          if (!parts[i]) continue;
          currentPath += "/" + parts[i];
          packagesMap[parts[i]] = convertFileSrc(currentPath);
        }
        packagesMap[""] = convertFileSrc(parentDir);
        loader.packages = packagesMap;

        // Read the URDF file contents using Tauri Rust command to avoid CORS/fetch restrictions
        const xmlText = await invoke<string>("read_file", { path });
        robot = loader.parse(xmlText);
        
        currentModel = robot;
        // ROS is Z-up, Three.js is Y-up by default, convert orientation
        robot.rotation.x = -Math.PI / 2;
        scene.add(robot);
        fitCameraToModel(robot);
      } else if (ext === 'stl') {
        const loader = new STLLoader();
        loader.load(
          resolvedUrl,
          (geometry) => {
            const material = new THREE.MeshStandardMaterial({ 
              color: 0x4c9df3, // Accent color
              roughness: 0.4,
              metalness: 0.2
            });
            const mesh = new THREE.Mesh(geometry, material);
            currentModel = mesh;
            scene.add(mesh);
            fitCameraToModel(mesh);
          },
          undefined,
          (err) => console.error("Error loading STL:", err)
        );
      } else if (ext === 'gltf' || ext === 'glb') {
        const loader = new GLTFLoader();
        loader.load(
          resolvedUrl,
          (gltf) => {
            currentModel = gltf.scene;
            scene.add(gltf.scene);
            fitCameraToModel(gltf.scene);
          },
          undefined,
          (err) => console.error("Error loading GLTF/GLB:", err)
        );
      }
    } catch (e) {
      console.error("Failed to load 3D file:", e);
    }
  }

  function fitCameraToModel(object: THREE.Object3D) {
    const box = new THREE.Box3().setFromObject(object);
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());

    const maxDim = Math.max(size.x, size.y, size.z);
    
    if (maxDim <= 0.0001 || isNaN(maxDim)) {
      // Set default position if meshes are still loading or size is zero
      camera.position.set(3, 3, 5);
      camera.lookAt(0, 0, 0);
      if (controls) {
        controls.target.set(0, 0, 0);
        controls.update();
      }
      return;
    }

    const fov = camera.fov * (Math.PI / 180);
    let cameraZ = Math.abs(maxDim / 2 / Math.tan(fov / 2));
    
    // Zoom out for spacing
    cameraZ *= 1.8;

    camera.position.set(center.x + cameraZ * 0.5, center.y + cameraZ * 0.5, center.z + cameraZ);
    camera.lookAt(center);
    
    if (controls) {
      controls.target.copy(center);
      controls.update();
    }
  }

  onMount(() => {
    if (!containerElement) return;

    // 1. Scene setup
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x141414); // Matches VS Code dark bg
    
    // 2. Camera setup
    camera = new THREE.PerspectiveCamera(
      45,
      containerElement.clientWidth / containerElement.clientHeight,
      0.01,
      1000
    );
    camera.position.set(3, 3, 5);

    // 3. Renderer setup
    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(containerElement.clientWidth, containerElement.clientHeight);
    renderer.shadowMap.enabled = true;
    renderer.domElement.style.pointerEvents = "auto";
    renderer.domElement.style.touchAction = "none";
    containerElement.appendChild(renderer.domElement);

    // 4. Controls setup
    controls = new OrbitControls(camera, containerElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;

    // Prevent default WebKit/OS drag-and-drop and selection behaviors that lock OrbitControls dragging
    const preventDefault = (e: Event) => e.preventDefault();
    containerElement.addEventListener("dragstart", preventDefault);
    containerElement.addEventListener("selectstart", preventDefault);

    // 5. Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambientLight);

    const dirLight1 = new THREE.DirectionalLight(0xffffff, 1.0);
    dirLight1.position.set(5, 15, 5);
    scene.add(dirLight1);

    const dirLight2 = new THREE.DirectionalLight(0x4c9df3, 0.4); // Subtle accent blue light
    dirLight2.position.set(-5, 5, -5);
    scene.add(dirLight2);

    // Grid helper
    const gridHelper = new THREE.GridHelper(20, 20, 0x4c9df3, 0x2e2e2e);
    gridHelper.position.y = -0.01;
    scene.add(gridHelper);

    // Resize listener
    const resizeObserver = new ResizeObserver(() => {
      if (!containerElement || !renderer || !camera) return;
      const width = containerElement.clientWidth;
      const height = containerElement.clientHeight;
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      renderer.setSize(width, height);
    });
    resizeObserver.observe(containerElement);

    // Start render loop
    function animate() {
      animationFrameId = requestAnimationFrame(animate);
      if (controls) controls.update();
      if (renderer && scene && camera) renderer.render(scene, camera);
    }
    animate();

    // Load initial file if present
    if (filePath) {
      load3DFile(filePath);
    }

    return () => {
      if (containerElement) {
        containerElement.removeEventListener("dragstart", preventDefault);
        containerElement.removeEventListener("selectstart", preventDefault);
      }
      resizeObserver.disconnect();
      cancelAnimationFrame(animationFrameId);
      if (renderer) renderer.dispose();
    };
  });
</script>

<div class="relative flex-1 h-full w-full bg-[#141414] overflow-hidden">
  <div bind:this={containerElement} class="w-full h-full pointer-events-auto" style="pointer-events: auto; touch-action: none;"></div>
  
  <!-- Premium legend overlay -->
  <div class="absolute bottom-4 left-4 bg-[#1e1e1e]/85 backdrop-blur-md text-[11px] text-muted border border-border-strong px-2.5 py-1.5 rounded flex items-center gap-2 pointer-events-none select-none z-10 shadow-lg">
    <div class="w-1.5 h-1.5 rounded-full bg-accent animate-pulse"></div>
    <span>3D Viewport</span>
    <span class="text-white/10">|</span>
    <span>Drag to orbit • Right-click to pan • Scroll to zoom</span>
  </div>
</div>
