import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_cylinder } from '@solid-euclid';

describe('Cylinder WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('create_cylinder', () => {
    describe('basic cylinder creation', () => {
      it('should create a cylinder with uniform material', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 24,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
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

      it('should create a cylinder with default material and sharp edges', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Sharp edges: only 3 regions (bottom_cap, body, top_cap)
        expect(mesh.geometry_count).toBe(3);

        mesh.free();
      });

      it('should create a cylinder with default material and smooth edges', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'SmoothEdge',
          top_edge: 'SmoothEdge',
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Smooth edges: same as sharp (3 regions)
        expect(mesh.geometry_count).toBe(3);

        mesh.free();
      });

      it('should create a cylinder with chamfered edges and 5 materials', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Chamfer: { distance: 0.1 } },
          top_edge: { Chamfer: { distance: 0.1 } },
          material: { Array: ['bottom_cap', 'bottom_edge', 'body', 'top_edge', 'top_cap'] },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(5);
        expect(mesh.material_ids).toContain('bottom_cap');
        expect(mesh.material_ids).toContain('bottom_edge');
        expect(mesh.material_ids).toContain('body');
        expect(mesh.material_ids).toContain('top_edge');
        expect(mesh.material_ids).toContain('top_cap');

        mesh.free();
      });

      it('should create a cylinder with filleted edges and 5 materials', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Fillet: { radius: 0.1 } },
          top_edge: { Fillet: { radius: 0.1 } },
          material: { Array: ['bottom_cap', 'bottom_edge', 'body', 'top_edge', 'top_cap'] },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(5);

        mesh.free();
      });

      it('should handle different segment counts', () => {
        const lowRes = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 6,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const highRes = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 24,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const lowGeo = lowRes.get_geometry(0)!;
        const highGeo = highRes.get_geometry(0)!;

        // Higher resolution should have more triangles
        expect(highGeo.triangle_count).toBeGreaterThan(lowGeo.triangle_count);

        lowRes.free();
        highRes.free();
      });
    });

    describe('edge corner types', () => {
      it('should create a cylinder with mixed edge types (chamfer bottom, fillet top)', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Chamfer: { distance: 0.1 } },
          top_edge: { Fillet: { radius: 0.1 } },
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Both edges have corners, so 5 regions
        expect(mesh.geometry_count).toBe(5);

        mesh.free();
      });

      it('should create a cylinder with one sharp edge and one chamfered edge', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: { Chamfer: { distance: 0.1 } },
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Sharp bottom, chamfered top: 4 regions
        expect(mesh.geometry_count).toBe(4);

        mesh.free();
      });

      it('should create a cylinder with one sharp edge and one filleted edge', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Fillet: { radius: 0.1 } },
          top_edge: 'Edge',
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Filleted bottom, sharp top: 4 regions
        expect(mesh.geometry_count).toBe(4);

        mesh.free();
      });

      it('should handle various chamfer distances', () => {
        const small = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Chamfer: { distance: 0.05 } },
          top_edge: { Chamfer: { distance: 0.05 } },
          material: { Uniform: 'default' },
        });

        const large = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Chamfer: { distance: 0.2 } },
          top_edge: { Chamfer: { distance: 0.2 } },
          material: { Uniform: 'default' },
        });

        expect(small).toBeDefined();
        expect(large).toBeDefined();

        const smallGeo = small.get_geometry(0)!;
        const largeGeo = large.get_geometry(0)!;
        expect(smallGeo.vertex_count).toBeGreaterThan(0);
        expect(largeGeo.vertex_count).toBeGreaterThan(0);

        small.free();
        large.free();
      });

      it('should handle various fillet radii', () => {
        // Test small fillet
        const small = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Fillet: { radius: 0.05 } },
          top_edge: { Fillet: { radius: 0.05 } },
          material: { Uniform: 'default' },
        });

        expect(small).toBeDefined();
        const smallGeo = small.get_geometry(0)!;
        expect(smallGeo.vertex_count).toBeGreaterThan(0);
        small.free();

        // Test large fillet
        const large = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Fillet: { radius: 0.2 } },
          top_edge: { Fillet: { radius: 0.2 } },
          material: { Uniform: 'default' },
        });

        expect(large).toBeDefined();
        const largeGeo = large.get_geometry(0)!;
        expect(largeGeo.vertex_count).toBeGreaterThan(0);
        large.free();
      });
    });

    describe('cylinder geometry variations', () => {
      it('should create a short cylinder', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 0.5,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a tall cylinder', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 10.0,
          segments: 24,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a thin cylinder', () => {
        const mesh = create_cylinder({
          radius: 0.1,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a fat cylinder (disc-like)', () => {
        const mesh = create_cylinder({
          radius: 2.0,
          height: 0.5,
          segments: 24,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });
    });

    describe('mesh data structure', () => {
      it('should provide valid vertex data', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices).toBeInstanceOf(Float32Array);
        expect(geo.normals).toBeInstanceOf(Float32Array);
        expect(geo.uvs).toBeInstanceOf(Float32Array);
        expect(geo.indices).toBeInstanceOf(Uint32Array);

        expect(geo.vertices.length).toBeGreaterThan(0);
        expect(geo.normals.length).toBeGreaterThan(0);
        expect(geo.uvs.length).toBeGreaterThan(0);
        expect(geo.indices.length).toBeGreaterThan(0);

        // Consistency checks
        expect(geo.vertices.length).toBe(geo.normals.length);
        expect(geo.uvs.length * 3).toBe(geo.vertices.length * 2);

        mesh.free();
      });

      it('should have correct vertex count', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // vertex_count should match vertices.length / 3
        expect(geo.vertex_count).toBe(geo.vertices.length / 3);

        mesh.free();
      });

      it('should have correct triangle count', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // triangle_count should match indices.length / 3
        expect(geo.triangle_count).toBe(geo.indices.length / 3);

        mesh.free();
      });

      it('should have indices within bounds', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;
        const vertexCount = geo.vertex_count;
        const indices = geo.indices;

        // Check first few and last few indices
        expect(indices[0]).toBeLessThan(vertexCount);
        expect(indices[indices.length - 1]).toBeLessThan(vertexCount);

        mesh.free();
      });

      it('should have normalized normals', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;
        const normals = geo.normals;

        // Check first normal
        const nx = normals[0]!;
        const ny = normals[1]!;
        const nz = normals[2]!;
        const length = Math.sqrt(nx * nx + ny * ny + nz * nz);

        expect(length).toBeCloseTo(1.0, 1);

        mesh.free();
      });
    });

    describe('geometry by material', () => {
      it('should get geometry by material name', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: 'Edge',
          top_edge: 'Edge',
          material: { Uniform: 'my_material' },
        });

        const geometry = mesh.get_geometry_by_material('my_material');
        const missing = mesh.get_geometry_by_material('nonexistent');

        expect(geometry).toBeDefined();
        expect(missing).toBeUndefined();

        mesh.free();
      });

      it('should access multiple material geometries', () => {
        const mesh = create_cylinder({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          bottom_edge: { Chamfer: { distance: 0.1 } },
          top_edge: { Chamfer: { distance: 0.1 } },
          material: { Array: ['bottom_cap', 'bottom_edge', 'body', 'top_edge', 'top_cap'] },
        });

        expect(mesh.geometry_count).toBe(5);

        // Each material should have valid geometry
        for (let i = 0; i < mesh.geometry_count; i++) {
          const geo = mesh.get_geometry(i);
          expect(geo).toBeDefined();
          expect(geo!.vertex_count).toBeGreaterThan(0);
        }

        mesh.free();
      });
    });
  });
});
