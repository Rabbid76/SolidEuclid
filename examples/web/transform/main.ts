import {
  AmbientLight,
  AxesHelper,
  BufferGeometry,
  Color,
  DirectionalLight,
  Group,
  LineSegments,
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
import init, { create_transform, WasmTriangularMesh } from '@solid-euclid';
import {
  createBufferGeometryFromWasm,
  createVertexNormals,
  createWireframeEdges,
  mergeBufferGeometries,
} from '../util/three-util';

type ViewMode = 'geometry' | 'solid' | 'wireframe';
type BaseGeometry = 'Hexahedron' | 'Tetrahedron' | 'Torus' | 'Cylinder';

// JSON shapes matching the serde representation of Transform<AnyModel>
type TransformationJson =
  | { Translation: [number, number, number] }
  | { ScaleXYZ: [number, number, number] }
  | { RotationX: number }
  | { RotationY: number }
  | { RotationZ: number };

type AnyModelJson =
  | { PlatonicSolid: { geometry: string; material: { Uniform: string } } }
  | {
      Torus: {
        major_radius: number;
        minor_radius: number;
        major_segments: number;
        minor_segments: number;
        material: { Uniform: string };
      };
    }
  | {
      Cylinder: {
        radius: number;
        height: number;
        segments: number;
        bottom_edge: string;
        top_edge: string;
        material: { Uniform: string };
      };
    }
  | { Transform: { transformation: TransformationJson; model: AnyModelJson } };

type TransformJson = { transformation: TransformationJson; model: AnyModelJson };

interface TransformConfig {
  baseGeometry: BaseGeometry;
  translationX: number;
  translationY: number;
  translationZ: number;
  rotationX: number; // radians
  rotationY: number; // radians
  rotationZ: number; // radians
  scaleX: number;
  scaleY: number;
  scaleZ: number;
  mirrorX: boolean;
}

const MATERIAL_ID = 'default';

function baseGeometryJson(base: BaseGeometry): AnyModelJson {
  switch (base) {
    case 'Hexahedron':
    case 'Tetrahedron':
      return {
        PlatonicSolid: {
          geometry: base,
          material: { Uniform: MATERIAL_ID },
        },
      };
    case 'Torus':
      return {
        Torus: {
          major_radius: 1.0,
          minor_radius: 0.3,
          major_segments: 36,
          minor_segments: 18,
          material: { Uniform: MATERIAL_ID },
        },
      };
    case 'Cylinder':
      return {
        Cylinder: {
          radius: 0.5,
          height: 1.5,
          segments: 24,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: MATERIAL_ID },
        },
      };
  }
}

/**
 * Builds a nested Transform<AnyModel> JSON: the base model is scaled first,
 * then rotated around X, Y and Z, and finally translated (outermost).
 */
