import {
  BufferAttribute,
  BufferGeometry,
  EdgesGeometry,
  Float32BufferAttribute,
  LineBasicMaterial,
  LineSegments,
} from 'three';
import type { WasmTriangularGeometry } from '@solid-euclid';

/**
 * Creates a Three.js BufferGeometry from a WasmTriangularGeometry
 */
export function createBufferGeometryFromWasm(wasmGeometry: WasmTriangularGeometry): BufferGeometry {
  const geometry = new BufferGeometry();

  // Copy typed arrays from WASM (view is temporary, need to copy)
  const vertices = new Float32Array(wasmGeometry.vertices);
  const normals = new Float32Array(wasmGeometry.normals);
  const uvs = new Float32Array(wasmGeometry.uvs);
  const indices = new Uint32Array(wasmGeometry.indices);

  geometry.setAttribute('position', new BufferAttribute(vertices, 3));
  geometry.setAttribute('normal', new BufferAttribute(normals, 3));
  geometry.setAttribute('uv', new BufferAttribute(uvs, 2));
  geometry.setIndex(new BufferAttribute(indices, 1));

  return geometry;
}

/**
 * Merges multiple BufferGeometries into one
 */
export function mergeBufferGeometries(geometries: BufferGeometry[]): BufferGeometry {
  if (geometries.length === 0) {
    return new BufferGeometry();
  }

  if (geometries.length === 1) {
    return geometries[0]!;
  }

  const merged = new BufferGeometry();

  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  let vertexOffset = 0;

  for (const geo of geometries) {
    const posAttr = geo.getAttribute('position') as BufferAttribute;
    const normAttr = geo.getAttribute('normal') as BufferAttribute;
    const uvAttr = geo.getAttribute('uv') as BufferAttribute;
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

  merged.setAttribute('position', new Float32BufferAttribute(positions, 3));
  merged.setAttribute('normal', new Float32BufferAttribute(normals, 3));
  merged.setAttribute('uv', new Float32BufferAttribute(uvs, 2));
  merged.setIndex(indices);

  return merged;
}

/**
 * Creates wireframe edges from a BufferGeometry showing all triangle edges
 */
export function createWireframeEdges(geometry: BufferGeometry): LineSegments {
  const edgesGeometry = new EdgesGeometry(geometry, 0); // threshold 0 to show all edges
  const edgesMaterial = new LineBasicMaterial({ color: 0xffffff });
  return new LineSegments(edgesGeometry, edgesMaterial);
}

/**
 * Creates vertex normal visualization lines from a BufferGeometry
 */
export function createVertexNormals(geometry: BufferGeometry, length: number = 0.1): LineSegments {
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

  const normalGeometry = new BufferGeometry();
  normalGeometry.setAttribute('position', new Float32BufferAttribute(positions, 3));

  const normalMaterial = new LineBasicMaterial({ color: 0xff0000 });
  return new LineSegments(normalGeometry, normalMaterial);
}

/**
 * Generates a cyclic material array for a given number of faces
 */
export function getCyclicMaterials(faceCount: number, materialIds: string[]): string[] {
  return Array.from({ length: faceCount }, (_, i) => materialIds[i % materialIds.length]!);
}
