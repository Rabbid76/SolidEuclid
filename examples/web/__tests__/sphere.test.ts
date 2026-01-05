import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_sphere, get_sphere_types } from '@solid-euclid';

describe('Sphere WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('get_sphere_types', () => {
    it('should return an array of available sphere types', () => {
      const types = get_sphere_types();
      expect(types).toBeInstanceOf(Array);
      expect(types).toHaveLength(2);
      expect(types).toContain('Slices');
      expect(types).toContain('Cube');
    });
  });

  describe('create_sphere', () => {
    describe('Slices sphere', () => {
      it('should create a UV sphere with uniform material', () => {
        const mesh = create_sphere({
          geometry: { Slices: { slices: 8, segments: 16 } },
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

      it('should create a UV sphere with default material (single face)', () => {
        const mesh = create_sphere({
          geometry: { Slices: { slices: 8, segments: 16 } },
          material: 'Default',
        });

        // Slices sphere has only 1 face
        expect(mesh.geometry_count).toBe(1);

        mesh.free();
      });

      it('should handle different resolution parameters', () => {
        const lowRes = create_sphere({
          geometry: { Slices: { slices: 4, segments: 8 } },
          material: { Uniform: 'default' },
        });

        const highRes = create_sphere({
          geometry: { Slices: { slices: 32, segments: 64 } },
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

    describe('Cube sphere', () => {
      it('should create a cube-mapped sphere with uniform material', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);
        expect(geo!.triangle_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create cube sphere with 6 materials for 6 faces', () => {
        const materials = ['face0', 'face1', 'face2', 'face3', 'face4', 'face5'];
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Array: materials },
        });

        // Cube sphere has 6 faces
        expect(mesh.geometry_count).toBe(6);
        expect(mesh.material_ids).toHaveLength(6);

        for (const mat of materials) {
          expect(mesh.material_ids).toContain(mat);
        }

        mesh.free();
      });

      it('should create cube sphere with default material (6 faces)', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: 'Default',
        });

        // Cube sphere has 6 faces, each with unique default material
        expect(mesh.geometry_count).toBe(6);

        mesh.free();
      });

      it('should handle cyclic materials for cube sphere', () => {
        const materials = ['mat_x', 'mat_y', 'mat_z'];
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Array: materials },
        });

        // 6 faces with 3 materials = 3 unique geometries
        expect(mesh.geometry_count).toBe(3);

        mesh.free();
      });

      it('should have correct triangle count based on segments', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // Cube sphere: 6 faces * segments^2 * 2 triangles per quad
        // = 6 * 4 * 4 * 2 = 192 triangles
        expect(geo.triangle_count).toBe(6 * 4 * 4 * 2);

        mesh.free();
      });
    });

    describe('mesh data structure', () => {
      it('should provide valid vertex data for Slices sphere', () => {
        const mesh = create_sphere({
          geometry: { Slices: { slices: 8, segments: 16 } },
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

      it('should provide valid vertex data for Cube sphere', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices).toBeInstanceOf(Float32Array);
        expect(geo.normals).toBeInstanceOf(Float32Array);
        expect(geo.uvs).toBeInstanceOf(Float32Array);
        expect(geo.indices).toBeInstanceOf(Uint32Array);

        // Consistency checks
        expect(geo.vertices.length).toBe(geo.normals.length);
        expect(geo.uvs.length * 3).toBe(geo.vertices.length * 2);

        mesh.free();
      });
    });

    describe('get_geometry methods', () => {
      it('should return undefined for out-of-bounds index', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Uniform: 'default' },
        });

        expect(mesh.get_geometry(999)).toBeUndefined();

        mesh.free();
      });

      it('should get geometry by material ID', () => {
        const mesh = create_sphere({
          geometry: { Cube: { segments: 4 } },
          material: { Array: ['mat_a', 'mat_b', 'mat_a', 'mat_b', 'mat_a', 'mat_b'] },
        });

        const geoA = mesh.get_geometry_by_material('mat_a');
        const geoB = mesh.get_geometry_by_material('mat_b');
        const geoC = mesh.get_geometry_by_material('nonexistent');

        expect(geoA).toBeDefined();
        expect(geoB).toBeDefined();
        expect(geoC).toBeUndefined();

        mesh.free();
      });
    });

    describe('error handling', () => {
      it('should throw error for invalid sphere type', () => {
        expect(() => {
          create_sphere({
            geometry: { InvalidType: { param: 10 } },
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for invalid model structure', () => {
        expect(() => {
          create_sphere({
            invalid: 'structure',
          });
        }).toThrow();
      });

      it('should throw error for null input', () => {
        expect(() => {
          create_sphere(null);
        }).toThrow();
      });

      it('should throw error for missing geometry parameters', () => {
        expect(() => {
          create_sphere({
            geometry: { Slices: {} },
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });
    });
  });
});
