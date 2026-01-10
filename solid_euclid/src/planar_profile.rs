use glam::DVec2;
use serde::{Deserialize, Serialize};

/// Represents the type of corner treatment at a path vertex.
///
/// This enum defines how corners are handled when creating profiles
/// for extrusion or other geometric operations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Corner {
    /// A sharp edge corner with no modification.
    Edge,
    /// A smooth edge that blends tangentially with adjacent segments.
    SmoothEdge,
    /// A chamfered corner that cuts off the corner at a given distance.
    Chamfer {
        /// The distance from the corner vertex to the chamfer cut.
        distance: f64,
    },
    /// A filleted corner with a smooth rounded transition.
    Fillet {
        /// The radius of the fillet arc.
        radius: f64,
    },
}

impl Default for Corner {
    fn default() -> Self {
        Self::Edge
    }
}

/// Represents a path command for defining 2D profile geometry.
///
/// Each command specifies a way to extend the path from the current
/// position to a new point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathCommand {
    /// Move to a new position without drawing.
    Move {
        /// The target point to move to.
        point: DVec2,
    },
    /// Draw a straight line to a point.
    Line {
        /// The end point of the line.
        point: DVec2,
    },
    /// Draw an elliptical arc to a point.
    Ellipse {
        /// The end point of the elliptical arc.
        point: DVec2,
        /// The center point of the ellipse.
        center: DVec2,
    },
    /// Draw a NURBS curve to a point.
    Nurbs {
        /// The end point of the NURBS curve.
        point: DVec2,
        /// The control points defining the curve shape.
        control_points: Vec<DVec2>,
    },
}

/// Represents a single command in a planar profile with corner treatment.
///
/// Combines a path command with a corner specification to define
/// how the path segment ends and transitions to the next segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileCommand {
    /// The path command defining the geometry.
    pub path: PathCommand,
    /// The corner treatment at the end of this path segment.
    pub corner: Corner,
}

impl ProfileCommand {
    /// Creates a new `ProfileCommand` with the given path and corner.
    pub fn new(path: PathCommand, corner: Corner) -> Self {
        Self { path, corner }
    }

    /// Creates a new `ProfileCommand` with the given path and a default edge corner.
    pub fn with_edge(path: PathCommand) -> Self {
        Self {
            path,
            corner: Corner::Edge,
        }
    }
}

/// Represents a 2D planar profile defined by a sequence of path commands.
///
/// A planar profile is used as the basis for extrusion, revolution,
/// or other operations that generate 3D geometry from 2D shapes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlanarProfile {
    /// The sequence of profile commands that define the profile shape.
    pub commands: Vec<ProfileCommand>,
}

impl PlanarProfile {
    /// Creates a new empty `PlanarProfile`.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Creates a new `PlanarProfile` with the given commands.
    pub fn with_commands(commands: Vec<ProfileCommand>) -> Self {
        Self { commands }
    }

    /// Adds a profile command to the profile.
    pub fn push(&mut self, command: ProfileCommand) {
        self.commands.push(command);
    }

    /// Returns the number of commands in the profile.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns true if the profile has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Corner tests

    #[test]
    fn test_corner_edge() {
        let corner = Corner::Edge;
        assert_eq!(corner, Corner::Edge);
    }

    #[test]
    fn test_corner_smooth_edge() {
        let corner = Corner::SmoothEdge;
        assert_eq!(corner, Corner::SmoothEdge);
    }

    #[test]
    fn test_corner_chamfer() {
        let corner = Corner::Chamfer { distance: 2.5 };
        if let Corner::Chamfer { distance } = corner {
            assert_eq!(distance, 2.5);
        } else {
            panic!("Expected Corner::Chamfer");
        }
    }

    #[test]
    fn test_corner_fillet() {
        let corner = Corner::Fillet { radius: 1.5 };
        if let Corner::Fillet { radius } = corner {
            assert_eq!(radius, 1.5);
        } else {
            panic!("Expected Corner::Fillet");
        }
    }