function buildTransformJson(config: TransformConfig): TransformJson {
  let model: AnyModelJson = baseGeometryJson(config.baseGeometry);

  // Innermost: non-uniform scale, optionally mirrored (negative determinant)
  const scaleX = config.mirrorX ? -config.scaleX : config.scaleX;
  model = {
    Transform: {
      transformation: { ScaleXYZ: [scaleX, config.scaleY, config.scaleZ] },
      model,
    },
  };

  // Then rotations, X innermost so the point is transformed by Rz * Ry * Rx
  model = { Transform: { transformation: { RotationX: config.rotationX }, model } };
  model = { Transform: { transformation: { RotationY: config.rotationY }, model } };
  model = { Transform: { transformation: { RotationZ: config.rotationZ }, model } };

  // Outermost: translation
  return {
    transformation: {
      Translation: [config.translationX, config.translationY, config.translationZ],
    },
    model,
  };
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
  camera.position.set(4, 4, 4);
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
  controls.maxDistance = 25;

  // Add AxesHelper
  const axesHelper = new AxesHelper(2);
  scene.add(axesHelper);

  // Load texture for the solid material
  const textureLoader = new TextureLoader();
  const texture = await new Promise<Texture>((resolve, reject) => {
    textureLoader.load(
      '../assets/512x512 Texel Density Texture 1.png',
      (tex: Texture) => {
        tex.colorSpace = SRGBColorSpace;
        tex.wrapS = RepeatWrapping;
        tex.wrapT = RepeatWrapping;
        resolve(tex);
      },
      undefined,
      reject
    );
  });

  // Create MeshStandardMaterial with 50% diffuse (roughness controls this)
  // polygonOffset is used to push the solid geometry slightly back to avoid z-fighting with wireframe
  const solidMaterial = new MeshStandardMaterial({
    map: texture,
    roughness: 0.5,
    metalness: 0.0,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
  });

  // State
  let currentConfig: TransformConfig | null = null;
  let currentGeometries: BufferGeometry[] = [];
  let currentViewMode: ViewMode | null = null;
  let solidGroup: Group | null = null;
  let wireframeGroup: Group | null = null;

  /**
   * Creates the geometry and mesh objects for the current transform configuration
   */
  function createTransformObjects(config: TransformConfig): {
    geometries: BufferGeometry[];
    meshGroup: Group;
    wireframe: Group;
  } {
    const wasmMesh: WasmTriangularMesh = create_transform(buildTransformJson(config));

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
    const wireframeEdges = createWireframeEdges(mergedGeometry);
    wireframe.add(wireframeEdges);
    const vertexNormals = createVertexNormals(mergedGeometry, 0.1);
    wireframe.add(vertexNormals);

    return { geometries, meshGroup, wireframe };
  }

  /**
   * Updates the scene with a new transform configuration
   */
  function setTransformConfig(config: TransformConfig) {
    // Remove old objects from scene
    if (solidGroup) scene.remove(solidGroup);
    if (wireframeGroup) scene.remove(wireframeGroup);

    // Dispose old geometry
    for (const geometry of currentGeometries) {
      geometry.dispose();
    }

    // Dispose the wireframe children's geometries and materials; each update
    // creates new ones, while the solid material is shared and retained
    if (wireframeGroup) {
      for (const child of wireframeGroup.children) {
        const lines = child as LineSegments;
        lines.geometry.dispose();
        const material = lines.material;
        if (Array.isArray(material)) {
          for (const m of material) {
            m.dispose();
          }
        } else {
          material.dispose();
        }
      }
    }

    // Create new objects
    const objects = createTransformObjects(config);
    currentGeometries = objects.geometries;
    solidGroup = objects.meshGroup;
    wireframeGroup = objects.wireframe;
    currentConfig = config;

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

  // Initialize with default configuration
  const defaultConfig: TransformConfig = {
    baseGeometry: 'Hexahedron',
    translationX: 0,
    translationY: 0,
    translationZ: 0,
    rotationX: 0,
    rotationY: 0,
    rotationZ: 0,
    scaleX: 1,
    scaleY: 1,
    scaleZ: 1,
    mirrorX: false,
  };
  setTransformConfig(defaultConfig);
  setViewMode('geometry');

  // Setup menu event listeners
  const baseGeometrySelect = document.getElementById('baseGeometry') as HTMLSelectElement;
  const translateXInput = document.getElementById('translateX') as HTMLInputElement;
  const translateYInput = document.getElementById('translateY') as HTMLInputElement;
  const translateZInput = document.getElementById('translateZ') as HTMLInputElement;
  const translateXValue = document.getElementById('translateXValue');
  const translateYValue = document.getElementById('translateYValue');
  const translateZValue = document.getElementById('translateZValue');
  const rotateXInput = document.getElementById('rotateX') as HTMLInputElement;
  const rotateYInput = document.getElementById('rotateY') as HTMLInputElement;
  const rotateZInput = document.getElementById('rotateZ') as HTMLInputElement;
  const rotateXValue = document.getElementById('rotateXValue');
  const rotateYValue = document.getElementById('rotateYValue');
  const rotateZValue = document.getElementById('rotateZValue');
  const scaleXInput = document.getElementById('scaleX') as HTMLInputElement;
  const scaleYInput = document.getElementById('scaleY') as HTMLInputElement;
  const scaleZInput = document.getElementById('scaleZ') as HTMLInputElement;
  const scaleXValue = document.getElementById('scaleXValue');
  const scaleYValue = document.getElementById('scaleYValue');
  const scaleZValue = document.getElementById('scaleZValue');
  const mirrorXInput = document.getElementById('mirrorX') as HTMLInputElement;
  const viewModeSelect = document.getElementById('viewMode') as HTMLSelectElement;

  const degreesToRadians = (degrees: number) => (degrees * Math.PI) / 180;

  function rebuildTransform() {
    if (!currentConfig) return;
    setTransformConfig({
      baseGeometry: baseGeometrySelect.value as BaseGeometry,
      translationX: parseFloat(translateXInput.value) || 0,
      translationY: parseFloat(translateYInput.value) || 0,
      translationZ: parseFloat(translateZInput.value) || 0,
      rotationX: degreesToRadians(parseFloat(rotateXInput.value) || 0),
      rotationY: degreesToRadians(parseFloat(rotateYInput.value) || 0),
      rotationZ: degreesToRadians(parseFloat(rotateZInput.value) || 0),
      scaleX: parseFloat(scaleXInput.value) || 1,
      scaleY: parseFloat(scaleYInput.value) || 1,
      scaleZ: parseFloat(scaleZInput.value) || 1,
      mirrorX: mirrorXInput.checked,
    });
  }

  function bindSlider(
    input: HTMLInputElement | null,
    valueLabel: HTMLElement | null,
    onUpdate: () => void
  ) {
    if (input) {
      input.addEventListener('input', () => {
        if (valueLabel) {
          valueLabel.textContent = input.value;
        }
        onUpdate();
      });
    }
  }

  if (baseGeometrySelect) {
    baseGeometrySelect.addEventListener('change', rebuildTransform);
  }

  bindSlider(translateXInput, translateXValue, rebuildTransform);
  bindSlider(translateYInput, translateYValue, rebuildTransform);
  bindSlider(translateZInput, translateZValue, rebuildTransform);
  bindSlider(rotateXInput, rotateXValue, rebuildTransform);
  bindSlider(rotateYInput, rotateYValue, rebuildTransform);
  bindSlider(rotateZInput, rotateZValue, rebuildTransform);
  bindSlider(scaleXInput, scaleXValue, rebuildTransform);
  bindSlider(scaleYInput, scaleYValue, rebuildTransform);
  bindSlider(scaleZInput, scaleZValue, rebuildTransform);

  if (mirrorXInput) {
    mirrorXInput.addEventListener('change', rebuildTransform);
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
