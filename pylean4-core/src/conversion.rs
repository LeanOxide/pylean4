//! Python ↔ Lean type conversion utilities.

use crate::object::{LeanObject, LeanTypeTag};
use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;
use pyo3::prelude::*;

/// Convert a Python int to a LeanObject (Nat).
#[allow(dead_code)]
pub fn py_int_to_lean(value: usize) -> PyResult<LeanObject> {
    let result = leo3::with_lean(|lean| {
        let nat = LeanNat::from_usize(lean, value)?;
        let any: LeanBound<'_, LeanAny> = nat.cast();
        Ok::<LeanUnbound<LeanAny>, leo3::LeanError>(any.unbind_mt())
    });

    match result {
        Ok(unbound) => Ok(LeanObject::new(unbound, LeanTypeTag::Nat)),
        Err(e) => Err(crate::error::lean_to_py_err(e)),
    }
}

/// Convert a Python str to a LeanObject (String).
#[allow(dead_code)]
pub fn py_str_to_lean(value: &str) -> PyResult<LeanObject> {
    let result = leo3::with_lean(|lean| {
        let s = LeanString::mk(lean, value)?;
        let any: LeanBound<'_, LeanAny> = s.cast();
        Ok::<LeanUnbound<LeanAny>, leo3::LeanError>(any.unbind_mt())
    });

    match result {
        Ok(unbound) => Ok(LeanObject::new(unbound, LeanTypeTag::String)),
        Err(e) => Err(crate::error::lean_to_py_err(e)),
    }
}

/// Convert a LeanObject (Nat) back to a Python int.
#[allow(dead_code)]
pub fn lean_nat_to_py(obj: &LeanObject) -> PyResult<usize> {
    if obj.type_tag != LeanTypeTag::Nat {
        return Err(crate::error::type_mismatch("Nat", &obj.type_tag.to_string()));
    }

    let result = leo3::with_lean(|lean| {
        let bound = obj.inner.bind(lean);
        LeanNat::to_usize(&bound.cast())
    });

    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(crate::error::lean_to_py_err(e)),
    }
}

/// Convert a LeanObject (String) back to a Python str.
#[allow(dead_code)]
pub fn lean_string_to_py(obj: &LeanObject) -> PyResult<String> {
    if obj.type_tag != LeanTypeTag::String {
        return Err(crate::error::type_mismatch(
            "String",
            &obj.type_tag.to_string(),
        ));
    }

    let result = leo3::with_lean(|lean| {
        let bound = obj.inner.bind(lean);
        let s = LeanString::cstr(&bound.cast())?;
        Ok::<String, leo3::LeanError>(s.to_owned())
    });

    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(crate::error::lean_to_py_err(e)),
    }
}
