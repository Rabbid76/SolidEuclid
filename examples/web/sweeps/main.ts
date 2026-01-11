import {
  AmbientLight,
  AxesHelper,
  BufferGeometry,
  Color,
  DirectionalLight,
  Group,
  Mesh,
  MeshStandardMaterial,
  PerspectiveCamera,
  RepeatWrapping,
  Scene,
  SRGBColorSpace,
  Texture,
  TextureLoader,
} from 'three';
import { WebGPURenderer } from 'three/webgpu';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import init, {
  create_torus,
  create_capsule,
  create_cylinder,
  WasmTriangularMesh,
} from '@solid-euclid';
import {
  createBufferGeometryFromWasm,
  createVertexNormals,
  createWireframeEdges,
  mergeBufferGeometries,
} from '../util/three-util';

type ViewMode = 'geometry' | 'solid' | 'wireframe';
type SweepType = 'Torus' | 'Capsule' | 'Cylinder';
type EdgeType = 'Edge' | 'SmoothEdge' | 'Chamfer' | 'Fillet';

// Material ID for uniform material (torus)
const MATERIAL_ID = 'default';

// Material IDs for capsule (3 regions)
const CAPSULE_MATERIALS = ['bottom', 'body', 'top'];

// Material IDs for cylinder (5 regions)
const CYLINDER_MATERIALS = ['bottom_cap', 'bottom_edge', 'body', 'top_edge', 'top_cap'];

interface TorusConfig {
  majorRadius: number;
  minorRadius: number;
  majorSegments: number;
  minorSegments: number;
}

interface CapsuleConfig {
  radius: number;
  height: number;
  segments: number;
}

interface EdgeConfig {
  type: EdgeType;
  chamferDistance: number;
  filletRadius: number;
}

interface CylinderConfig {
  radius: number;
  height: number;
  segments: number;
  bottomEdge: EdgeConfig;
  topEdge: EdgeConfig;
}

interface SweepConfig {
  type: SweepType;
  torus: TorusConfig;
  capsule: CapsuleConfig;
  cylinder: CylinderConfig;
}

