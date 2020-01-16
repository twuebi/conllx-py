use pyo3::basic::CompareOp;
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3::PyObjectProtocol;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::Chars;

use edit_tree::{Apply, EditTree};

#[pyclass]
pub struct PyEditTree {
    inner: edit_tree::EditTree<char>,
}

impl PyEditTree {
    pub fn new(a: Chars, b: Chars) -> Self {
        PyEditTree {
            inner: EditTree::create_tree(&a.collect::<Vec<char>>(), &b.collect::<Vec<char>>()),
        }
    }
}

#[pymethods]
impl PyEditTree {
    fn apply(&self, a: &PyAny) -> PyResult<String> {
        let a = t_from_any::<&str>(a, Some("string"))?;
        match self.inner.apply(&a.chars().collect::<Vec<char>>()) {
            Some(lem) => Ok(lem.iter().collect()),
            None => Err(exceptions::Exception::py_err(format!(
                "Couldnt apply {:?} to {}",
                &self.inner, &a
            ))),
        }
    }

    fn serialize_to_string(&self) -> PyResult<String> {
        Ok(serde_json::to_string(&self.inner).map_err(|_| {
            exceptions::Exception::py_err(format!(
                "Failed to serialize to string. {:?}",
                &self.inner
            ))
        })?)
    }

    #[staticmethod]
    fn deserialize_from_string(string: &str) -> PyResult<PyEditTree> {
        Ok(PyEditTree {
            inner: serde_json::from_str(string).map_err(|_| {
                exceptions::Exception::py_err(format!(
                    "Failed to deserialize edit tree from: {:?}",
                    string
                ))
            })?,
        })
    }
}

#[pyproto]
impl PyObjectProtocol for PyEditTree {
    fn __hash__(&self) -> PyResult<isize> {
        let mut h = DefaultHasher::new();
        self.inner.hash(&mut h);
        let h: u64 = h.finish();
        let r: PyResult<isize> = Ok(h as isize);
        r
    }

    fn __str__(&self) -> PyResult<String> {
        self.serialize_to_string()
    }

    fn __richcmp__(&self, other: &PyAny, op: CompareOp) -> PyResult<bool> {
        let other = if let Ok(other) = t_from_any::<&PyEditTree>(other, Some("EditTree")) {
            other
        } else {
            return Ok(false);
        };
        match op {
            CompareOp::Eq => Ok(self.inner.eq(&other.inner)),
            CompareOp::Ne => Ok(self.inner.ne(&other.inner)),
            _ => type_err("Can only use '==' and '!=' with EditTree.").into(),
        }
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.inner))
    }
}

pub(crate) fn t_from_any<'a, T>(any: &'a PyAny, to: Option<&str>) -> PyResult<T>
where
    T: FromPyObject<'a>,
{
    let any_type = any.get_type().name();
    let msg = to
        .map(|to| format!("expected '{}' got '{}'", to, any_type))
        .unwrap_or_else(|| format!("invalid argument: '{}'", any_type));
    any.extract::<T>().map_err(|_| type_err(msg))
}

pub(crate) fn type_err(msg: impl Into<String>) -> PyErr {
    exceptions::TypeError::py_err(msg.into())
}