    #[test]
    fn test_corner_default() {
        let corner = Corner::default();
        assert_eq!(corner, Corner::Edge);
    }

    // PathCommand tests

    #[test]
    fn test_path_command_move() {
        let point = DVec2::new(1.0, 2.0);
        let cmd = PathCommand::Move { point };
        if let PathCommand::Move { point: p } = cmd {
            assert_eq!(p, DVec2::new(1.0, 2.0));
        } else {
            panic!("Expected PathCommand::Move");
        }
    }

    #[test]
    fn test_path_command_line() {
        let point = DVec2::new(3.0, 4.0);
        let cmd = PathCommand::Line { point };
        if let PathCommand::Line { point: p } = cmd {
            assert_eq!(p, DVec2::new(3.0, 4.0));
        } else {
            panic!("Expected PathCommand::Line");
        }
    }

    #[test]
    fn test_path_command_ellipse() {
        let point = DVec2::new(5.0, 6.0);
        let center = DVec2::new(2.5, 3.0);
        let cmd = PathCommand::Ellipse { point, center };
        if let PathCommand::Ellipse {
            point: p,
            center: c,
        } = cmd
        {
            assert_eq!(p, DVec2::new(5.0, 6.0));
            assert_eq!(c, DVec2::new(2.5, 3.0));
        } else {
            panic!("Expected PathCommand::Ellipse");
        }
    }

    #[test]
    fn test_path_command_nurbs() {
        let point = DVec2::new(10.0, 10.0);
        let control_points = vec![
            DVec2::new(2.0, 4.0),
            DVec2::new(5.0, 8.0),
            DVec2::new(8.0, 6.0),
        ];
        let cmd = PathCommand::Nurbs {
            point,
            control_points: control_points.clone(),
        };
        if let PathCommand::Nurbs {
            point: p,
            control_points: cp,
        } = cmd
        {
            assert_eq!(p, DVec2::new(10.0, 10.0));
            assert_eq!(cp.len(), 3);
            assert_eq!(cp[0], DVec2::new(2.0, 4.0));
            assert_eq!(cp[1], DVec2::new(5.0, 8.0));
            assert_eq!(cp[2], DVec2::new(8.0, 6.0));
        } else {
            panic!("Expected PathCommand::Nurbs");
        }
    }

    // ProfileCommand tests

    #[test]
    fn test_profile_command_new() {
        let path = PathCommand::Line {
            point: DVec2::new(1.0, 0.0),
        };
        let corner = Corner::Chamfer { distance: 0.5 };
        let cmd = ProfileCommand::new(path.clone(), corner);
        assert_eq!(cmd.path, path);
        assert_eq!(cmd.corner, Corner::Chamfer { distance: 0.5 });
    }

    #[test]
    fn test_profile_command_with_edge() {
        let path = PathCommand::Line {
            point: DVec2::new(1.0, 0.0),
        };
        let cmd = ProfileCommand::with_edge(path.clone());
        assert_eq!(cmd.path, path);
        assert_eq!(cmd.corner, Corner::Edge);
    }

    // PlanarProfile tests

    #[test]
    fn test_planar_profile_new() {
        let profile = PlanarProfile::new();
        assert!(profile.is_empty());
        assert_eq!(profile.len(), 0);
    }

    #[test]
    fn test_planar_profile_default() {
        let profile = PlanarProfile::default();
        assert!(profile.is_empty());
    }

