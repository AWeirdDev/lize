use core::str;

use anyhow::{Context, Result};

use lize_sys::{SmallVec, Value, STACK_N};
use pyo3::{
    exceptions,
    prelude::*,
    types::{PyBytes, PyDict, PyFunction, PyNone, PyString, PyTuple},
    IntoPyObjectExt,
};

#[pyclass]
pub enum Runnable {
    /// Coming soon (tm)
    JustInTime(),
    Marshal {
        marshal: Py<PyModule>,
        bytes: Py<PyAny>,
        name: Py<PyAny>,
        annotations: Py<PyAny>,
        runnable: Option<Py<PyAny>>,
        defaults: Py<PyAny>,
        closure: Py<PyAny>,
    },
}

#[pymethods]
impl Runnable {
    #[staticmethod]
    pub fn jit() -> Self {
        Self::JustInTime()
    }

    #[staticmethod]
    pub fn from_pyfn(py: Python<'_>, r#fn: Py<PyFunction>) -> PyResult<Self> {
        let function = r#fn.bind(py);
        let marshal = py.import("marshal")?;

        let bytes = marshal
            .getattr("dumps")?
            .call1((function.getattr("__code__")?,))?
            .unbind();

        Ok(Self::Marshal {
            marshal: marshal.unbind(),
            bytes,
            name: function.getattr("__name__")?.unbind(),
            annotations: function.getattr("__annotations__")?.unbind(),
            defaults: function.getattr("__defaults__")?.unbind(),
            closure: function.getattr("__closure__")?.unbind(),
            runnable: None,
        })
    }

    #[pyo3(name = "run", signature = (*args, **kwargs))]
    pub fn run(
        &self,
        py: Python<'_>,
        args: Py<PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        match self {
            Runnable::JustInTime() => todo!(),
            Runnable::Marshal {
                marshal,
                bytes,
                name,
                annotations,
                defaults,
                closure,
                runnable,
            } => {
                if let Some(r) = runnable {
                    return r.call(py, args, kwargs);
                }

                let code = marshal.getattr(py, "loads")?.call1(py, (bytes,))?;
                let types = py.import("types")?;
                let ft = types.getattr("FunctionType")?.call1((
                    code,
                    PyDict::new(py),
                    name,
                    defaults,
                    closure,
                ))?;
                ft.setattr("__annotations__", annotations)?;

                Ok(ft.call(args, kwargs)?.unbind())
            }
        }
    }

    #[pyo3(name = "__call__", signature = (*args, **kwargs))]
    pub fn __call__(
        &self,
        py: Python<'_>,
        args: Py<PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        self.run(py, args, kwargs)
    }

    pub fn as_bytes(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        match self {
            Self::JustInTime() => todo!(),
            Self::Marshal { .. } => {
                println!("working...");
                let value = self.as_lize(py)?;
                println!("ok");

                let mut buffer = SmallVec::<[u8; STACK_N]>::new();
                value.serialize_into(&mut buffer)?;

                let bytes = PyBytes::new(py, &buffer);
                Ok(bytes.unbind())
            }
        }
    }

    #[staticmethod]
    pub fn from_bytes(py: Python<'_>, bytes: &[u8]) -> PyResult<Self> {
        let value = Value::deserialize_from(bytes)?;
        match value {
            Value::Vector(vec) => {
                if vec.len() != 3 {
                    return Err(exceptions::PyValueError::new_err(
                        "Invalid marshal'd object for lize",
                    ));
                }

                let bytes = vec[0].as_slice().unwrap();
                let name = str::from_utf8(vec[1].as_slice().unwrap())?;
                let defaults = lize_to_py(py, &vec[2])?;

                let marshal = py.import("marshal")?;

                Ok(Self::Marshal {
                    marshal: marshal.unbind(),
                    bytes: PyBytes::new(py, bytes).unbind().into_any(),
                    name: PyString::new(py, name).unbind().into_any(),
                    annotations: py.None(),
                    runnable: None,
                    defaults,
                    closure: py.None(),
                })
            }
            _ => Err(exceptions::PyValueError::new_err("Invalid marshal")),
        }
    }

    pub fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        match self {
            Self::JustInTime() => todo!(),
            Self::Marshal {
                marshal: _,
                bytes: _,
                name,
                annotations,
                ..
            } => {
                if let Ok(ann) = annotations.bind(py).downcast_exact::<PyDict>() {
                    let py_ann = ann
                        .iter()
                        .filter(|(k, _)| k.extract::<&str>().unwrap() != "return")
                        .map(|(k, v)| {
                            format!(
                                "{}: {}",
                                k.extract::<&str>().unwrap_or("?"),
                                v.getattr("__name__")
                                    .map(|v| v.to_string())
                                    .unwrap_or(String::from("?"))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    let result = format!(
                        "Runnable(<marshal> {}({}) -> {})",
                        name.bind(py).to_string(),
                        py_ann,
                        ann.get_item("return")?
                            .map(|v| v
                                .getattr("__name__")
                                .map(|v| v.to_string())
                                .unwrap_or(String::from("?")))
                            .unwrap_or(String::from("?")),
                    );

                    Ok(result)
                } else {
                    Ok(format!(
                        "Runnable(<marshal> {}(...) -> ?)",
                        name.bind(py).to_string()
                    ))
                }
            }
        }
    }
}

impl<'a> Runnable {
    fn as_lize(&'a self, py: Python<'a>) -> PyResult<Value<'a>> {
        match self {
            Self::JustInTime() => todo!(),
            Self::Marshal {
                marshal: _,
                bytes,
                name,
                annotations: _,
                runnable: _,
                defaults,
                closure: _,
            } => Ok(Value::Vector(vec![
                Value::Slice(bytes.extract::<&[u8]>(py)?),          // bytes
                Value::Slice(name.extract::<&str>(py)?.as_bytes()), // name
                py_to_lize(py, defaults.extract(py)?)?,             // defaults
            ])),
        }
    }
}

