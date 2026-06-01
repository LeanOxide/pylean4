//! Thread-safe wrapper for Lean objects exposed to Python.
//!
//! `LeanObject` is the fundamental Python-visible type. It holds a
//! `LeanUnbound<LeanAny>` which is MT-safe (atomic reference counting)
//! and can be freely shared across Python threads.

use leo3::instance::LeanAny;
use leo3::LeanUnbound;
use pyo3::prelude::*;

/// Runtime type tag for Lean objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeanTypeTag {
    Nat,
    Int,
    String,
    Array,
    List,
    Closure,
    Expr,
    Name,
    Level,
    Environment,
    Other,
}

impl std::fmt::Display for LeanTypeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nat => write!(f, "Nat"),
            Self::Int => write!(f, "Int"),
            Self::String => write!(f, "String"),
            Self::Array => write!(f, "Array"),
            Self::List => write!(f, "List"),
            Self::Closure => write!(f, "Closure"),
            Self::Expr => write!(f, "Expr"),
            Self::Name => write!(f, "Name"),
            Self::Level => write!(f, "Level"),
            Self::Environment => write!(f, "Environment"),
            Self::Other => write!(f, "Object"),
        }
    }
}

/// A Python-visible handle to a Lean4 object.
///
/// This is the core type that bridges Python and Lean4. It holds a
/// thread-safe reference to a Lean object with automatic reference
/// counting on both sides (Python RC for the wrapper, Lean RC for
/// the inner object).
///
/// # Thread Safety
///
/// `LeanObject` is safe to share across Python threads. The inner
/// Lean object is marked as multi-threaded (atomic RC) upon creation.
#[pyclass(name = "LeanObject")]
#[derive(Clone)]
pub struct LeanObject {
    pub inner: LeanUnbound<LeanAny>,
    pub type_tag: LeanTypeTag,
}

impl LeanObject {
    /// Create a new LeanObject from an unbound Lean value.
    pub fn new(inner: LeanUnbound<LeanAny>, type_tag: LeanTypeTag) -> Self {
        Self { inner, type_tag }
    }
}

#[pymethods]
impl LeanObject {
    /// Get the type tag of this object.
    #[getter]
    fn type_name(&self) -> &str {
        match self.type_tag {
            LeanTypeTag::Nat => "Nat",
            LeanTypeTag::Int => "Int",
            LeanTypeTag::String => "String",
            LeanTypeTag::Array => "Array",
            LeanTypeTag::List => "List",
            LeanTypeTag::Closure => "Closure",
            LeanTypeTag::Expr => "Expr",
            LeanTypeTag::Name => "Name",
            LeanTypeTag::Level => "Level",
            LeanTypeTag::Environment => "Environment",
            LeanTypeTag::Other => "Object",
        }
    }

    /// Check if this is a scalar (unboxed) value.
    fn is_scalar(&self) -> bool {
        leo3::with_lean(|lean| {
            let bound = self.inner.bind(lean);
            unsafe { leo3::ffi::inline::lean_is_scalar(bound.as_ptr()) }
        })
    }

    fn __repr__(&self) -> String {
        format!("LeanObject(type={})", self.type_tag)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
