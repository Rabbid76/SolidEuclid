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
  SphereGeometry,
  SRGBColorSpace,
  Texture,
  TextureLoader,
} from 'three';
import { WebGPURenderer } from 'three/webgpu';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import init, { create_extrude, WasmTriangularMesh } from '@solid-euclid';
import {
  createBufferGeometryFromWasm,
  createVertexNormals,
  createWireframeEdges,
  mergeBufferGeometries,
} from '../util/three-util';

type ViewMode = 'geometry' | 'solid' | 'wireframe';
type CornerType = 'Edge' | 'SmoothEdge' | 'Chamfer' | 'Fillet';

// Material IDs for extrude (5 regions: bottom_cap, body edges, top_cap)
// Geometry ID mapping: -1 = bottom cap, -2 = top cap, 0+ = profile segments (body)
const EXTRUDE_MATERIALS = ['bottom_cap', 'edge_0', 'edge_1', 'edge_2', 'top_cap'];

interface CornerConfig {
  type: CornerType;
  chamferDistance: number;
  filletRadius: number;
}

interface ExtrudeConfig {
  height: number;
  sampleDistance: number;
  outlinePoints: number;
  outlineRadius: number;
  outlineCorner: CornerConfig;
  holePoints: number;
  holeRadius: number;
  holeCorner: CornerConfig;
  // Custom point positions for outline (in XZ plane, Y is extrusion direction)
  customOutlinePoints: Array<{ x: number; z: number }>;
  // Custom point positions for hole (in XZ plane)
  customHolePoints: Array<{ x: number; z: number }>;
}

/**
 * Generates points for a regular polygon centered at origin
 */
function generateRegularPolygonPoints(
  numPoints: number,
  radius: number
): Array<{ x: number; z: number }> {
  const points: Array<{ x: number; z: number }> = [];
  for (let i = 0; i < numPoints; i++) {
    const angle = (i / numPoints) * Math.PI * 2 - Math.PI / 2;
    points.push({
      x: Math.cos(angle) * radius,
      z: Math.sin(angle) * radius,
    });
  }
  return points;
}

/**
 * Converts a CornerConfig to the WASM API format
 */
function cornerConfigToWasm(
  corner: CornerConfig
): string | { Chamfer: { distance: number } } | { Fillet: { radius: number } } {
  switch (corner.type) {
    case 'Edge':
      return 'Edge';
    case 'SmoothEdge':
      return 'SmoothEdge';
    case 'Chamfer':
      return { Chamfer: { distance: corner.chamferDistance } };
    case 'Fillet':
      return { Fillet: { radius: corner.filletRadius } };
  }
}

/**
 * Creates profile commands from points
 * Points are in XZ plane (X, Z) -> converted to profile XY plane
 */
