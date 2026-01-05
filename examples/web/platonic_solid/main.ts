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
  Scene,
  SRGBColorSpace,
  Texture,
  TextureLoader,
} from 'three';
import { WebGPURenderer } from 'three/webgpu';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import init, { create_platonic_solid, WasmTriangularMesh } from '@solid-euclid';
import {
  createBufferGeometryFromWasm,
  createVertexNormals,
  createWireframeEdges,
  getCyclicMaterials,
  mergeBufferGeometries,
} from '../util/three_util';

type ViewMode = 'solid' | 'wireframe';
type SolidType = 'Tetrahedron' | 'Hexahedron' | 'Octahedron' | 'Dodecahedron' | 'Icosahedron';

// Material IDs for the 5 textures (applied cyclically)
const MATERIAL_IDS = ['mat0', 'mat1', 'mat2', 'mat3', 'mat4'];

// Number of faces for each Platonic solid
const FACE_COUNTS: Record<SolidType, number> = {
  Tetrahedron: 4,
  Hexahedron: 6,
  Octahedron: 8,
  Dodecahedron: 12,
  Icosahedron: 20,
};

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
  controls.maxDistance = 10;

  // Add AxesHelper
  const axesHelper = new AxesHelper(2);
  scene.add(axesHelper);

  // Load 5 textures for cyclic material assignment
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
              resolve(tex);
            },
            undefined,
            reject
          );
        })
    )
  );

  // Create 5 MeshStandardMaterials with 50% diffuse (roughness controls this)
  const solidMaterials: Map<string, MeshStandardMaterial> = new Map();
  MATERIAL_IDS.forEach((id, index) => {
    solidMaterials.set(
      id,
      new MeshStandardMaterial({
        map: textures[index],
        roughness: 0.5,
        metalness: 0.0,
      })
    );
  });

  // State
  let currentSolidType: SolidType = 'Icosahedron';
  let currentViewMode: ViewMode | null = null;
  let currentMergedGeometry: BufferGeometry | null = null;
  let solidGroup: Group | null = null;
  let wireframeGroup: Group | null = null;

  /**
   * Creates the geometry and mesh objects for the current solid type using Material.Array
   */
  function createSolidObjects(solidType: SolidType): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    // Generate cyclic materials based on the number of faces
    const faceCount = FACE_COUNTS[solidType];
    const cyclicMaterials = getCyclicMaterials(faceCount, MATERIAL_IDS);

    const wasmMesh: WasmTriangularMesh = create_platonic_solid({
      geometry: solidType,
      material: { Array: cyclicMaterials },
    });

    // Create a group to hold meshes for each material
    const meshGroup = new Group();
    const geometries: BufferGeometry[] = [];

    // Create a mesh for each material in the mesh
    const materialIds = wasmMesh.material_ids as string[];
    for (let i = 0; i < wasmMesh.geometry_count; i++) {
      const wasmGeometry = wasmMesh.get_geometry(i);
      if (wasmGeometry) {
        const geometry = createBufferGeometryFromWasm(wasmGeometry);
        geometries.push(geometry);

        const materialId = materialIds[i];
        if (materialId) {
          const material = solidMaterials.get(materialId);
          if (material) {
            const mesh = new Mesh(geometry, material);
            meshGroup.add(mesh);
          }
        }
      }
    }

    // Create merged geometry for wireframe visualization
    const mergedGeometry = mergeBufferGeometries(geometries);

    // Create wireframe visualization group
    const wireframe = new Group();
    const wireframeEdges = createWireframeEdges(mergedGeometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(mergedGeometry, 0.15);
    wireframe.add(vertexNormals);

    return { mergedGeometry, meshGroup, wireframe };
  }

  /**
   * Updates the scene with a new solid type
   */
  function setSolidType(solidType: SolidType) {
    if (solidType === currentSolidType && currentMergedGeometry) return;

    // Remove old objects from scene
    if (solidGroup) scene.remove(solidGroup);
    if (wireframeGroup) scene.remove(wireframeGroup);

    // Dispose old geometry
    if (currentMergedGeometry) {
      currentMergedGeometry.dispose();
    }

    // Create new objects
    const objects = createSolidObjects(solidType);
    currentMergedGeometry = objects.mergedGeometry;
    solidGroup = objects.meshGroup;
    wireframeGroup = objects.wireframe;
    currentSolidType = solidType;

    // Re-apply current view mode
    const prevMode = currentViewMode;
    currentViewMode = null; // Force update
    setViewMode(prevMode ?? 'wireframe');
  }

  /**
   * Updates the view mode (solid or wireframe)
   */
  function setViewMode(mode: ViewMode) {
    if (mode === currentViewMode) return;
    currentViewMode = mode;

    if (mode === 'solid') {
      if (wireframeGroup) scene.remove(wireframeGroup);
      if (solidGroup) scene.add(solidGroup);
    } else {
      if (solidGroup) scene.remove(solidGroup);
      if (wireframeGroup) scene.add(wireframeGroup);
    }
  }

  // Initialize with default wireframe Icosahedron
  setSolidType('Icosahedron');
  setViewMode('wireframe');

  // Setup menu event listeners
  const solidTypeSelect = document.getElementById('solidType') as HTMLSelectElement;
  if (solidTypeSelect) {
    solidTypeSelect.addEventListener('change', (e) => {
      const target = e.target as HTMLSelectElement;
      setSolidType(target.value as SolidType);
    });
  }

  const viewModeSelect = document.getElementById('viewMode') as HTMLSelectElement;
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
