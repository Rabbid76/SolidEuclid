import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, create_transform } from '@solid-euclid';
import type { WasmTriangularGeometry } from '@solid-euclid';

// JSON shapes matching the serde representation of Transform<AnyModel>
type TransformationJson =
  | { Translation: [number, number, number] }
  | { UniformScale: number }
  | { ScaleXYZ: [number, number, number] }
  | { RotationX: number }
  | { RotationY: number }
  | { RotationZ: number }
  | { Quaternion: [number, number, number, number] }
  | { Matrix: number[] };

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
  | { Transform: { transformation: TransformationJson; model: AnyModelJson } };

type TransformJson = { transformation: TransformationJson; model: AnyModelJson };

// The hexahedron spans ±1/sqrt(3) on every axis
const L = 1 / Math.sqrt(3);

function hexahedronModel(): AnyModelJson {
  return {
    PlatonicSolid: {
      geometry: 'Hexahedron',
      material: { Uniform: 'default' },
    },
  };
}

function transformed(model: AnyModelJson, transformation: TransformationJson): TransformJson {
  return { transformation, model };
}

function bounds(vertices: Float32Array): { min: number[]; max: number[] } {
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < vertices.length; i += 3) {
    for (let axis = 0; axis < 3; axis++) {
      min[axis] = Math.min(min[axis]!, vertices[i + axis]!);
      max[axis] = Math.max(max[axis]!, vertices[i + axis]!);
    }
  }
  return { min, max };
}

// Asserts that every triangle's geometric normal points away from the origin,
// i.e. the winding is outward-facing for a model centered at the origin.
function expectOutwardWinding(geo: WasmTriangularGeometry) {
  const vertices = geo.vertices;
  const indices = geo.indices;
  const position = (i: number): [number, number, number] => [
    vertices[i * 3]!,
    vertices[i * 3 + 1]!,
    vertices[i * 3 + 2]!,
  ];
  for (let t = 0; t < indices.length; t += 3) {
    const [ax, ay, az] = position(indices[t]!);
    const [bx, by, bz] = position(indices[t + 1]!);
    const [cx, cy, cz] = position(indices[t + 2]!);
    const ux = bx - ax;
    const uy = by - ay;
    const uz = bz - az;
    const wx = cx - ax;
    const wy = cy - ay;
    const wz = cz - az;
    const nx = uy * wz - uz * wy;
    const ny = uz * wx - ux * wz;
    const nz = ux * wy - uy * wx;
    const centroidX = (ax + bx + cx) / 3;
    const centroidY = (ay + by + cy) / 3;
    const centroidZ = (az + bz + cz) / 3;
    const dot = nx * centroidX + ny * centroidY + nz * centroidZ;
    expect(dot).toBeGreaterThan(0);
  }
}

