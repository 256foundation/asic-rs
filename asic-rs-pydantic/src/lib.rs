use std::{fmt::Display, net::IpAddr, str::FromStr, time::Duration};

use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use pyo3::{
    PyTypeInfo,
    exceptions::PyValueError,
    prelude::*,
    types::{PyAnyMethods, PyBool, PyDict, PyDictMethods, PyList, PyListMethods, PyType},
};

pub use asic_rs_pydantic_macros::{
    PyPydanticData, PyPydanticEnum, PyPydanticModel, PyPydanticTaggedUnion, py_pydantic_model,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PydanticSchemaMode {
    Validation,
    Serialization,
}

pub trait PyPydanticType: Sized {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>>;

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self>;

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>>;

    fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.to_pydantic_data(py)
    }
}

pub trait PyPydanticStringEnum: Clone + Display + FromStr + PyTypeInfo + Sized {
    const PYDANTIC_VALUES: &'static [&'static str];

    fn to_pydantic_enum_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>>;
}

impl<T> PyPydanticType for T
where
    T: PyPydanticStringEnum + for<'py> FromPyObject<'py, 'py>,
    <T as FromStr>::Err: Display,
    for<'py> <T as FromPyObject<'py, 'py>>::Error: Into<PyErr>,
{
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        let string_schema = literal_schema(core_schema, T::PYDANTIC_VALUES)?;
        if mode == PydanticSchemaMode::Serialization {
            return Ok(string_schema);
        }

        let py = core_schema.py();
        let instance_schema =
            core_schema.call_method1("is_instance_schema", (py.get_type::<T>(),))?;
        union_schema(core_schema, [instance_schema, string_schema])
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(value) = value.extract::<T>() {
            return Ok(value);
        }

        let value = value.extract::<String>()?;
        T::from_str(&value).map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.to_string().into_pyobject(py)?.into_any().unbind())
    }

    fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.to_pydantic_enum_repr_value(py)
    }
}

macro_rules! impl_pydantic_python_value {
    ($schema:literal; $($ty:ty),* $(,)?) => {
        $(
            impl PyPydanticType for $ty {
                fn pydantic_schema<'py>(
                    core_schema: &Bound<'py, PyAny>,
                    _mode: PydanticSchemaMode,
                ) -> PyResult<Bound<'py, PyAny>> {
                    core_schema.call_method0($schema)
                }

                fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
                    value.extract()
                }

                fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                    Ok((*self).into_pyobject(py)?.clone().into_any().unbind())
                }
            }
        )*
    };
}

impl_pydantic_python_value!("int_schema"; i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
impl_pydantic_python_value!("float_schema"; f32, f64);

impl PyPydanticType for bool {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        _mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        core_schema.call_method0("bool_schema")
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        value.extract()
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyBool::new(py, *self).to_owned().into_any().unbind())
    }
}

impl PyPydanticType for String {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        _mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        core_schema.call_method0("str_schema")
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        value.extract()
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.clone().into_pyobject(py)?.into_any().unbind())
    }
}

impl PyPydanticType for IpAddr {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        _mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        core_schema.call_method0("str_schema")
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(ip) = value.extract::<Self>() {
            return Ok(ip);
        }
        value
            .extract::<String>()?
            .parse()
            .map_err(|error| PyValueError::new_err(format!("Invalid IP address: {error}")))
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.to_string().into_pyobject(py)?.into_any().unbind())
    }
}

impl PyPydanticType for MacAddr {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        _mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        core_schema.call_method0("str_schema")
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        value
            .extract::<String>()?
            .parse()
            .map_err(|error| PyValueError::new_err(format!("Invalid MAC address: {error}")))
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.to_string().into_pyobject(py)?.into_any().unbind())
    }
}

fn duration_to_seconds(duration: Duration) -> f64 {
    duration.as_secs() as f64
}