async function main() {
  // Initialize WASM module
  await init();

  // Check WebGPU support
  if (!navigator.gpu) {
    const app = document.getElementById('app');
    if (app) {
      app.innerHTML =
        '<p style="color: white; padding: 20px;">WebGPU is not supported in this browser. Please use Chrome 113+ or Edge 113+.</p>';
    }
    return;
  }

  // Create scene
  const scene = new Scene();
  scene.background = new Color(0x1a1a2e);

  // Create perspective camera
  const camera = new PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
  camera.position.set(2, 2, 2);
  camera.lookAt(0, 0, 0);

  // Create WebGPU renderer
  const renderer = new WebGPURenderer({ antialias: true });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(window.devicePixelRatio);

  const app = document.getElementById('app');
  if (!app) {
    console.error('App container not found');
    return;
  }
  app.appendChild(renderer.domElement);

  // Initialize the renderer (required for WebGPU)
  await renderer.init();

  // Create OrbitControls
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.05;
  controls.screenSpacePanning = false;
  controls.minDistance = 1;
  controls.maxDistance = 15;

  // Add AxesHelper
  const axesHelper = new AxesHelper(2);
  scene.add(axesHelper);

  // Load textures for materials (like platonic_solid example)
  const textureLoader = new TextureLoader();
  const textureFiles = [
    '../assets/512x512 Texel Density Texture 1.png',
    '../assets/512x512 Texel Density Texture 2.png',
    '../assets/512x512 Texel Density Texture 3.png',
    '../assets/512x512 Texel Density Texture 4.png',
    '../assets/512x512 Texel Density Texture 5.png',
  ];

  const textures = await Promise.all(
    textureFiles.map(
      (file) =>
        new Promise<Texture>((resolve, reject) => {
          textureLoader.load(
            file,
            (tex: Texture) => {
              tex.colorSpace = SRGBColorSpace;
              tex.wrapS = RepeatWrapping;
              tex.wrapT = RepeatWrapping;
              resolve(tex);
            },
            undefined,
            reject
          );
        })
    )
  );

  // Create MeshStandardMaterial with 50% diffuse (roughness controls this)
  // polygonOffset is used to push the solid geometry slightly back to avoid z-fighting with wireframe
  const solidMaterial = new MeshStandardMaterial({
    map: textures[0],
    roughness: 0.5,
    metalness: 0.0,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
  });

  // Create materials for capsule using textures (3 different textures for bottom, body, top)
  const capsuleMaterials: Map<string, MeshStandardMaterial> = new Map();
  CAPSULE_MATERIALS.forEach((id, index) => {
    capsuleMaterials.set(
      id,
      new MeshStandardMaterial({
        map: textures[index],
        roughness: 0.5,
        metalness: 0.0,
        polygonOffset: true,
        polygonOffsetFactor: 1,
        polygonOffsetUnits: 1,
      })
    );
  });

  // Create materials for cylinder using textures (5 regions, cycle through 3 textures)
  const cylinderMaterials: Map<string, MeshStandardMaterial> = new Map();
  CYLINDER_MATERIALS.forEach((id, index) => {
    cylinderMaterials.set(
      id,
      new MeshStandardMaterial({
        map: textures[index % textures.length],
        roughness: 0.5,
        metalness: 0.0,
        polygonOffset: true,
        polygonOffsetFactor: 1,
        polygonOffsetUnits: 1,
      })
    );
  });

  // State
  let currentConfig: SweepConfig = {
    type: 'Torus',
    torus: {
      majorRadius: 1.0,
      minorRadius: 0.3,
      majorSegments: 36,
      minorSegments: 18,
    },
    capsule: {
      radius: 0.3,
      height: 1.0,
      segments: 24,
    },
    cylinder: {
      radius: 0.5,
      height: 1.5,
      segments: 24,
      bottomEdge: { type: 'Edge', chamferDistance: 0.1, filletRadius: 0.1 },
      topEdge: { type: 'Edge', chamferDistance: 0.1, filletRadius: 0.1 },
    },
  };
  let currentViewMode: ViewMode | null = null;
  let currentMergedGeometry: BufferGeometry | null = null;
  let solidGroup: Group | null = null;
  let wireframeGroup: Group | null = null;

  /**
   * Creates the geometry and mesh objects for a torus
   */
  function createTorusObjects(config: TorusConfig): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    const wasmMesh: WasmTriangularMesh = create_torus({
      major_radius: config.majorRadius,
      minor_radius: config.minorRadius,
      major_segments: config.majorSegments,
      minor_segments: config.minorSegments,
      material: { Uniform: MATERIAL_ID },
    });

    // Create a group to hold meshes
    const meshGroup = new Group();
    const geometries: BufferGeometry[] = [];

    // Create a mesh for each material in the mesh
    for (let i = 0; i < wasmMesh.geometry_count; i++) {
      const wasmGeometry = wasmMesh.get_geometry(i);
      if (wasmGeometry) {
        const geometry = createBufferGeometryFromWasm(wasmGeometry);
        geometries.push(geometry);

        const mesh = new Mesh(geometry, solidMaterial);
        meshGroup.add(mesh);
      }
    }

    // Create merged geometry for wireframe visualization
    const mergedGeometry = mergeBufferGeometries(geometries);

    // Create wireframe visualization group
    const wireframe = new Group();
    // Rotate the wireframe 90° around the x-axis to match the solid
    const wireframeEdges = createWireframeEdges(mergedGeometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(mergedGeometry, 0.1);
    wireframe.add(vertexNormals);

    return { mergedGeometry, meshGroup, wireframe };
  }

  /**
   * Creates the geometry and mesh objects for a capsule
   */
  function createCapsuleObjects(config: CapsuleConfig): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    // Create capsule with Array material to get 3 separate geometry groups
    const wasmMesh: WasmTriangularMesh = create_capsule({
      radius: config.radius,
      height: config.height,
      segments: config.segments,
      material: {
        Array: CAPSULE_MATERIALS,
      },
    });

    // Create a group to hold meshes
    const meshGroup = new Group();
    const geometries: BufferGeometry[] = [];

    // Create a mesh for each material in the mesh (bottom, body, top)
    const materialIds = wasmMesh.material_ids as string[];
    for (let i = 0; i < wasmMesh.geometry_count; i++) {
      const wasmGeometry = wasmMesh.get_geometry(i);
      if (wasmGeometry) {
        const geometry = createBufferGeometryFromWasm(wasmGeometry);
        geometries.push(geometry);

        // Select material based on material ID
        const materialId = materialIds[i] ?? '';
        const material = capsuleMaterials.get(materialId) ?? solidMaterial;

        const mesh = new Mesh(geometry, material);
        meshGroup.add(mesh);
      }
    }

    // Create merged geometry for wireframe visualization
    const mergedGeometry = mergeBufferGeometries(geometries);

    // Create wireframe visualization group
    const wireframe = new Group();
    // Rotate the wireframe 90° around the x-axis to match the solid
    const wireframeEdges = createWireframeEdges(mergedGeometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(mergedGeometry, 0.1);
    wireframe.add(vertexNormals);

    return { mergedGeometry, meshGroup, wireframe };
  }

  /**
   * Converts an EdgeConfig to the WASM API format
   */
  function edgeConfigToWasm(
    edge: EdgeConfig
  ): string | { Chamfer: { distance: number } } | { Fillet: { radius: number } } {
    switch (edge.type) {
      case 'Edge':
        return 'Edge';
      case 'SmoothEdge':
        return 'SmoothEdge';
      case 'Chamfer':
        return { Chamfer: { distance: edge.chamferDistance } };
      case 'Fillet':
        return { Fillet: { radius: edge.filletRadius } };
    }
  }

  /**
   * Creates the geometry and mesh objects for a cylinder
   */
  function createCylinderObjects(config: CylinderConfig): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    // Create cylinder with Array material to get 5 separate geometry groups
    const wasmMesh: WasmTriangularMesh = create_cylinder({
      radius: config.radius,
      height: config.height,
      segments: config.segments,
      bottom_edge: edgeConfigToWasm(config.bottomEdge),
      top_edge: edgeConfigToWasm(config.topEdge),
      material: {
        Array: CYLINDER_MATERIALS,
      },
    });

    // Create a group to hold meshes
    const meshGroup = new Group();
    const geometries: BufferGeometry[] = [];

    // Create a mesh for each material in the mesh (bottom_cap, bottom_edge, body, top_edge, top_cap)
    const materialIds = wasmMesh.material_ids as string[];
    for (let i = 0; i < wasmMesh.geometry_count; i++) {
      const wasmGeometry = wasmMesh.get_geometry(i);
      if (wasmGeometry) {
        const geometry = createBufferGeometryFromWasm(wasmGeometry);
        geometries.push(geometry);

        // Select material based on material ID
        const materialId = materialIds[i] ?? '';
        const material = cylinderMaterials.get(materialId) ?? solidMaterial;

        const mesh = new Mesh(geometry, material);
        meshGroup.add(mesh);
      }
    }

    // Create merged geometry for wireframe visualization
    const mergedGeometry = mergeBufferGeometries(geometries);

    // Create wireframe visualization group
    const wireframe = new Group();
    const wireframeEdges = createWireframeEdges(mergedGeometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(mergedGeometry, 0.1);
    wireframe.add(vertexNormals);

    return { mergedGeometry, meshGroup, wireframe };
  }

  /**
   * Creates the geometry and mesh objects for the current sweep configuration
   */
  function createSweepObjects(config: SweepConfig): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    switch (config.type) {
      case 'Torus':
        return createTorusObjects(config.torus);
      case 'Capsule':
        return createCapsuleObjects(config.capsule);
      case 'Cylinder':
        return createCylinderObjects(config.cylinder);
      default:
        throw new Error(`Unknown sweep type: ${config.type}`);
    }
  }

  /**
   * Checks if two edge configs are equal
   */
  function edgeConfigsEqual(a: EdgeConfig, b: EdgeConfig): boolean {
    if (a.type !== b.type) return false;
    if (a.type === 'Chamfer' && a.chamferDistance !== b.chamferDistance) return false;
    if (a.type === 'Fillet' && a.filletRadius !== b.filletRadius) return false;
    return true;
  }

  /**
   * Checks if two sweep configs are equal
   */
  function configsEqual(a: SweepConfig, b: SweepConfig): boolean {
    if (a.type !== b.type) return false;
    if (a.type === 'Torus' && b.type === 'Torus') {
      return (
        a.torus.majorRadius === b.torus.majorRadius &&
        a.torus.minorRadius === b.torus.minorRadius &&
        a.torus.majorSegments === b.torus.majorSegments &&
        a.torus.minorSegments === b.torus.minorSegments
      );
    }
    if (a.type === 'Capsule' && b.type === 'Capsule') {
      return (
        a.capsule.radius === b.capsule.radius &&
        a.capsule.height === b.capsule.height &&
        a.capsule.segments === b.capsule.segments
      );
    }
    if (a.type === 'Cylinder' && b.type === 'Cylinder') {
      return (
        a.cylinder.radius === b.cylinder.radius &&
        a.cylinder.height === b.cylinder.height &&
        a.cylinder.segments === b.cylinder.segments &&
        edgeConfigsEqual(a.cylinder.bottomEdge, b.cylinder.bottomEdge) &&
        edgeConfigsEqual(a.cylinder.topEdge, b.cylinder.topEdge)
      );
    }
    return true;
  }

  /**
   * Updates the scene with a new sweep configuration
   */
  function setSweepConfig(config: SweepConfig) {
    if (configsEqual(config, currentConfig) && currentMergedGeometry) return;

    // Remove old objects from scene
    if (solidGroup) scene.remove(solidGroup);
    if (wireframeGroup) scene.remove(wireframeGroup);

    // Dispose old geometry
    if (currentMergedGeometry) {
      currentMergedGeometry.dispose();
    }

    // Create new objects
    const objects = createSweepObjects(config);
    currentMergedGeometry = objects.mergedGeometry;
    solidGroup = objects.meshGroup;
    wireframeGroup = objects.wireframe;
    currentConfig = {
      type: config.type,
      torus: { ...config.torus },
      capsule: { ...config.capsule },
      cylinder: {
        ...config.cylinder,
        bottomEdge: { ...config.cylinder.bottomEdge },
        topEdge: { ...config.cylinder.topEdge },
      },
    };

    // Re-apply current view mode
    const prevMode = currentViewMode;
    currentViewMode = null; // Force update
    setViewMode(prevMode ?? 'geometry');
  }

  /**
   * Updates the view mode (geometry, solid, or wireframe)
   */
  function setViewMode(mode: ViewMode) {
    if (mode === currentViewMode) return;
    currentViewMode = mode;

    if (mode === 'geometry') {
      // Show both solid and wireframe (wireframe on top)
      if (solidGroup) scene.add(solidGroup);
      if (wireframeGroup) scene.add(wireframeGroup);
    } else if (mode === 'solid') {
      if (wireframeGroup) scene.remove(wireframeGroup);
      if (solidGroup) scene.add(solidGroup);
    } else {
      // wireframe mode
      if (solidGroup) scene.remove(solidGroup);
      if (wireframeGroup) scene.add(wireframeGroup);
    }
  }

  /**
   * Updates UI visibility based on sweep type
   */
  function updateUIVisibility(sweepType: SweepType) {
    const torusOptions = document.getElementById('torusOptions');
    const capsuleOptions = document.getElementById('capsuleOptions');
    const cylinderOptions = document.getElementById('cylinderOptions');
    if (torusOptions) {
      torusOptions.style.display = sweepType === 'Torus' ? 'flex' : 'none';
    }
    if (capsuleOptions) {
      capsuleOptions.style.display = sweepType === 'Capsule' ? 'flex' : 'none';
    }
    if (cylinderOptions) {
      cylinderOptions.style.display = sweepType === 'Cylinder' ? 'flex' : 'none';
    }
  }

  /**
   * Updates edge parameter UI visibility based on edge type
   */
  function updateEdgeUIVisibility(position: 'bottom' | 'top', edgeType: EdgeType) {
    const chamferLabel = document.getElementById(`${position}ChamferDistanceLabel`);
    const filletLabel = document.getElementById(`${position}FilletRadiusLabel`);
    if (chamferLabel) {
      chamferLabel.style.display = edgeType === 'Chamfer' ? 'flex' : 'none';
    }
    if (filletLabel) {
      filletLabel.style.display = edgeType === 'Fillet' ? 'flex' : 'none';
    }
  }

  // Initialize with default geometry view (solid + wireframe)
  setSweepConfig(currentConfig);
  setViewMode('geometry');
  updateUIVisibility('Torus');

  // Setup menu event listeners
  const sweepTypeSelect = document.getElementById('sweepType') as HTMLSelectElement;
  const majorRadiusInput = document.getElementById('majorRadius') as HTMLInputElement;
  const minorRadiusInput = document.getElementById('minorRadius') as HTMLInputElement;
  const majorSegmentsInput = document.getElementById('majorSegments') as HTMLInputElement;
  const minorSegmentsInput = document.getElementById('minorSegments') as HTMLInputElement;
  const capsuleRadiusInput = document.getElementById('capsuleRadius') as HTMLInputElement;
  const capsuleHeightInput = document.getElementById('capsuleHeight') as HTMLInputElement;
  const capsuleSegmentsInput = document.getElementById('capsuleSegments') as HTMLInputElement;
  const cylinderRadiusInput = document.getElementById('cylinderRadius') as HTMLInputElement;
  const cylinderHeightInput = document.getElementById('cylinderHeight') as HTMLInputElement;
  const cylinderSegmentsInput = document.getElementById('cylinderSegments') as HTMLInputElement;
  const cylinderBottomEdgeSelect = document.getElementById(
    'cylinderBottomEdge'
  ) as HTMLSelectElement;
  const cylinderTopEdgeSelect = document.getElementById('cylinderTopEdge') as HTMLSelectElement;
  const bottomChamferDistanceInput = document.getElementById(
    'bottomChamferDistance'
  ) as HTMLInputElement;
  const bottomFilletRadiusInput = document.getElementById('bottomFilletRadius') as HTMLInputElement;
  const topChamferDistanceInput = document.getElementById('topChamferDistance') as HTMLInputElement;
  const topFilletRadiusInput = document.getElementById('topFilletRadius') as HTMLInputElement;
  const bottomChamferDistanceValue = document.getElementById('bottomChamferDistanceValue');
  const bottomFilletRadiusValue = document.getElementById('bottomFilletRadiusValue');
  const topChamferDistanceValue = document.getElementById('topChamferDistanceValue');
  const topFilletRadiusValue = document.getElementById('topFilletRadiusValue');
  const viewModeSelect = document.getElementById('viewMode') as HTMLSelectElement;

  function rebuildSweep() {
    const type = sweepTypeSelect.value as SweepType;
    const majorRadius = parseFloat(majorRadiusInput.value) || 1.0;
    const minorRadius = parseFloat(minorRadiusInput.value) || 0.3;
    const majorSegments = parseInt(majorSegmentsInput.value, 10) || 36;
    const minorSegments = parseInt(minorSegmentsInput.value, 10) || 18;
    const capsuleRadius = parseFloat(capsuleRadiusInput?.value) || 0.3;
    const capsuleHeight = parseFloat(capsuleHeightInput?.value) || 1.0;
    const capsuleSegments = parseInt(capsuleSegmentsInput?.value, 10) || 24;
    const cylinderRadius = parseFloat(cylinderRadiusInput?.value) || 0.5;
    const cylinderHeight = parseFloat(cylinderHeightInput?.value) || 1.5;
    const cylinderSegments = parseInt(cylinderSegmentsInput?.value, 10) || 24;
    const bottomEdgeType = (cylinderBottomEdgeSelect?.value || 'Edge') as EdgeType;
    const topEdgeType = (cylinderTopEdgeSelect?.value || 'Edge') as EdgeType;
    const bottomChamferDistance = parseFloat(bottomChamferDistanceInput?.value) || 0.1;
    const bottomFilletRadius = parseFloat(bottomFilletRadiusInput?.value) || 0.1;
    const topChamferDistance = parseFloat(topChamferDistanceInput?.value) || 0.1;
    const topFilletRadius = parseFloat(topFilletRadiusInput?.value) || 0.1;

    setSweepConfig({
      type,
      torus: { majorRadius, minorRadius, majorSegments, minorSegments },
      capsule: { radius: capsuleRadius, height: capsuleHeight, segments: capsuleSegments },
      cylinder: {
        radius: cylinderRadius,
        height: cylinderHeight,
        segments: cylinderSegments,
        bottomEdge: {
          type: bottomEdgeType,
          chamferDistance: bottomChamferDistance,
          filletRadius: bottomFilletRadius,
        },
        topEdge: {
          type: topEdgeType,
          chamferDistance: topChamferDistance,
          filletRadius: topFilletRadius,
        },
      },
    });
  }

  if (sweepTypeSelect) {
    sweepTypeSelect.addEventListener('change', () => {
      updateUIVisibility(sweepTypeSelect.value as SweepType);
      rebuildSweep();
    });
  }

  if (majorRadiusInput) {
    majorRadiusInput.addEventListener('change', rebuildSweep);
  }

  if (minorRadiusInput) {
    minorRadiusInput.addEventListener('change', rebuildSweep);
  }

  if (majorSegmentsInput) {
    majorSegmentsInput.addEventListener('change', rebuildSweep);
  }

  if (minorSegmentsInput) {
    minorSegmentsInput.addEventListener('change', rebuildSweep);
  }

  if (capsuleRadiusInput) {
    capsuleRadiusInput.addEventListener('change', rebuildSweep);
  }

  if (capsuleHeightInput) {
    capsuleHeightInput.addEventListener('change', rebuildSweep);
  }

  if (capsuleSegmentsInput) {
    capsuleSegmentsInput.addEventListener('change', rebuildSweep);
  }

  // Cylinder event listeners
  if (cylinderRadiusInput) {
    cylinderRadiusInput.addEventListener('change', rebuildSweep);
  }

  if (cylinderHeightInput) {
    cylinderHeightInput.addEventListener('change', rebuildSweep);
  }

  if (cylinderSegmentsInput) {
    cylinderSegmentsInput.addEventListener('change', rebuildSweep);
  }

  if (cylinderBottomEdgeSelect) {
    cylinderBottomEdgeSelect.addEventListener('change', () => {
      updateEdgeUIVisibility('bottom', cylinderBottomEdgeSelect.value as EdgeType);
      rebuildSweep();
    });
  }

  if (cylinderTopEdgeSelect) {
    cylinderTopEdgeSelect.addEventListener('change', () => {
      updateEdgeUIVisibility('top', cylinderTopEdgeSelect.value as EdgeType);
      rebuildSweep();
    });
  }

  if (bottomChamferDistanceInput) {
    bottomChamferDistanceInput.addEventListener('input', () => {
      if (bottomChamferDistanceValue) {
        bottomChamferDistanceValue.textContent = bottomChamferDistanceInput.value;
      }
      rebuildSweep();
    });
  }

  if (bottomFilletRadiusInput) {
    bottomFilletRadiusInput.addEventListener('input', () => {
      if (bottomFilletRadiusValue) {
        bottomFilletRadiusValue.textContent = bottomFilletRadiusInput.value;
      }
      rebuildSweep();
    });
  }

  if (topChamferDistanceInput) {
    topChamferDistanceInput.addEventListener('input', () => {
      if (topChamferDistanceValue) {
        topChamferDistanceValue.textContent = topChamferDistanceInput.value;
      }
      rebuildSweep();
    });
  }

  if (topFilletRadiusInput) {
    topFilletRadiusInput.addEventListener('input', () => {
      if (topFilletRadiusValue) {
        topFilletRadiusValue.textContent = topFilletRadiusInput.value;
      }
      rebuildSweep();
    });
  }

  if (viewModeSelect) {
    viewModeSelect.addEventListener('change', (e) => {
      const target = e.target as HTMLSelectElement;
      setViewMode(target.value as ViewMode);
    });
  }

  // Add AmbientLight (50% intensity)
  const ambientLight = new AmbientLight(0xffffff, 0.5);
  scene.add(ambientLight);

  // Add DirectionalLight (50% intensity) - RectAreaLight not supported in WebGPU
  const directionalLight = new DirectionalLight(0xffffff, 0.5);
  directionalLight.position.set(2, 3, 2);
  scene.add(directionalLight);

  // Handle window resize
  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
  });

  // Animation loop using setAnimationLoop (recommended for WebGPU)
  renderer.setAnimationLoop(() => {
    controls.update();
    renderer.render(scene, camera);
  });
}

main().catch(console.error);