    #[test]
    fn test_planar_profile_with_commands() {
        let commands = vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ];
        let profile = PlanarProfile::with_commands(commands);
        assert_eq!(profile.len(), 3);
        assert!(!profile.is_empty());
    }

    #[test]
    fn test_planar_profile_push() {
        let mut profile = PlanarProfile::new();
        assert!(profile.is_empty());

        profile.push(ProfileCommand::with_edge(PathCommand::Move {
            point: DVec2::new(0.0, 0.0),
        }));
        assert_eq!(profile.len(), 1);

        profile.push(ProfileCommand::new(
            PathCommand::Line {
                point: DVec2::new(5.0, 5.0),
            },
            Corner::Chamfer { distance: 0.25 },
        ));
        assert_eq!(profile.len(), 2);
    }

    #[test]
    fn test_planar_profile_clone() {
        let commands = vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ];
        let profile = PlanarProfile::with_commands(commands);
        let cloned = profile.clone();
        assert_eq!(profile, cloned);
    }

    #[test]
    fn test_corner_equality() {
        assert_eq!(Corner::Edge, Corner::Edge);
        assert_eq!(Corner::SmoothEdge, Corner::SmoothEdge);
        assert_eq!(
            Corner::Chamfer { distance: 1.0 },
            Corner::Chamfer { distance: 1.0 }
        );
        assert_eq!(
            Corner::Fillet { radius: 2.0 },
            Corner::Fillet { radius: 2.0 }
        );
        assert_ne!(Corner::Edge, Corner::SmoothEdge);
        assert_ne!(Corner::Edge, Corner::Chamfer { distance: 1.0 });
        assert_ne!(Corner::SmoothEdge, Corner::Chamfer { distance: 1.0 });
        assert_ne!(
            Corner::Chamfer { distance: 1.0 },
            Corner::Fillet { radius: 1.0 }
        );
    }

    // Serialization tests

    #[test]
    fn test_corner_serialize_deserialize() {
        let edge = Corner::Edge;
        let json = serde_json::to_string(&edge).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, deserialized);

        let smooth_edge = Corner::SmoothEdge;
        let json = serde_json::to_string(&smooth_edge).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(smooth_edge, deserialized);

        let chamfer = Corner::Chamfer { distance: 2.5 };
        let json = serde_json::to_string(&chamfer).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(chamfer, deserialized);

        let fillet = Corner::Fillet { radius: 1.5 };
        let json = serde_json::to_string(&fillet).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(fillet, deserialized);
    }

    #[test]
    fn test_path_command_serialize_deserialize() {
        let move_cmd = PathCommand::Move {
            point: DVec2::new(1.0, 2.0),
        };
        let json = serde_json::to_string(&move_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(move_cmd, deserialized);

        let line_cmd = PathCommand::Line {
            point: DVec2::new(3.0, 4.0),
        };
        let json = serde_json::to_string(&line_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(line_cmd, deserialized);

        let ellipse_cmd = PathCommand::Ellipse {
            point: DVec2::new(5.0, 6.0),
            center: DVec2::new(2.5, 3.0),
        };
        let json = serde_json::to_string(&ellipse_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(ellipse_cmd, deserialized);

        let nurbs_cmd = PathCommand::Nurbs {
            point: DVec2::new(10.0, 10.0),
            control_points: vec![DVec2::new(2.0, 4.0), DVec2::new(5.0, 8.0)],
        };
        let json = serde_json::to_string(&nurbs_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(nurbs_cmd, deserialized);
    }

    #[test]
    fn test_profile_command_serialize_deserialize() {
        let cmd = ProfileCommand::new(
            PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            },
            Corner::Chamfer { distance: 0.5 },
        );
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ProfileCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_planar_profile_serialize_deserialize() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Chamfer { distance: 0.2 },
            ),
            ProfileCommand::with_edge(PathCommand::Ellipse {
                point: DVec2::new(0.0, 1.0),
                center: DVec2::new(0.5, 1.0),
            }),
        ]);

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: PlanarProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
    }

    #[test]
    fn test_planar_profile_serialize_empty() {
        let profile = PlanarProfile::new();
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: PlanarProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_planar_profile_json_format() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(1.0, 2.0),
            })]);

        let json = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json.contains("commands"));
        assert!(json.contains("Move"));
        assert!(json.contains("Edge"));
    }
}