impl PyPydanticType for Duration {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        match mode {
            PydanticSchemaMode::Validation => core_schema.call_method0("any_schema"),
            PydanticSchemaMode::Serialization => core_schema.call_method0("float_schema"),
        }
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(duration) = value.extract::<Self>() {
            return Ok(duration);
        }
        if let Ok(seconds) = value.extract::<f64>()
            && seconds.is_finite()
            && seconds >= 0.0
        {
            return Ok(Self::from_secs_f64(seconds));
        }
        if let Ok(dict) = value.cast::<PyDict>() {
            let secs = required_dict_item(dict, "secs")?.extract::<u64>()?;
            return Ok(Self::from_secs(secs));
        }
        Err(PyValueError::new_err(
            "Expected duration as timedelta, non-negative seconds, or {secs} dict",
        ))
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(duration_to_seconds(*self)
            .into_pyobject(py)?
            .into_any()
            .unbind())
    }
}

macro_rules! impl_pydantic_measurement {
    ($ty:ty, $from_unit:ident, $as_unit:ident) => {
        impl PyPydanticType for $ty {
            fn pydantic_schema<'py>(
                core_schema: &Bound<'py, PyAny>,
                _mode: PydanticSchemaMode,
            ) -> PyResult<Bound<'py, PyAny>> {
                core_schema.call_method0("float_schema")
            }

            fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
                Ok(Self::$from_unit(value.extract::<f64>()?))
            }

            fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                Ok(self.$as_unit().into_pyobject(py)?.into_any().unbind())
            }
        }
    };
}

impl_pydantic_measurement!(AngularVelocity, from_rpm, as_rpm);
impl_pydantic_measurement!(Frequency, from_megahertz, as_megahertz);
impl_pydantic_measurement!(Power, from_watts, as_watts);
impl_pydantic_measurement!(Temperature, from_celsius, as_celsius);
impl_pydantic_measurement!(Voltage, from_volts, as_volts);

impl<T> PyPydanticType for Option<T>
where
    T: PyPydanticType,
{
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = T::pydantic_schema(core_schema, mode)?;
        nullable_schema(core_schema, &inner)
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if value.is_none() {
            Ok(None)
        } else {
            T::from_pydantic(value).map(Some)
        }
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if let Some(value) = self {
            value.to_pydantic_data(py)
        } else {
            Ok(py.None())
        }
    }

    fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if let Some(value) = self {
            value.to_pydantic_repr_value(py)
        } else {
            Ok(py.None())
        }
    }
}

impl<T> PyPydanticType for Vec<T>
where
    T: PyPydanticType,
{
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = T::pydantic_schema(core_schema, mode)?;
        list_schema(core_schema, &inner)
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        value
            .try_iter()?
            .map(|item| {
                let item = item?;
                T::from_pydantic(&item)
            })
            .collect()
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for value in self {
            list.append(value.to_pydantic_data(py)?)?;
        }
        Ok(list.into_any().unbind())
    }

    fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for value in self {
            list.append(value.to_pydantic_repr_value(py)?)?;
        }
        Ok(list.into_any().unbind())
    }
}

pub fn typed_dict_field<'py>(
    core_schema: &Bound<'py, PyAny>,
    schema: &Bound<'py, PyAny>,
    required: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let kwargs = PyDict::new(core_schema.py());
    kwargs.set_item("required", required)?;
    core_schema.call_method("typed_dict_field", (schema,), Some(&kwargs))
}

pub fn typed_dict_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    fields: &Bound<'py, PyDict>,
    ref_name: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let kwargs = PyDict::new(core_schema.py());
    if let Some(ref_name) = ref_name {
        kwargs.set_item("ref", ref_name)?;
    }
    core_schema.call_method("typed_dict_schema", (fields,), Some(&kwargs))
}

