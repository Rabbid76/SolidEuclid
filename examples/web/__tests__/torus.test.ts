import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_torus } from '@solid-euclid';

describe('Torus WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('create_torus', () => {
    describe('basic torus creation', () => {
      it('should create a torus with uniform material', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 36,
          minor_segments: 18,
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

      it('should create a torus with default material', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 12,
          minor_segments: 8,
          material: 'Default',
        });

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should handle different segment counts', () => {
        const lowRes = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 8,
          minor_segments: 6,
          material: { Uniform: 'default' },
        });

        const highRes = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 48,
          minor_segments: 24,
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

    describe('torus geometry variations', () => {
      it('should create a thin ring (small minor radius)', () => {
        const mesh = create_torus({
          major_radius: 10.0,
          minor_radius: 0.1,
          major_segments: 48,
          minor_segments: 12,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a fat ring (large minor radius relative to major)', () => {
        const mesh = create_torus({
          major_radius: 1.0,
          minor_radius: 0.9,
          major_segments: 24,
          minor_segments: 24,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBeGreaterThan(0);

        mesh.free();
      });

      it('should create a small torus', () => {
        const mesh = create_torus({
          major_radius: 0.01,
          minor_radius: 0.001,
          major_segments: 12,
          minor_segments: 8,
          material: { Uniform: 'default' },
        });

        expect(mesh).toBeDefined();
        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();

        mesh.free();
      });
    });

    describe('mesh data structure', () => {
      it('should provide valid vertex data', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
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
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // vertex_count should match vertices.length / 3
        expect(geo.vertex_count).toBe(geo.vertices.length / 3);

        mesh.free();
      });

      it('should have correct triangle count', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
          material: { Uniform: 'default' },
        });

        const geo = mesh.get_geometry(0)!;

        // triangle_count should match indices.length / 3
        expect(geo.triangle_count).toBe(geo.indices.length / 3);

        mesh.free();
      });

      it('should have indices within bounds', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
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
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
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
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
          material: { Uniform: 'default' },
        });

        expect(mesh.get_geometry(999)).toBeUndefined();

        mesh.free();
      });

      it('should get geometry by material ID', () => {
        const mesh = create_torus({
          major_radius: 2.0,
          minor_radius: 0.5,
          major_segments: 24,
          minor_segments: 12,
          material: { Uniform: 'my_material' },
        });

        const geo = mesh.get_geometry_by_material('my_material');
        const missing = mesh.get_geometry_by_material('nonexistent');

        expect(geo).toBeDefined();
        expect(missing).toBeUndefined();

        mesh.free();
      });
    });

    describe('error handling', () => {
      it('should throw error for invalid torus structure', () => {
        expect(() => {
          create_torus({
            invalid: 'structure',
          });
        }).toThrow();
      });

      it('should throw error for null input', () => {
        expect(() => {
          create_torus(null);
        }).toThrow();
      });

      it('should throw error for missing major_radius', () => {
        expect(() => {
          create_torus({
            minor_radius: 0.5,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for missing minor_radius', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for missing segments', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            minor_radius: 0.5,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for zero major_radius', () => {
        expect(() => {
          create_torus({
            major_radius: 0.0,
            minor_radius: 0.5,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for negative major_radius', () => {
        expect(() => {
          create_torus({
            major_radius: -1.0,
            minor_radius: 0.5,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for zero minor_radius', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            minor_radius: 0.0,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for negative minor_radius', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            minor_radius: -0.5,
            major_segments: 24,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for zero major_segments', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            minor_radius: 0.5,
            major_segments: 0,
            minor_segments: 12,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });

      it('should throw error for zero minor_segments', () => {
        expect(() => {
          create_torus({
            major_radius: 2.0,
            minor_radius: 0.5,
            major_segments: 24,
            minor_segments: 0,
            material: { Uniform: 'default' },
          });
        }).toThrow();
      });
    });
  });
});
