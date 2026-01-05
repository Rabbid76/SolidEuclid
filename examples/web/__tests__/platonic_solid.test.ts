import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import {
  initSync,
  create_platonic_solid,
  get_platonic_solid_types,
} from '@solid-euclid';

describe('Platonic Solid WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('get_platonic_solid_types', () => {
    it('should return an array of all five Platonic solid types', () => {
      const types = get_platonic_solid_types();
      expect(types).toBeInstanceOf(Array);
      expect(types).toHaveLength(5);
      expect(types).toContain('Tetrahedron');
      expect(types).toContain('Hexahedron');
      expect(types).toContain('Octahedron');
      expect(types).toContain('Dodecahedron');
      expect(types).toContain('Icosahedron');
    });
  });

  describe('create_platonic_solid', () => {
    describe('with Uniform material', () => {
      it.each([
        ['Tetrahedron', 1],
        ['Hexahedron', 1],
        ['Octahedron', 1],
        ['Dodecahedron', 1],
        ['Icosahedron', 1],
      ])('should create %s with single geometry', (geometry, expectedCount) => {
        const mesh = create_platonic_solid({
          geometry,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(expectedCount);
        expect(mesh.material_ids).toContain('default');

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);
        expect(geo!.triangle_count).toBeGreaterThan(0);

        mesh.free();
      });
    });

    describe('with Array material', () => {
      it('should create Hexahedron with 6 different materials', () => {
        const materials = ['mat0', 'mat1', 'mat2', 'mat3', 'mat4', 'mat5'];
        const mesh = create_platonic_solid({
          geometry: 'Hexahedron',
          material: { Array: materials },
        });

        expect(mesh.geometry_count).toBe(6);
        expect(mesh.material_ids).toHaveLength(6);

        // Each material should be present
        for (const mat of materials) {
          expect(mesh.material_ids).toContain(mat);
        }

        mesh.free();
      });

      it('should create Tetrahedron with cyclic materials when fewer materials than faces', () => {
        const materials = ['mat_a', 'mat_b'];
        const mesh = create_platonic_solid({
          geometry: 'Tetrahedron',
          material: { Array: materials },
        });

        // Tetrahedron has 4 faces, 2 materials used cyclically = 2 unique geometries
        expect(mesh.geometry_count).toBe(2);

        mesh.free();
      });

      it('should deduplicate identical materials', () => {
        const materials = ['same', 'same', 'same', 'same', 'same', 'same'];
        const mesh = create_platonic_solid({
          geometry: 'Hexahedron',
          material: { Array: materials },
        });

        // All faces use the same material = 1 geometry
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.material_ids).toContain('same');

        mesh.free();
      });
    });

    describe('with Default material', () => {
      it('should create separate geometry for each face', () => {
        const mesh = create_platonic_solid({
          geometry: 'Tetrahedron',
          material: 'Default',
        });

        // Tetrahedron has 4 faces, each with unique default material
        expect(mesh.geometry_count).toBe(4);

        mesh.free();
      });
    });

    describe('with Map material', () => {
      it('should assign materials based on face map', () => {
        // JavaScript Map is required for HashMap<i32, String> as JS object keys are always strings
        const faceMap = new Map<number, string>();
        faceMap.set(0, 'special');
        faceMap.set(2, 'special');

        const mesh = create_platonic_solid({
          geometry: 'Tetrahedron',
          material: {
            Map: {
              default: 'base',
              face_map: faceMap,
            },
          },
        });

        // Should have 2 unique materials: "base" and "special"
        expect(mesh.geometry_count).toBe(2);
        expect(mesh.material_ids).toContain('base');
        expect(mesh.material_ids).toContain('special');

        mesh.free();
      });
    });

    describe('mesh data structure', () => {
      it('should provide valid vertex data', () => {
        const mesh = create_platonic_solid({
          geometry: 'Hexahedron',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // Check that typed arrays are returned
        expect(geo.vertices).toBeInstanceOf(Float32Array);
        expect(geo.normals).toBeInstanceOf(Float32Array);
        expect(geo.uvs).toBeInstanceOf(Float32Array);
        expect(geo.indices).toBeInstanceOf(Uint32Array);

        // Check data is not empty
        expect(geo.vertices.length).toBeGreaterThan(0);
        expect(geo.normals.length).toBeGreaterThan(0);
        expect(geo.uvs.length).toBeGreaterThan(0);
        expect(geo.indices.length).toBeGreaterThan(0);

        // Check consistency: vertices and normals should have same length
        expect(geo.vertices.length).toBe(geo.normals.length);

        // UVs should have 2/3 the length of vertices (2 vs 3 components)
        expect(geo.uvs.length * 3).toBe(geo.vertices.length * 2);

        mesh.free();
      });

      it('should provide vertex_count and triangle_count getters', () => {
        const mesh = create_platonic_solid({
          geometry: 'Octahedron',
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        expect(geo.vertex_count).toBe(geo.vertices.length / 3);
        expect(geo.triangle_count).toBe(geo.indices.length / 3);

        mesh.free();
      });
    });

    describe('get_geometry methods', () => {
      it('should return undefined for out-of-bounds index', () => {
        const mesh = create_platonic_solid({
          geometry: 'Tetrahedron',
          material: { Uniform: 'default' },
        });

        expect(mesh.get_geometry(999)).toBeUndefined();

        mesh.free();
      });

      it('should get geometry by material ID', () => {
        const mesh = create_platonic_solid({
          geometry: 'Hexahedron',
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
      it('should throw error for invalid geometry type', () => {
        expect(() => {
          create_platonic_solid({
            geometry: 'InvalidSolid',
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for invalid model structure', () => {
        expect(() => {
          create_platonic_solid({
            invalid: 'structure',
          });
        }).toThrow();
      });

      it('should throw error for null input', () => {
        expect(() => {
          create_platonic_solid(null);
        }).toThrow();
      });
    });
  });
});