#[macro_export]
macro_rules! pydantic_typed_dict_schema {
    ($core_schema:expr, $ref_name:expr, { $($fields:tt)* }) => {{
        let fields = ::pyo3::types::PyDict::new($core_schema.py());
        $crate::pydantic_typed_dict_schema!(@fields fields, $core_schema, $($fields)*,);
        $crate::typed_dict_schema($core_schema, &fields, Some($ref_name))
    }};

    (@fields $fields:ident, $core_schema:expr, $(,)?) => {};

    (@fields $fields:ident, $core_schema:expr, $field:expr => required($schema:expr), $($rest:tt)*) => {{
        $crate::pydantic_typed_dict_schema!(@insert $fields, $core_schema, $field, required($schema));
        $crate::pydantic_typed_dict_schema!(@fields $fields, $core_schema, $($rest)*);
    }};

    (@fields $fields:ident, $core_schema:expr, $field:expr => required_if($schema:expr, $required:expr), $($rest:tt)*) => {{
        $crate::pydantic_typed_dict_schema!(@insert $fields, $core_schema, $field, required_if($schema, $required));
        $crate::pydantic_typed_dict_schema!(@fields $fields, $core_schema, $($rest)*);
    }};

    (@fields $fields:ident, $core_schema:expr, $field:expr => nullable($schema:expr), $($rest:tt)*) => {{
        $crate::pydantic_typed_dict_schema!(@insert $fields, $core_schema, $field, nullable($schema));
        $crate::pydantic_typed_dict_schema!(@fields $fields, $core_schema, $($rest)*);
    }};

    (@fields $fields:ident, $core_schema:expr, $field:expr => nullable_if($schema:expr, $required:expr), $($rest:tt)*) => {{
        $crate::pydantic_typed_dict_schema!(@insert $fields, $core_schema, $field, nullable_if($schema, $required));
        $crate::pydantic_typed_dict_schema!(@fields $fields, $core_schema, $($rest)*);
    }};

    (@insert $fields:ident, $core_schema:expr, $field:expr, required($schema:expr)) => {{
        $fields.set_item(
            $field,
            $crate::typed_dict_field($core_schema, &$schema, true)?,
        )?;
    }};

    (@insert $fields:ident, $core_schema:expr, $field:expr, required_if($schema:expr, $required:expr)) => {{
        $fields.set_item(
            $field,
            $crate::typed_dict_field($core_schema, &$schema, $required)?,
        )?;
    }};

    (@insert $fields:ident, $core_schema:expr, $field:expr, nullable($schema:expr)) => {{
        $fields.set_item(
            $field,
            $crate::nullable_field($core_schema, &$schema, true)?,
        )?;
    }};

    (@insert $fields:ident, $core_schema:expr, $field:expr, nullable_if($schema:expr, $required:expr)) => {{
        $fields.set_item(
            $field,
            $crate::nullable_field($core_schema, &$schema, $required)?,
        )?;
    }};

}

pub fn tagged_union_schema<'py, I>(
    core_schema: &Bound<'py, PyAny>,
    choices: I,
    discriminator: &str,
    ref_name: Option<&str>,
) -> PyResult<Bound<'py, PyAny>>
where
    I: IntoIterator<Item = (&'static str, Bound<'py, PyAny>)>,
{
    let py = core_schema.py();
    let choices_dict = PyDict::new(py);
    for (tag, schema) in choices {
        choices_dict.set_item(tag, schema)?;
    }
    let kwargs = PyDict::new(py);
    if let Some(ref_name) = ref_name {
        kwargs.set_item("ref", ref_name)?;
    }
    core_schema.call_method(
        "tagged_union_schema",
        (choices_dict, discriminator),
        Some(&kwargs),
    )
}

pub fn union_schema<'py, I>(
    core_schema: &Bound<'py, PyAny>,
    choices: I,
) -> PyResult<Bound<'py, PyAny>>
where
    I: IntoIterator<Item = Bound<'py, PyAny>>,
{
    let choices_list = PyList::empty(core_schema.py());
    for schema in choices {
        choices_list.append(schema)?;
    }
    core_schema.call_method1("union_schema", (choices_list,))
}

pub fn literal_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    values: &[&str],
) -> PyResult<Bound<'py, PyAny>> {
    let values = PyList::new(core_schema.py(), values)?;
    core_schema.call_method1("literal_schema", (values,))
}

pub fn nullable_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    schema: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    core_schema.call_method1("nullable_schema", (schema,))
}

pub fn nullable_field<'py>(
    core_schema: &Bound<'py, PyAny>,
    schema: &Bound<'py, PyAny>,
    required: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let schema = nullable_schema(core_schema, schema)?;
    typed_dict_field(core_schema, &schema, required)
}