function createProfileCommands(
  points: Array<{ x: number; z: number }>,
  corner: CornerConfig,
  isHole: boolean
): Array<{ path: object; corner: string | object }> {
  if (points.length < 3) return [];

  const commands: Array<{ path: object; corner: string | object }> = [];
  const cornerWasm = cornerConfigToWasm(corner);

  // For holes, reverse the winding order (clockwise)
  const orderedPoints = isHole ? [...points].reverse() : points;

  // Move to first point - Move commands must always have 'Edge' corner
  commands.push({
    path: { Move: { point: [orderedPoints[0]!.x, orderedPoints[0]!.z] } },
    corner: 'Edge',
  });

  // Line to each subsequent point
  for (let i = 1; i < orderedPoints.length; i++) {
    commands.push({
      path: { Line: { point: [orderedPoints[i]!.x, orderedPoints[i]!.z] } },
      corner: cornerWasm,
    });
  }

  // Close the profile - the corner here applies to the closing point (point[0])
  // This corner treatment is applied at the junction between the closing line
  // and the first line (from point[0] to point[1])
  commands.push({
    path: { Line: { point: [orderedPoints[0]!.x, orderedPoints[0]!.z] } },
    corner: cornerWasm,
  });

  return commands;
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
  const orbitControls = new OrbitControls(camera, renderer.domElement);
  orbitControls.enableDamping = true;
  orbitControls.dampingFactor = 0.05;
  orbitControls.screenSpacePanning = false;
  orbitControls.minDistance = 1;
  orbitControls.maxDistance = 15;

  // Add AxesHelper
  const axesHelper = new AxesHelper(2);
  scene.add(axesHelper);

  // Load textures for materials (5 textures for different extrude regions)
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

  // Create MeshStandardMaterial (fallback)
  const solidMaterial = new MeshStandardMaterial({
    map: textures[0],
    roughness: 0.5,
    metalness: 0.0,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
  });

  // Create materials for extrude using textures (5 different textures for different regions)
  // Maps material ID strings to MeshStandardMaterial instances
  const extrudeMaterials: Map<string, MeshStandardMaterial> = new Map();
  EXTRUDE_MATERIALS.forEach((id, index) => {
    extrudeMaterials.set(
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

  // Point marker material
  const pointMarkerMaterial = new MeshStandardMaterial({
    color: 0xff6600,
    roughness: 0.3,
    metalness: 0.5,
    emissive: 0x442200,
  });

  const holePointMarkerMaterial = new MeshStandardMaterial({
    color: 0x0066ff,
    roughness: 0.3,
    metalness: 0.5,
    emissive: 0x002244,
  });

  // State - defaults: 5-point outline with 4-point hole, transform controls enabled
  let currentConfig: ExtrudeConfig = {
    height: 1.0,
    sampleDistance: 0.05,
    outlinePoints: 5,
    outlineRadius: 0.8,
    outlineCorner: { type: 'Fillet', chamferDistance: 0.1, filletRadius: 0.1 },
    holePoints: 4,
    holeRadius: 0.3,
    holeCorner: { type: 'Fillet', chamferDistance: 0.05, filletRadius: 0.05 },
    customOutlinePoints: generateRegularPolygonPoints(5, 0.8),
    customHolePoints: generateRegularPolygonPoints(4, 0.3),
  };
  let currentViewMode: ViewMode | null = null;
  let currentMergedGeometry: BufferGeometry | null = null;
  let solidGroup: Group | null = null;
  let wireframeGroup: Group | null = null;

  // Transform controls state - enabled by default
  let transformControlsEnabled = true;
  const pointMarkers: Mesh[] = [];
  const holePointMarkers: Mesh[] = [];
  const transformControlsList: TransformControls[] = [];

  /**
   * Creates point markers for transform controls
   */
  function createPointMarkers() {
    // Clear existing markers
    pointMarkers.forEach((marker) => {
      scene.remove(marker);
      marker.geometry.dispose();
    });
    holePointMarkers.forEach((marker) => {
      scene.remove(marker);
      marker.geometry.dispose();
    });
    pointMarkers.length = 0;
    holePointMarkers.length = 0;

    // Clear existing transform controls
    transformControlsList.forEach((tc) => {
      tc.detach();
      scene.remove(tc.getHelper());
      tc.dispose();
    });
    transformControlsList.length = 0;

    const markerGeometry = new SphereGeometry(0.03, 16, 16);

    // Create outline point markers
    currentConfig.customOutlinePoints.forEach((point, index) => {
      const marker = new Mesh(markerGeometry, pointMarkerMaterial);
      marker.position.set(point.x, 0, point.z);
      marker.userData = { type: 'outline', index };
      pointMarkers.push(marker);

      if (transformControlsEnabled) {
        scene.add(marker);
        const tc = new TransformControls(camera, renderer.domElement);
        tc.setMode('translate');
        tc.setSize(0.5);
        tc.attach(marker);
        // Constrain to XZ plane
        tc.showY = false;
        scene.add(tc.getHelper());
        transformControlsList.push(tc);

        tc.addEventListener('dragging-changed', (event) => {
          orbitControls.enabled = !event.value;
          if (!event.value) {
            // Update custom point position when drag ends
            currentConfig.customOutlinePoints[index] = {
              x: marker.position.x,
              z: marker.position.z,
            };
            rebuildExtrude();
          }
        });
      }
    });

    // Create hole point markers
    currentConfig.customHolePoints.forEach((point, index) => {
      const marker = new Mesh(markerGeometry, holePointMarkerMaterial);
      marker.position.set(point.x, 0, point.z);
      marker.userData = { type: 'hole', index };
      holePointMarkers.push(marker);

      if (transformControlsEnabled) {
        scene.add(marker);
        const tc = new TransformControls(camera, renderer.domElement);
        tc.setMode('translate');
        tc.setSize(0.5);
        tc.attach(marker);
        // Constrain to XZ plane
        tc.showY = false;
        scene.add(tc.getHelper());
        transformControlsList.push(tc);

        tc.addEventListener('dragging-changed', (event) => {
          orbitControls.enabled = !event.value;
          if (!event.value) {
            // Update custom point position when drag ends
            currentConfig.customHolePoints[index] = {
              x: marker.position.x,
              z: marker.position.z,
            };
            rebuildExtrude();
          }
        });
      }
    });
  }

  /**
   * Creates the geometry and mesh objects for an extrude
   */
  function createExtrudeObjects(config: ExtrudeConfig): {
    mergedGeometry: BufferGeometry;
    meshGroup: Group;
    wireframe: Group;
  } {
    // Build profile commands from custom points
    const outlineCommands = createProfileCommands(
      config.customOutlinePoints,
      config.outlineCorner,
      false
    );

    // Build hole commands if enabled
    const holeCommands =
      config.holePoints >= 3
        ? createProfileCommands(config.customHolePoints, config.holeCorner, true)
        : [];

    // Combine commands - outline first, then hole (holes are inner contours)
    const allCommands = [...outlineCommands];
    if (holeCommands.length > 0) {
      allCommands.push(...holeCommands);
    }

    console.log(`Profile commands (${allCommands.length} total):`);
    allCommands.forEach((cmd, i) => {
      const pathType = Object.keys(cmd.path)[0];
      const pathPoint = (cmd.path as Record<string, { point: number[] }>)[pathType]?.point;
      const cornerStr = typeof cmd.corner === 'string' ? cmd.corner : JSON.stringify(cmd.corner);
      console.log(
        `  ${i}: ${pathType} to [${pathPoint?.join(', ') ?? 'N/A'}], corner: ${cornerStr}`
      );
    });

    // Create extrude with Array material to get separate geometry groups for each face
    const wasmMesh: WasmTriangularMesh = create_extrude({
      profile: {
        commands: allCommands,
      },
      height: config.height,
      sample_distance: config.sampleDistance,
      material: {
        Array: EXTRUDE_MATERIALS,
      },
    });

    // Create a group to hold meshes
    const meshGroup = new Group();
    const geometries: BufferGeometry[] = [];

    // Create a mesh for each material in the mesh
    // Material IDs from WASM correspond to geometry IDs remapped to material names
    const materialIds = wasmMesh.material_ids as string[];
    console.log(
      `Creating extrude mesh with ${wasmMesh.geometry_count} geometries, material_ids: ${materialIds.join(', ')}`
    );

    for (let i = 0; i < wasmMesh.geometry_count; i++) {
      const wasmGeometry = wasmMesh.get_geometry(i);
      if (wasmGeometry) {
        const geometry = createBufferGeometryFromWasm(wasmGeometry);
        geometries.push(geometry);

        // Select material based on material ID
        const materialId = materialIds[i] ?? '';
        const material = extrudeMaterials.get(materialId) ?? solidMaterial;

        const mesh = new Mesh(geometry, material);
        meshGroup.add(mesh);

        console.log(
          `  Geometry ${i} (${materialId}): ${wasmGeometry.vertex_count} vertices, ${wasmGeometry.triangle_count} triangles`
        );
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
   * Updates the scene with a new extrude configuration
   */
  function setExtrudeConfig(config: ExtrudeConfig) {
    // Remove old objects from scene
    if (solidGroup) scene.remove(solidGroup);
    if (wireframeGroup) scene.remove(wireframeGroup);

    // Dispose old geometry
    if (currentMergedGeometry) {
      currentMergedGeometry.dispose();
    }

    try {
      // Create new objects
      const objects = createExtrudeObjects(config);
      currentMergedGeometry = objects.mergedGeometry;
      solidGroup = objects.meshGroup;
      wireframeGroup = objects.wireframe;
      currentConfig = { ...config };

      // Re-apply current view mode
      const prevMode = currentViewMode;
      currentViewMode = null; // Force update
      setViewMode(prevMode ?? 'geometry');
    } catch (error) {
      console.error('Failed to create extrude:', error);
      // Keep old config on error
    }
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
   * Updates edge parameter UI visibility based on corner type
   */
  function updateCornerUIVisibility(prefix: string, cornerType: CornerType) {
    const chamferLabel = document.getElementById(`${prefix}ChamferDistanceLabel`);
    const filletLabel = document.getElementById(`${prefix}FilletRadiusLabel`);
    if (chamferLabel) {
      chamferLabel.style.display = cornerType === 'Chamfer' ? 'flex' : 'none';
    }
    if (filletLabel) {
      filletLabel.style.display = cornerType === 'Fillet' ? 'flex' : 'none';
    }
  }

  /**
   * Updates hole UI visibility based on hole points
   */
  function updateHoleUIVisibility(holePoints: number) {
    const hasHole = holePoints >= 3;
    const holeRadiusLabel = document.getElementById('holeRadiusLabel');
    const holeCornerTypeLabel = document.getElementById('holeCornerTypeLabel');
    const holeChamferDistanceLabel = document.getElementById('holeChamferDistanceLabel');
    const holeFilletRadiusLabel = document.getElementById('holeFilletRadiusLabel');

    if (holeRadiusLabel) {
      holeRadiusLabel.style.display = hasHole ? 'flex' : 'none';
    }
    if (holeCornerTypeLabel) {
      holeCornerTypeLabel.style.display = hasHole ? 'flex' : 'none';
    }
    if (!hasHole) {
      if (holeChamferDistanceLabel) holeChamferDistanceLabel.style.display = 'none';
      if (holeFilletRadiusLabel) holeFilletRadiusLabel.style.display = 'none';
    } else {
      const holeCornerType = (document.getElementById('holeCornerType') as HTMLSelectElement)
        ?.value;
      updateCornerUIVisibility('hole', holeCornerType as CornerType);
    }
  }

  /**
   * Updates transform controls UI visibility
   */
  function updateTransformControlsUI(enabled: boolean) {
    const transformModeLabel = document.getElementById('transformModeLabel');
    if (transformModeLabel) {
      transformModeLabel.style.display = enabled ? 'flex' : 'none';
    }
  }

  function rebuildExtrude() {
    const heightInput = document.getElementById('height') as HTMLInputElement;
    const sampleDistanceInput = document.getElementById('sampleDistance') as HTMLInputElement;
    const outlinePointsSelect = document.getElementById('outlinePoints') as HTMLSelectElement;
    const outlineRadiusInput = document.getElementById('outlineRadius') as HTMLInputElement;
    const outlineCornerTypeSelect = document.getElementById(
      'outlineCornerType'
    ) as HTMLSelectElement;
    const outlineChamferDistanceInput = document.getElementById(
      'outlineChamferDistance'
    ) as HTMLInputElement;
    const outlineFilletRadiusInput = document.getElementById(
      'outlineFilletRadius'
    ) as HTMLInputElement;
    const holePointsSelect = document.getElementById('holePoints') as HTMLSelectElement;
    const holeRadiusInput = document.getElementById('holeRadius') as HTMLInputElement;
    const holeCornerTypeSelect = document.getElementById('holeCornerType') as HTMLSelectElement;
    const holeChamferDistanceInput = document.getElementById(
      'holeChamferDistance'
    ) as HTMLInputElement;
    const holeFilletRadiusInput = document.getElementById('holeFilletRadius') as HTMLInputElement;

    const height = parseFloat(heightInput?.value) || 1.0;
    const sampleDistance = parseFloat(sampleDistanceInput?.value) || 0.05;
    const outlinePoints = parseInt(outlinePointsSelect?.value, 10) || 4;
    const outlineRadius = parseFloat(outlineRadiusInput?.value) || 0.8;
    const outlineCornerType = (outlineCornerTypeSelect?.value || 'Edge') as CornerType;
    const outlineChamferDistance = parseFloat(outlineChamferDistanceInput?.value) || 0.1;
    const outlineFilletRadius = parseFloat(outlineFilletRadiusInput?.value) || 0.1;
    const holePoints = parseInt(holePointsSelect?.value, 10) || 0;
    const holeRadius = parseFloat(holeRadiusInput?.value) || 0.3;
    const holeCornerType = (holeCornerTypeSelect?.value || 'Edge') as CornerType;
    const holeChamferDistance = parseFloat(holeChamferDistanceInput?.value) || 0.05;
    const holeFilletRadius = parseFloat(holeFilletRadiusInput?.value) || 0.05;

    // Check if outline points changed - regenerate points
    let customOutlinePoints = currentConfig.customOutlinePoints;
    if (outlinePoints !== currentConfig.outlinePoints) {
      customOutlinePoints = generateRegularPolygonPoints(outlinePoints, outlineRadius);
    }

    // Check if hole points changed - regenerate points
    let customHolePoints = currentConfig.customHolePoints;
    if (holePoints !== currentConfig.holePoints) {
      customHolePoints =
        holePoints >= 3 ? generateRegularPolygonPoints(holePoints, holeRadius) : [];
    }

    const newConfig: ExtrudeConfig = {
      height,
      sampleDistance,
      outlinePoints,
      outlineRadius,
      outlineCorner: {
        type: outlineCornerType,
        chamferDistance: outlineChamferDistance,
        filletRadius: outlineFilletRadius,
      },
      holePoints,
      holeRadius,
      holeCorner: {
        type: holeCornerType,
        chamferDistance: holeChamferDistance,
        filletRadius: holeFilletRadius,
      },
      customOutlinePoints,
      customHolePoints,
    };

    setExtrudeConfig(newConfig);

    // Recreate point markers if transform controls are enabled
    if (transformControlsEnabled) {
      createPointMarkers();
    }
  }

  // Initialize with default geometry view
  setExtrudeConfig(currentConfig);
  setViewMode('geometry');

  // Initialize point markers since transform controls are enabled by default
  createPointMarkers();

  // Setup menu event listeners
  const heightInput = document.getElementById('height') as HTMLInputElement;
  const heightValue = document.getElementById('heightValue');
  const sampleDistanceInput = document.getElementById('sampleDistance') as HTMLInputElement;
  const outlinePointsSelect = document.getElementById('outlinePoints') as HTMLSelectElement;
  const outlineRadiusInput = document.getElementById('outlineRadius') as HTMLInputElement;
  const outlineCornerTypeSelect = document.getElementById('outlineCornerType') as HTMLSelectElement;
  const outlineChamferDistanceInput = document.getElementById(
    'outlineChamferDistance'
  ) as HTMLInputElement;
  const outlineFilletRadiusInput = document.getElementById(
    'outlineFilletRadius'
  ) as HTMLInputElement;
  const outlineChamferDistanceValue = document.getElementById('outlineChamferDistanceValue');
  const outlineFilletRadiusValue = document.getElementById('outlineFilletRadiusValue');
  const holePointsSelect = document.getElementById('holePoints') as HTMLSelectElement;
  const holeRadiusInput = document.getElementById('holeRadius') as HTMLInputElement;
  const holeCornerTypeSelect = document.getElementById('holeCornerType') as HTMLSelectElement;
  const holeChamferDistanceInput = document.getElementById(
    'holeChamferDistance'
  ) as HTMLInputElement;
  const holeFilletRadiusInput = document.getElementById('holeFilletRadius') as HTMLInputElement;
  const holeChamferDistanceValue = document.getElementById('holeChamferDistanceValue');
  const holeFilletRadiusValue = document.getElementById('holeFilletRadiusValue');
  const showTransformControlsCheckbox = document.getElementById(
    'showTransformControls'
  ) as HTMLInputElement;
  const viewModeSelect = document.getElementById('viewMode') as HTMLSelectElement;

  if (heightInput) {
    heightInput.addEventListener('input', () => {
      if (heightValue) {
        heightValue.textContent = heightInput.value;
      }
      rebuildExtrude();
    });
  }

  if (sampleDistanceInput) {
    sampleDistanceInput.addEventListener('change', rebuildExtrude);
  }

  if (outlinePointsSelect) {
    outlinePointsSelect.addEventListener('change', rebuildExtrude);
  }

  if (outlineRadiusInput) {
    outlineRadiusInput.addEventListener('change', () => {
      // Regenerate outline points with new radius
      const outlinePoints = parseInt(outlinePointsSelect?.value, 10) || 4;
      const outlineRadius = parseFloat(outlineRadiusInput.value) || 0.8;
      currentConfig.customOutlinePoints = generateRegularPolygonPoints(
        outlinePoints,
        outlineRadius
      );
      rebuildExtrude();
    });
  }

  if (outlineCornerTypeSelect) {
    outlineCornerTypeSelect.addEventListener('change', () => {
      updateCornerUIVisibility('outline', outlineCornerTypeSelect.value as CornerType);
      rebuildExtrude();
    });
  }

  if (outlineChamferDistanceInput) {
    outlineChamferDistanceInput.addEventListener('input', () => {
      if (outlineChamferDistanceValue) {
        outlineChamferDistanceValue.textContent = outlineChamferDistanceInput.value;
      }
      rebuildExtrude();
    });
  }

  if (outlineFilletRadiusInput) {
    outlineFilletRadiusInput.addEventListener('input', () => {
      if (outlineFilletRadiusValue) {
        outlineFilletRadiusValue.textContent = outlineFilletRadiusInput.value;
      }
      rebuildExtrude();
    });
  }

  if (holePointsSelect) {
    holePointsSelect.addEventListener('change', () => {
      const holePoints = parseInt(holePointsSelect.value, 10) || 0;
      updateHoleUIVisibility(holePoints);
      rebuildExtrude();
    });
  }

  if (holeRadiusInput) {
    holeRadiusInput.addEventListener('change', () => {
      // Regenerate hole points with new radius
      const holePoints = parseInt(holePointsSelect?.value, 10) || 0;
      const holeRadius = parseFloat(holeRadiusInput.value) || 0.3;
      if (holePoints >= 3) {
        currentConfig.customHolePoints = generateRegularPolygonPoints(holePoints, holeRadius);
      }
      rebuildExtrude();
    });
  }

  if (holeCornerTypeSelect) {
    holeCornerTypeSelect.addEventListener('change', () => {
      updateCornerUIVisibility('hole', holeCornerTypeSelect.value as CornerType);
      rebuildExtrude();
    });
  }

  if (holeChamferDistanceInput) {
    holeChamferDistanceInput.addEventListener('input', () => {
      if (holeChamferDistanceValue) {
        holeChamferDistanceValue.textContent = holeChamferDistanceInput.value;
      }
      rebuildExtrude();
    });
  }

  if (holeFilletRadiusInput) {
    holeFilletRadiusInput.addEventListener('input', () => {
      if (holeFilletRadiusValue) {
        holeFilletRadiusValue.textContent = holeFilletRadiusInput.value;
      }
      rebuildExtrude();
    });
  }

  if (showTransformControlsCheckbox) {
    showTransformControlsCheckbox.addEventListener('change', () => {
      transformControlsEnabled = showTransformControlsCheckbox.checked;
      updateTransformControlsUI(transformControlsEnabled);
      createPointMarkers();
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

  // Add DirectionalLight (50% intensity)
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
    orbitControls.update();
    renderer.render(scene, camera);
  });
}

main().catch(console.error);
