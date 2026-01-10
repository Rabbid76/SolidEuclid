use std::error::Error;
use std::fmt;

/// Errors that can occur during geometry operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    /// The operation requires at least the specified minimum number of vertices/points.
    InsufficientVertices {
        /// The minimum number of vertices required.
        required: usize,
        /// The actual number of vertices provided.
        provided: usize,
    },
    /// The requested feature or operation is not yet implemented.
    NotYetImplemented {
        /// A description of the feature that is not yet implemented.
        feature: String,
    },
    /// An invalid parameter was provided to the operation.
    InvalidParameter {
        /// The name of the invalid parameter.
        parameter: String,
        /// A message describing why the parameter is invalid.
        message: String,
    },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryError::InsufficientVertices { required, provided } => {
                write!(
                    f,
                    "Insufficient vertices: required at least {}, but only {} provided",
                    required, provided
                )
            }
            GeometryError::NotYetImplemented { feature } => {
                write!(f, "Not yet implemented: {}", feature)
            }
            GeometryError::InvalidParameter { parameter, message } => {
                write!(f, "Invalid parameter '{}': {}", parameter, message)
            }
        }
    }
}

impl Error for GeometryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_vertices_error_display() {
        let error = GeometryError::InsufficientVertices {
            required: 3,
            provided: 2,
        };
        let message = format!("{}", error);
        assert!(message.contains("3"));
        assert!(message.contains("2"));
        assert!(message.contains("Insufficient vertices"));
    }

    #[test]
    fn test_insufficient_vertices_error_debug() {
        let error = GeometryError::InsufficientVertices {
            required: 3,
            provided: 1,
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InsufficientVertices"));
        assert!(debug_str.contains("required"));
        assert!(debug_str.contains("provided"));
    }

    #[test]
    fn test_insufficient_vertices_error_equality() {
        let error1 = GeometryError::InsufficientVertices {
            required: 3,
            provided: 2,
        };
        let error2 = GeometryError::InsufficientVertices {
            required: 3,
            provided: 2,
        };
        let error3 = GeometryError::InsufficientVertices {
            required: 3,
            provided: 1,
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_insufficient_vertices_error_clone() {
        let error = GeometryError::InsufficientVertices {
            required: 3,
            provided: 0,
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_geometry_error_is_error_trait() {
        let error: Box<dyn Error> = Box::new(GeometryError::InsufficientVertices {
            required: 3,
            provided: 2,
        });
        // Verify it implements Error trait by using it as a trait object
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn test_not_yet_implemented_error_display() {
        let error = GeometryError::NotYetImplemented {
            feature: "advanced tessellation".to_string(),
        };
        let message = format!("{}", error);
        assert!(message.contains("Not yet implemented"));
        assert!(message.contains("advanced tessellation"));
    }

    #[test]
    fn test_not_yet_implemented_error_debug() {
        let error = GeometryError::NotYetImplemented {
            feature: "CSG operations".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("NotYetImplemented"));
        assert!(debug_str.contains("feature"));
    }

    #[test]
    fn test_not_yet_implemented_error_equality() {
        let error1 = GeometryError::NotYetImplemented {
            feature: "feature_a".to_string(),
        };
        let error2 = GeometryError::NotYetImplemented {
            feature: "feature_a".to_string(),
        };
        let error3 = GeometryError::NotYetImplemented {
            feature: "feature_b".to_string(),
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_not_yet_implemented_error_clone() {
        let error = GeometryError::NotYetImplemented {
            feature: "some feature".to_string(),
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }
}
