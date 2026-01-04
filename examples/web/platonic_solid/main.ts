import * as THREE from 'three';
import { WebGPURenderer } from 'three/webgpu';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import init, { create_platonic_solid, WasmTriangularGeometry } from '@solid-euclid';

type ViewMode = 'solid' | 'wireframe';
type SolidType = 'Tetrahedron' | 'Hexahedron' | 'Octahedron' | 'Dodecahedron' | 'Icosahedron';

// Material IDs for the 5 textures (applied cyclically)
const MATERIAL_IDS = ['mat0', 'mat1', 'mat2', 'mat3', 'mat4'];

// Number of faces for each Platonic solid
const FACE_COUNTS: Record<SolidType, number> = {
  'Tetrahedron': 4,
  'Hexahedron': 6,
  'Octahedron': 8,
  'Dodecahedron': 12,
  'Icosahedron': 20,
};

/**
 * Generates a cyclic material array for a given number of faces
 */
function getCyclicMaterials(faceCount: number): string[] {
  return Array.from({ length: faceCount }, (_, i) => MATERIAL_IDS[i % MATERIAL_IDS.length]!);
}

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
 * Merges multiple BufferGeometries into one
 */
function mergeBufferGeometries(geometries: THREE.BufferGeometry[]): THREE.BufferGeometry {
  if (geometries.length === 0) {
    return new THREE.BufferGeometry();
  }
  
  if (geometries.length === 1) {
    return geometries[0]!;
  }
  
  const merged = new THREE.BufferGeometry();
  
  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  let vertexOffset = 0;
  
  for (const geo of geometries) {
    const posAttr = geo.getAttribute('position') as THREE.BufferAttribute;
    const normAttr = geo.getAttribute('normal') as THREE.BufferAttribute;
    const uvAttr = geo.getAttribute('uv') as THREE.BufferAttribute;
    const indexAttr = geo.getIndex();
    
    // Copy positions
    for (let i = 0; i < posAttr.count; i++) {
      positions.push(posAttr.getX(i), posAttr.getY(i), posAttr.getZ(i));
    }
    
    // Copy normals
    for (let i = 0; i < normAttr.count; i++) {
      normals.push(normAttr.getX(i), normAttr.getY(i), normAttr.getZ(i));
    }
    
    // Copy UVs
    for (let i = 0; i < uvAttr.count; i++) {
      uvs.push(uvAttr.getX(i), uvAttr.getY(i));
    }
    
    // Copy indices with offset
    if (indexAttr) {
      for (let i = 0; i < indexAttr.count; i++) {
        indices.push(indexAttr.getX(i) + vertexOffset);
      }
    }
    
    vertexOffset += posAttr.count;
  }
  
  merged.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  merged.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
  merged.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  merged.setIndex(indices);
  
  return merged;
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

  // Load 5 textures for cyclic material assignment
  const textureLoader = new THREE.TextureLoader();
  const textureFiles = [
    '../assets/512x512 Texel Density Texture 1.png',
    '../assets/512x512 Texel Density Texture 2.png',
    '../assets/512x512 Texel Density Texture 3.png',
    '../assets/512x512 Texel Density Texture 4.png',
    '../assets/512x512 Texel Density Texture 5.png',
  ];
  
  const textures = await Promise.all(
    textureFiles.map((file) => new Promise<THREE.Texture>((resolve, reject) => {
      textureLoader.load(
        file,
        (tex: THREE.Texture) => {
          tex.colorSpace = THREE.SRGBColorSpace;
          resolve(tex);
        },
        undefined,
        reject
      );
    }))
  );

  // Create 5 MeshStandardMaterials with 50% diffuse (roughness controls this)
  const solidMaterials: Map<string, THREE.MeshStandardMaterial> = new Map();
  MATERIAL_IDS.forEach((id, index) => {
    solidMaterials.set(id, new THREE.MeshStandardMaterial({
      map: textures[index],
      roughness: 0.5,
      metalness: 0.0,
    }));
  });

  // State
  let currentSolidType: SolidType = 'Icosahedron';
  let currentViewMode: ViewMode | null = null;
  let currentMergedGeometry: THREE.BufferGeometry | null = null;
  let solidGroup: THREE.Group | null = null;
  let wireframeGroup: THREE.Group | null = null;

  /**
   * Creates the geometry and mesh objects for the current solid type using Material.Array
   */
  function createSolidObjects(solidType: SolidType): {
    mergedGeometry: THREE.BufferGeometry;
    meshGroup: THREE.Group;
    wireframe: THREE.Group;
  } {
    // Create model object with Material.Array for cyclic texture assignment
    // Generate cyclic materials based on the number of faces
    const faceCount = FACE_COUNTS[solidType];
    const cyclicMaterials = getCyclicMaterials(faceCount);
    
    const wasmMesh = create_platonic_solid({
      geometry: solidType,
      material: { Array: cyclicMaterials }
    });
    
    // Create a group to hold meshes for each material
    const meshGroup = new THREE.Group();
    const geometries: THREE.BufferGeometry[] = [];
    
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
            const mesh = new THREE.Mesh(geometry, material);
            meshGroup.add(mesh);
          }
        }
      }
    }
    
    // Create merged geometry for wireframe visualization
    const mergedGeometry = mergeBufferGeometries(geometries);

    // Create wireframe visualization group
    const wireframe = new THREE.Group();
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
