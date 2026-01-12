import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_capsule } from '@solid-euclid';

describe('Capsule WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('create_capsule', () => {
    describe('basic capsule creation', () => {
      it('should create a capsule with uniform material', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 24,
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

      it('should create a capsule with default material', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        // Default material creates 3 geometry regions for capsule
        expect(mesh.geometry_count).toBe(3);

        mesh.free();
      });

      it('should create a capsule with 3 materials (bottom, body, top)', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Array: ['bottom', 'body', 'top'] },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(3);
        expect(mesh.material_ids).toContain('bottom');
        expect(mesh.material_ids).toContain('body');
        expect(mesh.material_ids).toContain('top');

        mesh.free();
      });

      it('should handle different segment counts', () => {
        const lowRes = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 6,
          material: { Uniform: 'default' },
        });

        const highRes = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 24,
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

    describe('capsule geometry variations', () => {
      it('should create a capsule with zero height (sphere-like)', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 0.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a tall capsule', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 10.0,
          segments: 24,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a thin capsule', () => {
        const mesh = create_capsule({
          radius: 0.1,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a fat capsule', () => {
        const mesh = create_capsule({
          radius: 2.0,
          height: 1.0,
          segments: 24,
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
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
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
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // vertex_count should match vertices.length / 3
        expect(geo.vertex_count).toBe(geo.vertices.length / 3);

        mesh.free();
      });

      it('should have correct triangle count', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // triangle_count should match indices.length / 3
        expect(geo.triangle_count).toBe(geo.indices.length / 3);

        mesh.free();
      });

      it('should have indices within bounds', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;
        const vertexCount = geo.vertex_count;

        for (let i = 0; i < geo.indices.length; i++) {
          expect(geo.indices[i]).toBeLessThan(vertexCount);
        }

        mesh.free();
      });

      it('should have normalized normals', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        for (let i = 0; i < geo.normals.length; i += 3) {
          const nx = geo.normals[i];
          const ny = geo.normals[i + 1];
          const nz = geo.normals[i + 2];
          const length = Math.sqrt(nx * nx + ny * ny + nz * nz);
          expect(length).toBeCloseTo(1.0, 2);
        }

        mesh.free();
      });
    });

    describe('get_geometry methods', () => {
      it('should return undefined for out-of-bounds index', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'default' },
        });

        expect(mesh.get_geometry(999)).toBeUndefined();

        mesh.free();
      });

      it('should get geometry by material ID', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Uniform: 'my_material' },
        });

        const geo = mesh.get_geometry_by_material('my_material');
        const missing = mesh.get_geometry_by_material('nonexistent');

        expect(geo).toBeDefined();
        expect(missing).toBeUndefined();

        mesh.free();
      });

      it('should get multiple geometries with Array material', () => {
        const mesh = create_capsule({
          radius: 0.5,
          height: 2.0,
          segments: 12,
          material: { Array: ['bottom_mat', 'body_mat', 'top_mat'] },
        });

        const bottomGeo = mesh.get_geometry_by_material('bottom_mat');
        const bodyGeo = mesh.get_geometry_by_material('body_mat');
        const topGeo = mesh.get_geometry_by_material('top_mat');

        expect(bottomGeo).toBeDefined();
        expect(bodyGeo).toBeDefined();
        expect(topGeo).toBeDefined();

        mesh.free();
      });
    });

    describe('error handling', () => {
      it('should throw error for invalid capsule structure', () => {
        expect(() => {
          create_capsule({
            invalid: 'structure',
          });
        }).toThrow();
      });

      it('should throw error for null input', () => {
        expect(() => {
          create_capsule(null);
        }).toThrow();
      });

      it('should throw error for missing radius', () => {
        expect(() => {
          create_capsule({
            height: 2.0,
            segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for missing height', () => {
        expect(() => {
          create_capsule({
            radius: 0.5,
            segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for missing segments', () => {
        expect(() => {
          create_capsule({
            radius: 0.5,
            height: 2.0,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for zero radius', () => {
        expect(() => {
          create_capsule({
            radius: 0.0,
            height: 2.0,
            segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for negative radius', () => {
        expect(() => {
          create_capsule({
            radius: -0.5,
            height: 2.0,
            segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for negative height', () => {
        expect(() => {
          create_capsule({
            radius: 0.5,
            height: -2.0,
            segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for insufficient segments', () => {
        expect(() => {
          create_capsule({
            radius: 0.5,
            height: 2.0,
            segments: 2,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });
    });
  });
});