describe('Transform WebAssembly API', () => {
  beforeAll(() => {
    const wasmPath = resolve(__dirname, '../../../pkg/solid_euclid_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    initSync({ module: wasmBuffer });
  });

  describe('create_transform', () => {
    describe('basic transform creation', () => {
      it('should create a translated hexahedron with uniform material', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { Translation: [1, 2, 3] }));

        expect(mesh).toBeDefined();
        expect(mesh.geometry_count).toBe(1);
        expect(mesh.material_ids).toContain('default');

        const geo = mesh.get_geometry(0);
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBe(24); // 6 faces * 4 vertices
        expect(geo!.triangle_count).toBe(12); // 6 faces * 2 triangles

        mesh.free();
      });

      it('should apply the translation to the vertex positions', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { Translation: [1, 2, 3] }));
        const geo = mesh.get_geometry(0)!;
        const { min, max } = bounds(geo.vertices);

        expect(min[0]).toBeCloseTo(1 - L, 4);
        expect(max[0]).toBeCloseTo(1 + L, 4);
        expect(min[1]).toBeCloseTo(2 - L, 4);
        expect(max[1]).toBeCloseTo(2 + L, 4);
        expect(min[2]).toBeCloseTo(3 - L, 4);
        expect(max[2]).toBeCloseTo(3 + L, 4);

        mesh.free();
      });

      it('should apply a uniform scale to the vertex positions', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { UniformScale: 2.0 }));
        const geo = mesh.get_geometry(0)!;
        const { min, max } = bounds(geo.vertices);

        expect(min[0]).toBeCloseTo(-2 * L, 4);
        expect(max[0]).toBeCloseTo(2 * L, 4);
        expect(min[1]).toBeCloseTo(-2 * L, 4);
        expect(max[1]).toBeCloseTo(2 * L, 4);
        expect(min[2]).toBeCloseTo(-2 * L, 4);
        expect(max[2]).toBeCloseTo(2 * L, 4);

        mesh.free();
      });

      it('should apply a non-uniform scale to the vertex positions', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { ScaleXYZ: [2, 3, 4] }));
        const geo = mesh.get_geometry(0)!;
        const { min, max } = bounds(geo.vertices);

        expect(min[0]).toBeCloseTo(-2 * L, 4);
        expect(max[0]).toBeCloseTo(2 * L, 4);
        expect(min[1]).toBeCloseTo(-3 * L, 4);
        expect(max[1]).toBeCloseTo(3 * L, 4);
        expect(min[2]).toBeCloseTo(-4 * L, 4);
        expect(max[2]).toBeCloseTo(4 * L, 4);

        mesh.free();
      });

      it('should apply a rotation around the Z-axis', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { RotationZ: Math.PI / 2 }));
        const geo = mesh.get_geometry(0)!;

        // First vertex of the hexahedron is (-L, -L, L); rotated 90° around Z
        // it becomes (L, -L, L)
        expect(geo.vertices[0]).toBeCloseTo(L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-L, 5);
        expect(geo.vertices[2]).toBeCloseTo(L, 5);

        mesh.free();
      });

      it('should apply a rotation around the X-axis', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { RotationX: Math.PI / 2 }));
        const geo = mesh.get_geometry(0)!;

        // First vertex of the hexahedron is (-L, -L, L); rotated 90° around X
        // it becomes (-L, -L, -L)
        expect(geo.vertices[0]).toBeCloseTo(-L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-L, 5);
        expect(geo.vertices[2]).toBeCloseTo(-L, 5);

        mesh.free();
      });

      it('should apply a rotation around the Y-axis', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { RotationY: Math.PI / 2 }));
        const geo = mesh.get_geometry(0)!;

        // First vertex of the hexahedron is (-L, -L, L); rotated 90° around Y
        // it becomes (L, -L, L)
        expect(geo.vertices[0]).toBeCloseTo(L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-L, 5);
        expect(geo.vertices[2]).toBeCloseTo(L, 5);

        mesh.free();
      });

      it('should apply a quaternion rotation', () => {
        // 90° rotation around Z as a quaternion (x, y, z, w)
        const mesh = create_transform(
          transformed(hexahedronModel(), {
            Quaternion: [0, 0, Math.SQRT1_2, Math.SQRT1_2],
          })
        );
        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices[0]).toBeCloseTo(L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-L, 5);
        expect(geo.vertices[2]).toBeCloseTo(L, 5);

        mesh.free();
      });

      it('should apply a matrix transformation', () => {
        // Uniform scale by 2 as a flat column-major 4x4 matrix
        const mesh = create_transform(
          transformed(hexahedronModel(), {
            Matrix: [2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1],
          })
        );
        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices[0]).toBeCloseTo(-2 * L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-2 * L, 5);
        expect(geo.vertices[2]).toBeCloseTo(2 * L, 5);

        mesh.free();
      });
    });

    describe('nested transforms', () => {
      it('should apply nested transforms inside-out', () => {
        // Scale X by 2 first, then translate Z by 5
        const mesh = create_transform({
          transformation: { Translation: [0, 0, 5] },
          model: {
            Transform: {
              transformation: { ScaleXYZ: [2, 1, 1] },
              model: hexahedronModel(),
            },
          },
        });

        expect(mesh.geometry_count).toBe(1);
        const geo = mesh.get_geometry(0)!;
        expect(geo.vertex_count).toBe(24);
        expect(geo.triangle_count).toBe(12);

        const { min, max } = bounds(geo.vertices);
        expect(min[0]).toBeCloseTo(-2 * L, 4);
        expect(max[0]).toBeCloseTo(2 * L, 4);
        expect(min[2]).toBeCloseTo(5 - L, 4);
        expect(max[2]).toBeCloseTo(5 + L, 4);

        mesh.free();
      });

      it('should apply deeply nested transforms', () => {
        // Scale, rotate, translate — three levels of nesting
        const mesh = create_transform({
          transformation: { Translation: [1, 0, 0] },
          model: {
            Transform: {
              transformation: { RotationZ: Math.PI / 2 },
              model: {
                Transform: {
                  transformation: { UniformScale: 2 },
                  model: hexahedronModel(),
                },
              },
            },
          },
        });

        expect(mesh.geometry_count).toBe(1);
        const geo = mesh.get_geometry(0)!;
        expect(geo.vertex_count).toBe(24);

        // First vertex: (-L, -L, L) scaled by 2 -> (-2L, -2L, 2L), rotated 90°
        // around Z -> (2L, -2L, 2L), translated -> (1 + 2L, -2L, 2L)
        expect(geo.vertices[0]).toBeCloseTo(1 + 2 * L, 5);
        expect(geo.vertices[1]).toBeCloseTo(-2 * L, 5);
        expect(geo.vertices[2]).toBeCloseTo(2 * L, 5);

        mesh.free();
      });
    });

    describe('different base models', () => {
      it('should transform a torus', () => {
        const mesh = create_transform(
          transformed(
            {
              Torus: {
                major_radius: 2.0,
                minor_radius: 0.5,
                major_segments: 24,
                minor_segments: 12,
                material: { Uniform: 'default' },
              },
            },
            { Translation: [0, 0, 3] }
          )
        );

        expect(mesh.geometry_count).toBe(1);
        const geo = mesh.get_geometry(0)!;
        expect(geo.vertex_count).toBeGreaterThan(0);
        expect(geo.triangle_count).toBeGreaterThan(0);

        // The torus lies in the XY-plane; translated by 3 along Z
        const { min, max } = bounds(geo.vertices);
        expect(min[2]).toBeCloseTo(3 - 0.5, 4);
        expect(max[2]).toBeCloseTo(3 + 0.5, 4);

        mesh.free();
      });

      it('should transform a platonic solid other than the hexahedron', () => {
        const mesh = create_transform(
          transformed(
            {
              PlatonicSolid: {
                geometry: 'Tetrahedron',
                material: { Uniform: 'default' },
              },
            },
            { UniformScale: 2 }
          )
        );

        expect(mesh.geometry_count).toBe(1);
        const geo = mesh.get_geometry(0)!;
        expect(geo.vertex_count).toBe(12); // 4 faces * 3 vertices
        expect(geo.triangle_count).toBe(4);

        mesh.free();
      });
    });

    describe('winding', () => {
      it('should keep outward winding for a positive scale', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { UniformScale: 2 }));
        const geo = mesh.get_geometry(0)!;
        expectOutwardWinding(geo);
        mesh.free();
      });

      it('should flip winding for a mirroring transformation', () => {
        // A negative scale mirrors the geometry; the winding must be reversed
        // so faces keep their outward orientation
        const mesh = create_transform(transformed(hexahedronModel(), { UniformScale: -1 }));
        const geo = mesh.get_geometry(0)!;
        expectOutwardWinding(geo);
        mesh.free();
      });

      it('should flip winding for a non-uniform mirroring scale', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { ScaleXYZ: [-1, 2, 3] }));
        const geo = mesh.get_geometry(0)!;
        expectOutwardWinding(geo);
        mesh.free();
      });
    });

    describe('mesh data', () => {
      it('should return typed arrays', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { Translation: [1, 0, 0] }));
        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices).toBeInstanceOf(Float32Array);
        expect(geo.normals).toBeInstanceOf(Float32Array);
        expect(geo.uvs).toBeInstanceOf(Float32Array);
        expect(geo.indices).toBeInstanceOf(Uint32Array);

        mesh.free();
      });

      it('should have consistent geometry data sizes', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { UniformScale: 2 }));
        const geo = mesh.get_geometry(0)!;

        expect(geo.vertices.length % 3).toBe(0);
        expect(geo.vertices.length).toBe(geo.normals.length);
        expect(geo.uvs.length).toBe((geo.vertices.length / 3) * 2);
        expect(geo.indices.length % 3).toBe(0);

        mesh.free();
      });

      it('should have normalized normals after a non-uniform scale', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { ScaleXYZ: [2, 3, 4] }));
        const geo = mesh.get_geometry(0)!;

        for (let i = 0; i < geo.vertex_count; i++) {
          const nx = geo.normals[i * 3]!;
          const ny = geo.normals[i * 3 + 1]!;
          const nz = geo.normals[i * 3 + 2]!;
          const length = Math.sqrt(nx * nx + ny * ny + nz * nz);
          expect(length).toBeCloseTo(1.0, 4);
        }

        mesh.free();
      });

      it('should have indices within bounds', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { RotationY: 0.7 }));
        const geo = mesh.get_geometry(0)!;

        for (const index of geo.indices) {
          expect(index).toBeLessThan(geo.vertex_count);
        }

        mesh.free();
      });

      it('should find the geometry by material id', () => {
        const mesh = create_transform(transformed(hexahedronModel(), { Translation: [1, 0, 0] }));

        const geo = mesh.get_geometry_by_material('default');
        expect(geo).toBeDefined();
        expect(geo!.vertex_count).toBe(24);
        expect(mesh.get_geometry_by_material('nonexistent')).toBeUndefined();

        mesh.free();
      });
    });

    describe('error handling', () => {
      it('should throw for a NaN scale', () => {
        expect(() =>
          create_transform(transformed(hexahedronModel(), { UniformScale: NaN }))
        ).toThrow();
      });

      it('should throw for an infinite scale', () => {
        expect(() =>
          create_transform(transformed(hexahedronModel(), { UniformScale: Infinity }))
        ).toThrow();
      });

      it('should throw for a NaN translation component', () => {
        expect(() =>
          create_transform(transformed(hexahedronModel(), { Translation: [1, NaN, 3] }))
        ).toThrow();
      });

      it('should throw for a NaN rotation', () => {
        expect(() =>
          create_transform(transformed(hexahedronModel(), { RotationX: NaN }))
        ).toThrow();
      });

      it('should throw for an unknown transformation variant', () => {
        expect(() =>
          create_transform(
            transformed(hexahedronModel(), {
              Bogus: 1,
            } as unknown as TransformationJson)
          )
        ).toThrow();
      });

      it('should throw for a missing transformation', () => {
        expect(() =>
          create_transform({ model: hexahedronModel() } as unknown as TransformJson)
        ).toThrow();
      });

      it('should throw for an unknown model variant', () => {
        expect(() =>
          create_transform({
            transformation: { Translation: [0, 0, 0] },
            model: { Bogus: {} } as unknown as AnyModelJson,
          })
        ).toThrow();
      });
    });
  });
});
