import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_extrude } from '@solid-euclid';

describe('Extrude WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  // Helper function to create a square profile
  const createSquareProfile = () => ({
    commands: [
      { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
      { path: { Line: { point: [1.0, 0.0] } }, corner: 'Edge' },
      { path: { Line: { point: [1.0, 1.0] } }, corner: 'Edge' },
      { path: { Line: { point: [0.0, 1.0] } }, corner: 'Edge' },
      { path: { Line: { point: [0.0, 0.0] } }, corner: 'Edge' },
    ],
  });

  // Helper function to create a triangle profile
  const createTriangleProfile = () => ({
    commands: [
      { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
      { path: { Line: { point: [1.0, 0.0] } }, corner: 'Edge' },
      { path: { Line: { point: [0.5, 0.866] } }, corner: 'Edge' },
      { path: { Line: { point: [0.0, 0.0] } }, corner: 'Edge' },
    ],
  });

  describe('create_extrude', () => {
    describe('basic extrusion creation', () => {
      it('should create a square extrusion with uniform material', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.material_ids).toContain('default');

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);
        expect(geo!.triangle_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a triangle extrusion with uniform material', () => {
        const mesh = create_extrude({
          profile: createTriangleProfile(),
          height: 1.5,
          sample_distance: 0.1,
          material: { Uniform: 'metal' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.material_ids).toContain('metal');

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);
        expect(geo!.triangle_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create an extrusion with default material', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Default material creates separate materials for each profile segment and caps
        expect(mesh.geometry_count).toBeGreaterThanOrEqual(3);

        mesh.free();
      });
    });

    describe('corner types', () => {
      it('should create an extrusion with filleted corners', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.0] } }, corner: { Fillet: { radius: 0.1 } } },
              { path: { Line: { point: [1.0, 1.0] } }, corner: { Fillet: { radius: 0.1 } } },
              { path: { Line: { point: [0.0, 1.0] } }, corner: { Fillet: { radius: 0.1 } } },
              { path: { Line: { point: [0.0, 0.0] } }, corner: { Fillet: { radius: 0.1 } } },
            ],
          },
          height: 2.0,
          sample_distance: 0.05,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create an extrusion with chamfered corners', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.0] } }, corner: { Chamfer: { distance: 0.1 } } },
              { path: { Line: { point: [1.0, 1.0] } }, corner: { Chamfer: { distance: 0.1 } } },
              { path: { Line: { point: [0.0, 1.0] } }, corner: { Chamfer: { distance: 0.1 } } },
              { path: { Line: { point: [0.0, 0.0] } }, corner: { Chamfer: { distance: 0.1 } } },
            ],
          },
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create an extrusion with smooth edge corners', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.0] } }, corner: 'SmoothEdge' },
              { path: { Line: { point: [1.0, 1.0] } }, corner: 'SmoothEdge' },
              { path: { Line: { point: [0.0, 1.0] } }, corner: 'SmoothEdge' },
              { path: { Line: { point: [0.0, 0.0] } }, corner: 'SmoothEdge' },
            ],
          },
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });

      it('should create an extrusion with mixed corner types', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.0] } }, corner: { Fillet: { radius: 0.1 } } },
              { path: { Line: { point: [1.0, 1.0] } }, corner: { Chamfer: { distance: 0.1 } } },
              { path: { Line: { point: [0.0, 1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [0.0, 0.0] } }, corner: 'SmoothEdge' },
            ],
          },
          height: 2.0,
          sample_distance: 0.05,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });
    });

    describe('height variations', () => {
      it('should handle small height', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 0.5,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should handle large height', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 10.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create extrusions with different heights', () => {
        const heights = [0.5, 1.0, 2.0, 5.0];

        for (const height of heights) {
          const mesh = create_extrude({
            profile: createSquareProfile(),
            height,
            sample_distance: 0.1,
            material: { Uniform: 'default' },
          });

          expect(mesh).toBeDefined();
          expect(mesh.geometry_count).toBe(1);
          expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

          mesh.free();
        }
      });
    });

    describe('sample distance variations', () => {
      it('should handle different sample distances', () => {
        const sampleDistances = [0.01, 0.05, 0.1, 0.2, 0.5];

        for (const sample_distance of sampleDistances) {
          const mesh = create_extrude({
            profile: createSquareProfile(),
            height: 2.0,
            sample_distance,
            material: { Uniform: 'default' },
          });

          expect(mesh).toBeDefined();
          expect(mesh.geometry_count).toBe(1);

          mesh.free();
        }
      });

      it('should produce more vertices with smaller sample distance for curved corners', () => {
        const profileWithFillets = {
          commands: [
            { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
            { path: { Line: { point: [1.0, 0.0] } }, corner: { Fillet: { radius: 0.2 } } },
            { path: { Line: { point: [1.0, 1.0] } }, corner: { Fillet: { radius: 0.2 } } },
            { path: { Line: { point: [0.0, 1.0] } }, corner: { Fillet: { radius: 0.2 } } },
            { path: { Line: { point: [0.0, 0.0] } }, corner: { Fillet: { radius: 0.2 } } },
          ],
        };

        const coarseMesh = create_extrude({
          profile: profileWithFillets,
          height: 2.0,
          sample_distance: 0.2,
          material: { Uniform: 'default' },
        });

        const fineMesh = create_extrude({
          profile: profileWithFillets,
          height: 2.0,
          sample_distance: 0.02,
          material: { Uniform: 'default' },
        });

        const coarseGeo = coarseMesh.get_geometry(0)!;
        const fineGeo = fineMesh.get_geometry(0)!;

        // Finer sample distance should produce more triangles for curved corners
        expect(fineGeo.triangle_count).toBeGreaterThanOrEqual(coarseGeo.triangle_count);

        coarseMesh.free();
        fineMesh.free();
      });
    });

    describe('geometry data validation', () => {
      it('should have consistent geometry data sizes', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // vertices and normals should have same length (3 components per vertex)
        expect(geo.vertices.length).toBe(geo.normals.length);
        // uvs have 2 components per vertex, vertices have 3
        expect(geo.uvs.length * 3).toBe(geo.vertices.length * 2);

        mesh.free();
      });

      it('should have normalized normals', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;
        const normals = geo.normals;

        for (let i = 0; i < normals.length; i += 3) {
          const nx = normals[i];
          const ny = normals[i + 1];
          const nz = normals[i + 2];
          const length = Math.sqrt(nx * nx + ny * ny + nz * nz);
          expect(length).toBeCloseTo(1.0, 2);
        }

        mesh.free();
      });

      it('should have indices within vertex bounds', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;
        const vertexCount = geo.vertex_count;
        const indices = geo.indices;

        for (let i = 0; i < indices.length; i++) {
          expect(indices[i]).toBeLessThan(vertexCount);
        }

        mesh.free();
      });

      it('should have non-empty geometry arrays', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices.length).toBeGreaterThan(0);
        expect(geo.normals.length).toBeGreaterThan(0);
        expect(geo.uvs.length).toBeGreaterThan(0);
        expect(geo.indices.length).toBeGreaterThan(0);

        mesh.free();
      });
    });

    describe('material variations', () => {
      it('should create extrusion with array materials', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Array: ['body', 'bottom_cap', 'top_cap'] },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBeGreaterThanOrEqual(1);

        mesh.free();
      });

      it('should support Map material for custom face mapping', () => {
        // JavaScript Map is required for HashMap<i32, String> as JS object keys are always strings
        const faceMap = new Map<number, string>();
        faceMap.set(-1, 'bottom');
        faceMap.set(-2, 'top');

        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: {
            Map: {
              default: 'base',
              face_map: faceMap,
            },
          },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBeGreaterThanOrEqual(1);

        mesh.free();
      });

      it('should get geometry by material name', () => {
        const mesh = create_extrude({
          profile: createSquareProfile(),
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'my_material' },
        });

        const geometry = mesh.get_geometry_by_material('my_material');
        const missing = mesh.get_geometry_by_material('nonexistent');

        expect(geometry).toBeDefined();
        expect(missing).toBeUndefined();

        mesh.free();
      });
    });

    describe('complex profiles', () => {
      it('should create an L-shaped extrusion', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 0.5] } }, corner: 'Edge' },
              { path: { Line: { point: [0.5, 0.5] } }, corner: 'Edge' },
              { path: { Line: { point: [0.5, 1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [0.0, 1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [0.0, 0.0] } }, corner: 'Edge' },
            ],
          },
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a pentagon extrusion', () => {
        // Regular pentagon vertices
        const angle = (2 * Math.PI) / 5;
        const commands = [
          { path: { Move: { point: [Math.cos(0), Math.sin(0)] } }, corner: 'Edge' },
        ];
        for (let i = 1; i <= 5; i++) {
          const a = i * angle;
          commands.push({
            path: { Line: { point: [Math.cos(a), Math.sin(a)] } },
            corner: 'Edge',
          });
        }

        const mesh = create_extrude({
          profile: { commands },
          height: 1.5,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a hexagon extrusion with filleted corners', () => {
        // Regular hexagon vertices
        const angle = (2 * Math.PI) / 6;
        const commands = [
          { path: { Move: { point: [Math.cos(0), Math.sin(0)] } }, corner: 'Edge' },
        ];
        for (let i = 1; i <= 6; i++) {
          const a = i * angle;
          commands.push({
            path: { Line: { point: [Math.cos(a), Math.sin(a)] } },
            corner: { Fillet: { radius: 0.1 } },
          });
        }

        const mesh = create_extrude({
          profile: { commands },
          height: 2.0,
          sample_distance: 0.05,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.get_geometry(0)!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });
    });

    describe('edge cases', () => {
      it('should handle very small profiles', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [0.0, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [0.01, 0.0] } }, corner: 'Edge' },
              { path: { Line: { point: [0.01, 0.01] } }, corner: 'Edge' },
              { path: { Line: { point: [0.0, 0.01] } }, corner: 'Edge' },
              { path: { Line: { point: [0.0, 0.0] } }, corner: 'Edge' },
            ],
          },
          height: 0.05,
          sample_distance: 0.01,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });

      it('should handle profiles not centered at origin', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [10.0, 10.0] } }, corner: 'Edge' },
              { path: { Line: { point: [11.0, 10.0] } }, corner: 'Edge' },
              { path: { Line: { point: [11.0, 11.0] } }, corner: 'Edge' },
              { path: { Line: { point: [10.0, 11.0] } }, corner: 'Edge' },
              { path: { Line: { point: [10.0, 10.0] } }, corner: 'Edge' },
            ],
          },
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });

      it('should handle profiles with negative coordinates', () => {
        const mesh = create_extrude({
          profile: {
            commands: [
              { path: { Move: { point: [-1.0, -1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, -1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [1.0, 1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [-1.0, 1.0] } }, corner: 'Edge' },
              { path: { Line: { point: [-1.0, -1.0] } }, corner: 'Edge' },
            ],
          },
          height: 2.0,
          sample_distance: 0.1,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });
    });
  });
});