#[derive(Debug, FromPyObject, IntoPyObject)]
pub enum PyValue {
    Str(String),
    U8(u8),
    Int32(i32),
    Int(i64),
    Float32(f32),
    Float(f64),
    Bool(bool),
    Vec(Vec<Py<PyAny>>),
    Map(Py<PyDict>),
    Run(Py<Runnable>),
    Callable(Py<PyFunction>),
    #[allow(dead_code)]
    None(Py<PyNone>),
}

#[pyfunction]
pub fn serialize(py: Python<'_>, value: PyValue) -> Result<Bound<'_, PyBytes>> {
    let lz = py_to_lize(py, value)?;
    let mut buf = SmallVec::<[u8; STACK_N]>::new();
    lz.serialize_into(&mut buf)?;

    let bytes = PyBytes::new(py, &buf);
    Ok(bytes)
}

#[pyfunction]
pub fn deserialize(py: Python<'_>, bytes: &[u8]) -> Result<Py<PyAny>> {
    let lize_value = Value::deserialize_from(bytes)?;
    let value = lize_to_py(py, &lize_value)?;
    Ok(value)
}

fn py_to_lize(py: Python<'_>, value: PyValue) -> Result<Value<'_>> {
    match value {
        PyValue::Bool(b) => Ok(Value::Bool(b)),
        PyValue::Float32(f) => Ok(Value::F32(f)),
        PyValue::Float(f) => Ok(Value::F64(f)),
        PyValue::U8(u) => {
            if u <= 235 {
                Ok(Value::SmallU8(u))
            } else {
                Ok(Value::U8(u))
            }
        }
        PyValue::Int32(i) => Ok(Value::I32(i)),
        PyValue::Int(i) => Ok(Value::I64(i)),
        PyValue::Str(s) => Ok(Value::SliceLike(format!("s{}", s).into())),
        PyValue::Map(m) => {
            let binding = m.bind(py);
            let mut lize_value = vec![];

            for (k, v) in binding {
                let key = py_to_lize(
                    py,
                    k.extract()
                        .context(format!("Failed to extract key for dict {:?}", binding))?,
                )?;
                let val = py_to_lize(
                    py,
                    v.extract()
                        .context(format!("Failed to extract value for dict {:?}", binding))?,
                )?;
                lize_value.push((key, val));
            }

            Ok(Value::HashMap(lize_value))
        }
        PyValue::None(_) => Ok(Value::Optional(None)),
        PyValue::Vec(mut v) => {
            let mut lize_value = vec![];

            for item in v.drain(..) {
                lize_value.push(py_to_lize(py, item.extract::<PyValue>(py)?)?);
            }

            Ok(Value::Vector(lize_value))
        }
        PyValue::Run(runnable) => {
            let binding = runnable.bind(py);
            let mut data = binding.get().as_lize(py)?.serialize()?;
            data.insert(0, b'r');
            Ok(Value::SliceLike(data))
        }
        PyValue::Callable(callable) => {
            let runnable = Runnable::from_pyfn(py, callable)?;
            let mut data = runnable.as_lize(py)?.serialize()?;
            data.insert(0, b'r');
            Ok(Value::SliceLike(data))
        }
    }
}

fn lize_to_py(py: Python<'_>, lize_value: &Value<'_>) -> Result<Py<PyAny>> {
    match lize_value {
        Value::Bool(b) => Ok(PyValue::Bool(*b).into_py_any(py)?),

        Value::U8(u) => Ok(PyValue::Int(*u as i64).into_py_any(py)?),
        Value::SmallU8(u) => Ok(PyValue::Int(*u as i64).into_py_any(py)?),

        Value::F32(f) => Ok(PyValue::Float(*f as f64).into_py_any(py)?),
        Value::F64(f) => Ok(PyValue::Float(*f).into_py_any(py)?),

        Value::I32(i) => Ok(PyValue::Int(*i as i64).into_py_any(py)?),
        Value::I64(i) => Ok(PyValue::Int(*i).into_py_any(py)?),

        Value::Slice(sl) => {
            if let Ok(s) = str::from_utf8(&sl[0..1]) {
                if s == "s" {
                    Ok(PyValue::Str(String::from_utf8_lossy(&sl[1..]).to_string())
                        .into_py_any(py)?)
                } else if s == "r" {
                    Ok(Runnable::from_bytes(py, &sl[1..])?.into_py_any(py)?)
                } else {
                    Ok(PyValue::Str(s.to_string()).into_py_any(py)?)
                }
            } else {
                Err(anyhow::anyhow!("Invalid slice"))
            }
        }
        Value::SliceLike(_) => unreachable!(),

        Value::HashMap(m) => {
            let map = PyDict::new(py);
            for (k, v) in m {
                let k = lize_to_py(py, k)?;
                let v = lize_to_py(py, v)?;
                map.set_item(k, v)?;
            }

            Ok(PyValue::Map(map.unbind()).into_py_any(py)?)
        }

        Value::Optional(_) => Ok(py.None().into_py_any(py)?),
        Value::Vector(v) => {
            let mut vec = vec![];
            for item in v {
                vec.push(lize_to_py(py, item)?);
            }

            Ok(PyValue::Vec(vec).into_py_any(py)?)
        }
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn lize(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(serialize, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize, m)?)?;
    m.add_class::<Runnable>()?;

    Ok(())
}