pub fn list_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    item_schema: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    core_schema.call_method1("list_schema", (item_schema,))
}

pub fn required_dict_item<'py>(
    dict: &Bound<'py, PyDict>,
    key: &str,
) -> PyResult<Bound<'py, PyAny>> {
    dict.get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("Missing required key: {key}")))
}

pub fn py_to_string(value: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(value.str()?.to_str()?.to_string())
}

pub fn get_required_field<'py>(
    value: &Bound<'py, PyAny>,
    key: &str,
) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(dict) = value.cast::<PyDict>() {
        required_dict_item(dict, key)
    } else {
        value.getattr(key)
    }
}

pub fn get_optional_field<'py>(
    value: &Bound<'py, PyAny>,
    key: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    if let Ok(dict) = value.cast::<PyDict>() {
        dict.get_item(key)
    } else if value.hasattr(key)? {
        Ok(Some(value.getattr(key)?))
    } else {
        Ok(None)
    }
}

pub fn parse_optional<T>(value: Option<Bound<'_, PyAny>>) -> PyResult<Option<T>>
where
    for<'a> T: FromPyObject<'a, 'a>,
    for<'a> <T as FromPyObject<'a, 'a>>::Error: Into<PyErr>,
{
    match value {
        Some(value) if value.is_none() => Ok(None),
        Some(value) => value.extract().map(Some).map_err(Into::into),
        None => Ok(None),
    }
}

pub fn parse_required_list<T, F>(value: &Bound<'_, PyAny>, key: &str, parse: F) -> PyResult<Vec<T>>
where
    F: for<'py> Fn(&Bound<'py, PyAny>) -> PyResult<T>,
{
    get_required_field(value, key)?
        .try_iter()?
        .map(|item| {
            let item = item?;
            parse(&item)
        })
        .collect()
}

pub fn parse_required_option<T>(value: &Bound<'_, PyAny>, key: &str) -> PyResult<Option<T>>
where
    for<'a> T: FromPyObject<'a, 'a>,
    for<'a> <T as FromPyObject<'a, 'a>>::Error: Into<PyErr>,
{
    get_required_field(value, key)?
        .extract::<Option<T>>()
        .map_err(Into::into)
}

pub fn model_core_schema(
    cls: &Bound<'_, PyType>,
    validation_schema: &Bound<'_, PyAny>,
    serialization_schema: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let py = cls.py();
    let core_schema = py.import("pydantic_core")?.getattr("core_schema")?;
    let validator = cls.getattr("_pydantic_validate")?;
    let serializer = cls.getattr("_pydantic_serialize")?;
    let instance_schema = core_schema.call_method1("is_instance_schema", (cls,))?;
    let python_schema = union_schema(&core_schema, [instance_schema, validation_schema.clone()])?;
    let serializer_kwargs = PyDict::new(py);
    serializer_kwargs.set_item("return_schema", serialization_schema)?;
    let serializer_schema = core_schema.call_method(
        "plain_serializer_function_ser_schema",
        (serializer,),
        Some(&serializer_kwargs),
    )?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("json_schema_input_schema", validation_schema)?;
    kwargs.set_item("serialization", serializer_schema)?;
    let schema = core_schema.call_method(
        "no_info_after_validator_function",
        (validator, python_schema),
        Some(&kwargs),
    )?;
    Ok(schema.unbind())
}

pub fn model_json_schema(
    cls: &Bound<'_, PyType>,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let adapter = cls
        .py()
        .import("pydantic")?
        .getattr("TypeAdapter")?
        .call1((cls,))?;
    Ok(adapter.call_method("json_schema", (), kwargs)?.unbind())
}

pub fn reject_model_kwargs(kwargs: Option<&Bound<'_, PyDict>>, method: &str) -> PyResult<()> {
    if let Some(kwargs) = kwargs
        && !kwargs.is_empty()
    {
        return Err(PyValueError::new_err(format!(
            "{method} keyword arguments are not supported by asic_rs models"
        )));
    }
    Ok(())
}
