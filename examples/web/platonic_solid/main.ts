import * as THREE from 'three';
import { WebGPURenderer } from 'three/webgpu';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import init, { create_platonic_solid, WasmTriangularGeometry } from '@solid-euclid';

type ViewMode = 'solid' | 'wireframe';
type SolidType = 'tetrahedron' | 'hexahedron' | 'octahedron' | 'dodecahedron' | 'icosahedron';

/**
 * Creates a Three.js BufferGeometry from a WasmTriangularGeometry
 */
function createBufferGeometryFromWasm(wasmGeometry: WasmTriangularGeometry): THREE.BufferGeometry {
  const geometry = new THREE.BufferGeometry();
  
  // Copy typed arrays from WASM (view is temporary, need to copy)
  const vertices = new Float32Array(wasmGeometry.vertices);
  const normals = new Float32Array(wasmGeometry.normals);
  const uvs = new Float32Array(wasmGeometry.uvs);
  const indices = new Uint32Array(wasmGeometry.indices);

  geometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
  geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
  geometry.setAttribute('uv', new THREE.BufferAttribute(uvs, 2));
  geometry.setIndex(new THREE.BufferAttribute(indices, 1));

  return geometry;
}

/**
 * Creates wireframe edges from a BufferGeometry showing all triangle edges
 */
function createWireframeEdges(geometry: THREE.BufferGeometry): THREE.LineSegments {
  const edgesGeometry = new THREE.EdgesGeometry(geometry, 0); // threshold 0 to show all edges
  const edgesMaterial = new THREE.LineBasicMaterial({ color: 0xffffff });
  return new THREE.LineSegments(edgesGeometry, edgesMaterial);
}

/**
 * Creates vertex normal visualization lines from a BufferGeometry
 */
function createVertexNormals(geometry: THREE.BufferGeometry, length: number = 0.1): THREE.LineSegments {
  const positionAttr = geometry.getAttribute('position');
  const normalAttr = geometry.getAttribute('normal');
  
  if (!positionAttr || !normalAttr) {
    throw new Error('Geometry must have position and normal attributes');
  }

  const positions: number[] = [];
  
  // For indexed geometry, we need to iterate through unique vertices
  const processedVertices = new Set<string>();
  
  const addNormalLine = (index: number) => {
    const x = positionAttr.getX(index);
    const y = positionAttr.getY(index);
    const z = positionAttr.getZ(index);
    const nx = normalAttr.getX(index);
    const ny = normalAttr.getY(index);
    const nz = normalAttr.getZ(index);
    
    // Create unique key for this vertex+normal combination
    const key = `${x.toFixed(6)},${y.toFixed(6)},${z.toFixed(6)},${nx.toFixed(6)},${ny.toFixed(6)},${nz.toFixed(6)}`;
    
    if (!processedVertices.has(key)) {
      processedVertices.add(key);
      
      // Start point (vertex position)
      positions.push(x, y, z);
      // End point (vertex + normal * length)
      positions.push(x + nx * length, y + ny * length, z + nz * length);
    }
  };

  const indexAttr = geometry.getIndex();
  if (indexAttr) {
    for (let i = 0; i < indexAttr.count; i++) {
      addNormalLine(indexAttr.getX(i));
    }
  } else {
    for (let i = 0; i < positionAttr.count; i++) {
      addNormalLine(i);
    }
  }

  const normalGeometry = new THREE.BufferGeometry();
  normalGeometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  
  const normalMaterial = new THREE.LineBasicMaterial({ color: 0xff0000 });
  return new THREE.LineSegments(normalGeometry, normalMaterial);
}

async function main() {
  // Initialize WASM module
  await init();

  // Check WebGPU support
  if (!navigator.gpu) {
    const app = document.getElementById('app');
    if (app) {
      app.innerHTML = '<p style="color: white; padding: 20px;">WebGPU is not supported in this browser. Please use Chrome 113+ or Edge 113+.</p>';
    }
    return;
  }

  // Create scene
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x1a1a2e);

  // Create perspective camera
  const camera = new THREE.PerspectiveCamera(
    75,
    window.innerWidth / window.innerHeight,
    0.1,
    1000
  );
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
  const axesHelper = new THREE.AxesHelper(2);
  scene.add(axesHelper);

  // Load texture
  const textureLoader = new THREE.TextureLoader();
  const texture = await new Promise<THREE.Texture>((resolve, reject) => {
    textureLoader.load(
      '../assets/512x512 Texel Density Texture 1.png',
      (tex: THREE.Texture) => {
        tex.colorSpace = THREE.SRGBColorSpace;
        resolve(tex);
      },
      undefined,
      reject
    );
  });

  // Create MeshStandardMaterial with 50% diffuse (roughness controls this)
  const solidMaterial = new THREE.MeshStandardMaterial({
    map: texture,
    roughness: 0.5,
    metalness: 0.0,
  });

  // State
  let currentSolidType: SolidType = 'hexahedron';
  let currentViewMode: ViewMode | null = null;
  let currentGeometry: THREE.BufferGeometry | null = null;
  let solidMesh: THREE.Mesh | null = null;
  let wireframeGroup: THREE.Group | null = null;

  /**
   * Creates the geometry and mesh objects for the current solid type
   */
  function createSolidObjects(solidType: SolidType): {
    geometry: THREE.BufferGeometry;
    mesh: THREE.Mesh;
    wireframe: THREE.Group;
  } {
    // Create geometry from WASM
    const wasmGeometry = create_platonic_solid(solidType, 'default');
    const geometry = createBufferGeometryFromWasm(wasmGeometry);

    // Create solid mesh
    const mesh = new THREE.Mesh(geometry, solidMaterial);

    // Create wireframe visualization group
    const wireframe = new THREE.Group();
    const wireframeEdges = createWireframeEdges(geometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(geometry, 0.15);
    wireframe.add(vertexNormals);

    return { geometry, mesh, wireframe };
  }

  /**
   * Updates the scene with a new solid type
   */
  function setSolidType(solidType: SolidType) {
    if (solidType === currentSolidType && currentGeometry) return;

    // Remove old objects from scene
    if (solidMesh) scene.remove(solidMesh);
    if (wireframeGroup) scene.remove(wireframeGroup);

    // Dispose old geometry
    if (currentGeometry) {
      currentGeometry.dispose();
    }

    // Create new objects
    const objects = createSolidObjects(solidType);
    currentGeometry = objects.geometry;
    solidMesh = objects.mesh;
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
      if (solidMesh) scene.add(solidMesh);
    } else {
      if (solidMesh) scene.remove(solidMesh);
      if (wireframeGroup) scene.add(wireframeGroup);
    }
  }

  // Initialize with default solid (hexahedron/cube)
  setSolidType('hexahedron');
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
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
  scene.add(ambientLight);

  // Add DirectionalLight (50% intensity) - RectAreaLight not supported in WebGPU
  const directionalLight = new THREE.DirectionalLight(0xffffff, 0.5);
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
