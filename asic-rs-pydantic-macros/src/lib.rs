use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Field, Fields, ItemStruct, LitStr, Path, parse_macro_input,
    spanned::Spanned,
};

#[derive(Default)]
struct PydanticModelOptions {
    schema: Option<LitStr>,
    parse: Option<LitStr>,
    new: bool,
    repr: bool,
    no_repr: bool,
    getters: bool,
    name: Option<LitStr>,
}

impl PydanticModelOptions {
    fn parse_option(&mut self, meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<bool> {
        if meta.path.is_ident("schema") {
            self.schema = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("parse") {
            self.parse = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("new") {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error("new does not accept a value"));
            }
            self.new = true;
        } else if meta.path.is_ident("repr") {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error("repr does not accept a value"));
            }
            self.repr = true;
        } else if meta.path.is_ident("no_repr") {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error("no_repr does not accept a value"));
            }
            self.no_repr = true;
        } else if meta.path.is_ident("getters") {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error("getters does not accept a value"));
            }
            self.getters = true;
        } else if meta.path.is_ident("name") {
            if self.name.is_some() {
                return Err(meta.error("duplicate pydantic name"));
            }
            self.name = Some(meta.value()?.parse()?);
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn parse(input: &DeriveInput) -> syn::Result<Self> {
        let mut options = Self::default();

        for attr in &input.attrs {
            if !attr.path().is_ident("pydantic") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if options.parse_option(&meta)? {
                    Ok(())
                } else {
                    Err(meta.error("unknown pydantic option"))
                }
            })?;
        }

        Ok(options)
    }
}

#[derive(Default)]
struct PydanticModelAttrOptions {
    model: PydanticModelOptions,
    manual: bool,
}

#[derive(Default)]
struct PydanticModelFieldOptions {
    default: Option<Expr>,
    literal: Option<LitStr>,
}

impl PydanticModelFieldOptions {
    fn parse(field: &Field) -> syn::Result<Self> {
        let mut options = Self::default();

        for attr in &field.attrs {
            if !attr.path().is_ident("pydantic") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    if options.default.is_some() {
                        return Err(meta.error("duplicate pydantic default"));
                    }
                    options.default = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("literal") {
                    if options.literal.is_some() {
                        return Err(meta.error("duplicate pydantic literal"));
                    }
                    options.literal = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("unknown pydantic field option"))
                }
            })?;
        }

        if options.default.is_some() && options.literal.is_some() {
            return Err(syn::Error::new_spanned(
                field,
                "pydantic default and literal cannot both be set",
            ));
        }

        Ok(options)
    }
}

#[proc_macro_attribute]
pub fn py_pydantic_model(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut options = PydanticModelAttrOptions::default();
    let parser = syn::meta::parser(|meta| {
        if options.model.parse_option(&meta)? {
            Ok(())
        } else if meta.path.is_ident("manual") {
            if meta.input.peek(syn::Token![=]) {
                return Err(meta.error("manual does not accept a value"));
            }
            options.manual = true;
            Ok(())
        } else {
            Err(meta.error("unknown py_pydantic_model option"))
        }
    });
    parse_macro_input!(args with parser);

    let item = parse_macro_input!(input as ItemStruct);
    expand_py_pydantic_model_attr(item, options)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_py_pydantic_model_attr(
    item: ItemStruct,
    options: PydanticModelAttrOptions,
) -> syn::Result<proc_macro2::TokenStream> {
    if options.model.repr && options.model.no_repr {
        return Err(syn::Error::new_spanned(
            &item,
            "repr and no_repr cannot both be set",
        ));
    }

    let new = options.model.new.then(|| quote!(new));
    let data = (!options.manual).then(|| quote!(, ::asic_rs_pydantic::PyPydanticData));
    let mut pydantic_options = Vec::new();
    if let Some(schema) = options.model.schema {
        pydantic_options.push(quote!(schema = #schema));
    }
    if let Some(parse) = options.model.parse {
        pydantic_options.push(quote!(parse = #parse));
    }
    if let Some(new) = new {
        pydantic_options.push(new);
    }
    if options.model.no_repr {
        pydantic_options.push(quote!(no_repr));
    }
    if options.model.getters {
        pydantic_options.push(quote!(getters));
    }
    if let Some(name) = options.model.name {
        pydantic_options.push(quote!(name = #name));
    }
    let pydantic_attr =
        (!pydantic_options.is_empty()).then(|| quote!(#[pydantic(#(#pydantic_options),*)]));

    Ok(quote! {
        #[derive(::asic_rs_pydantic::PyPydanticModel #data)]
        #pydantic_attr
        #item
    })
}

#[proc_macro_derive(PyPydanticModel, attributes(pydantic))]
pub fn derive_py_pydantic_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_py_pydantic_model(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_py_pydantic_model(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let options = PydanticModelOptions::parse(input)?;
    let rust_name_str = name.to_string();
    let name_str = options
        .name
        .clone()
        .unwrap_or_else(|| LitStr::new(&rust_name_str, name.span()));
    let schema = options
        .schema
        .map(|schema| schema.parse::<Path>())
        .transpose()?;
    let parse = options
        .parse
        .map(|parse| parse.parse::<Path>())
        .transpose()?;

    if schema.is_some() != parse.is_some() {
        return Err(syn::Error::new_spanned(
            input,
            "PyPydanticModel requires schema and parse to be provided together",
        ));
    }

    let schema_body = schema.as_ref().map_or_else(
        || expand_generated_pydantic_schema(input),
        |schema| {
            Ok(quote! {
                #schema(core_schema, mode)
            })
        },
    )?;
    let parse_body = parse.as_ref().map_or_else(
        || expand_generated_pydantic_parse(input),
        |parse| {
            Ok(quote! {
                #parse(value)
            })
        },
    )?;
    let new_method = options.new.then(|| {
        quote! {
            #[new]
            #[pyo3(signature = (**kwargs: "object"))]
            fn new(
                kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<Self> {
                let kwargs = kwargs.ok_or_else(|| {
                    ::pyo3::exceptions::PyValueError::new_err(
                        "Expected keyword arguments for data model",
                    )
                })?;
                use ::pyo3::types::PyAnyMethods as _;
                <Self as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(kwargs.as_any())
            }
        }
    });
    if options.repr && options.no_repr {
        return Err(syn::Error::new_spanned(
            input,
            "repr and no_repr cannot both be set",
        ));
    }

    let repr_method = (!options.no_repr)
        .then(|| expand_generated_pydantic_repr(input))
        .transpose()?;
    let getter_methods = options
        .getters
        .then(|| expand_generated_pydantic_getters(input))
        .transpose()?;

    Ok(quote! {
        impl ::asic_rs_pydantic::PyPydanticType for #name {
            fn pydantic_schema<'py>(
                core_schema: &::pyo3::Bound<'py, ::pyo3::PyAny>,
                mode: ::asic_rs_pydantic::PydanticSchemaMode,
            ) -> ::pyo3::PyResult<::pyo3::Bound<'py, ::pyo3::PyAny>> {
                #schema_body
            }

            fn from_pydantic(value: &::pyo3::Bound<'_, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
                #parse_body
            }

            fn to_pydantic_data(
                &self,
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                self.to_pydantic_data(py)
            }

            fn to_pydantic_repr_value(
                &self,
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::IntoPyObject as _;
                Ok(<Self as ::std::clone::Clone>::clone(self)
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            }
        }

        #[::pyo3::pymethods]
        impl #name {
            #new_method
            #repr_method
            #getter_methods

            #[classmethod]
            #[pyo3(signature = (_source_type: "object", _handler: "object") -> "object")]
            pub fn __get_pydantic_core_schema__(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                _source_type: &::pyo3::Bound<'_, ::pyo3::PyAny>,
                _handler: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                let core_schema = cls.py().import("pydantic_core")?.getattr("core_schema")?;
                let validation_schema =
                    <Self as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                    &core_schema,
                    ::asic_rs_pydantic::PydanticSchemaMode::Validation,
                )?;
                let serialization_schema =
                    <Self as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                    &core_schema,
                    ::asic_rs_pydantic::PydanticSchemaMode::Serialization,
                )?;
                ::asic_rs_pydantic::model_core_schema(
                    cls,
                    &validation_schema,
                    &serialization_schema,
                )
            }

            #[classmethod]
            #[pyo3(signature = (obj: "object", **_kwargs: "object") -> #name_str)]
            pub fn model_validate(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
                _kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                use ::pyo3::IntoPyObject as _;
                ::asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_validate")?;
                if obj.is_instance(cls)? {
                    return Ok(obj.clone().unbind());
                }
                Ok(
                    <Self as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(obj)?
                        .into_pyobject(obj.py())?
                        .into_any()
                        .unbind()
                )
            }

            #[classmethod]
            #[pyo3(signature = (**kwargs: "object") -> "dict[str, object]")]
            pub fn model_json_schema(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                ::asic_rs_pydantic::model_json_schema(cls, kwargs)
            }

            #[pyo3(signature = (**_kwargs: "object") -> "dict[str, object]")]
            pub fn model_dump(
                &self,
                py: ::pyo3::Python<'_>,
                _kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                ::asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_dump")?;
                <Self as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(self, py)
            }

            #[classmethod]
            #[pyo3(signature = (value: "object") -> #name_str)]
            fn _pydantic_validate(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                value: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                Self::model_validate(cls, value, None)
            }

            #[staticmethod]
            #[pyo3(signature = (value: #name_str) -> "dict[str, object]")]
            fn _pydantic_serialize(
                value: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                let model = value.extract::<Self>()?;
                <Self as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(&model, value.py())
            }
        }
    })
}

fn expand_generated_pydantic_repr(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name_str = input.ident.to_string();
    let fields = named_struct_fields(input)?
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
            let ty = &field.ty;
            let key = LitStr::new(&ident.to_string(), ident.span());
            Ok(quote! {
                let value =
                    <#ty as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_repr_value(
                        &self.#ident,
                        py,
                    )?;
                let value_repr: String = value.bind(py).repr()?.extract()?;
                fields.push(format!("{}={}", #key, value_repr));
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        fn __repr__(&self, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<String> {
            use ::pyo3::types::PyAnyMethods as _;
            let mut fields = Vec::new();
            #(#fields)*
            Ok(format!("{}({})", #name_str, fields.join(", ")))
        }
    })
}

fn expand_generated_pydantic_getters(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = named_struct_fields(input)?
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
            let ty = &field.ty;
            Ok(quote! {
                #[getter]
                fn #ident(
                    &self,
                    py: ::pyo3::Python<'_>,
                ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                    <#ty as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_repr_value(
                        &self.#ident,
                        py,
                    )
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#fields)*
    })
}

fn named_struct_fields(
    input: &DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<Field, syn::Token![,]>> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(&fields.named),
            _ => Err(syn::Error::new_spanned(
                input,
                "PyPydanticModel generated schemas require a struct with named fields",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            input,
            "PyPydanticModel generated schemas require a struct",
        )),
    }
}

fn expand_generated_pydantic_schema(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name_str = input.ident.to_string();
    let fields = named_struct_fields(input)?
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
            let ty = &field.ty;
            let options = PydanticModelFieldOptions::parse(field)?;
            let key = LitStr::new(&ident.to_string(), ident.span());
            let schema = if let Some(literal) = &options.literal {
                quote! {
                    ::asic_rs_pydantic::literal_schema(core_schema, &[#literal])?
                }
            } else {
                quote! {
                    <#ty as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                        core_schema,
                        mode,
                    )?
                }
            };
            let required = if options.default.is_some() || options.literal.is_some() {
                quote!(mode == ::asic_rs_pydantic::PydanticSchemaMode::Serialization)
            } else {
                quote!(true)
            };
            Ok(quote! {
                let field_schema = #schema;
                fields.set_item(
                    #key,
                    ::asic_rs_pydantic::typed_dict_field(core_schema, &field_schema, #required)?,
                )?;
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        {
            use ::pyo3::types::PyDictMethods as _;
            let fields = ::pyo3::types::PyDict::new(core_schema.py());
            #(#fields)*
            ::asic_rs_pydantic::typed_dict_schema(
                core_schema,
                &fields,
                Some(concat!("asic_rs.", #name_str)),
            )
        }
    })
}

fn expand_generated_pydantic_parse(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = named_struct_fields(input)?
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
            let ty = &field.ty;
            let options = PydanticModelFieldOptions::parse(field)?;
            let key = LitStr::new(&ident.to_string(), ident.span());
            if let Some(literal) = options.literal {
                Ok(quote! {
                    #ident: {
                        if let Some(actual) = ::asic_rs_pydantic::get_optional_field(value, #key)? {
                            let actual = ::asic_rs_pydantic::py_to_string(&actual)?;
                            if actual != #literal {
                                return Err(::pyo3::exceptions::PyValueError::new_err(
                                    format!(
                                        "Expected {} to be {:?}, got {:?}",
                                        #key,
                                        #literal,
                                        actual,
                                    ),
                                ));
                            }
                        }
                        #literal.to_owned()
                    },
                })
            } else if let Some(default) = options.default {
                Ok(quote! {
                    #ident: if let Some(field) = ::asic_rs_pydantic::get_optional_field(value, #key)? {
                        <#ty as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(&field)?
                    } else {
                        #default
                    },
                })
            } else {
                Ok(quote! {
                    #ident: <#ty as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(
                        &::asic_rs_pydantic::get_required_field(value, #key)?,
                    )?,
                })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        {
            use ::pyo3::types::PyAnyMethods as _;
            if let Ok(model) = value.extract::<Self>() {
                return Ok(model);
            }
            Ok(Self {
                #(#fields)*
            })
        }
    })
}

#[proc_macro_derive(PyPydanticEnum, attributes(pydantic))]
pub fn derive_py_pydantic_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_py_pydantic_enum(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(Default)]
struct PydanticEnumVariantOptions {
    value: Option<LitStr>,
}

impl PydanticEnumVariantOptions {
    fn parse(variant: &syn::Variant) -> syn::Result<Self> {
        let mut options = Self::default();

        for attr in &variant.attrs {
            if !attr.path().is_ident("pydantic") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value") {
                    if options.value.is_some() {
                        return Err(meta.error("duplicate pydantic enum value"));
                    }
                    options.value = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("unknown pydantic enum option"))
                }
            })?;
        }

        Ok(options)
    }
}

fn expand_py_pydantic_enum(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "PyPydanticEnum requires an enum",
        ));
    };

    let values = data
        .variants
        .iter()
        .map(|variant| {
            if !matches!(variant.fields, Fields::Unit) {
                return Err(syn::Error::new_spanned(
                    variant,
                    "PyPydanticEnum only supports fieldless variants",
                ));
            }

            PydanticEnumVariantOptions::parse(variant)?
                .value
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        variant,
                        "PyPydanticEnum variants require #[pydantic(value = \"...\")]",
                    )
                })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::asic_rs_pydantic::PyPydanticStringEnum for #name #ty_generics #where_clause {
            const PYDANTIC_VALUES: &'static [&'static str] = &[#(#values),*];

            fn to_pydantic_enum_repr_value(
                &self,
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::IntoPyObject as _;
                Ok(<Self as ::std::clone::Clone>::clone(self)
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            }
        }

        #[::pyo3::pymethods]
        impl #name {
            #[classmethod]
            #[pyo3(signature = (_source_type: "object", _handler: "object") -> "object")]
            pub fn __get_pydantic_core_schema__(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                _source_type: &::pyo3::Bound<'_, ::pyo3::PyAny>,
                _handler: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                let core_schema = cls.py().import("pydantic_core")?.getattr("core_schema")?;
                let validation_schema =
                    <Self as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                        &core_schema,
                        ::asic_rs_pydantic::PydanticSchemaMode::Validation,
                    )?;
                let serialization_schema =
                    <Self as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                        &core_schema,
                        ::asic_rs_pydantic::PydanticSchemaMode::Serialization,
                    )?;
                ::asic_rs_pydantic::model_core_schema(
                    cls,
                    &validation_schema,
                    &serialization_schema,
                )
            }

            #[classmethod]
            #[pyo3(signature = (obj: "object", **_kwargs: "object") -> #name_str)]
            pub fn model_validate(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
                _kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                use ::pyo3::IntoPyObject as _;
                ::asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_validate")?;
                if obj.is_instance(cls)? {
                    return Ok(obj.clone().unbind());
                }
                Ok(
                    <Self as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(obj)?
                        .into_pyobject(obj.py())?
                        .into_any()
                        .unbind()
                )
            }

            #[classmethod]
            #[pyo3(signature = (**kwargs: "object") -> "dict[str, object]")]
            pub fn model_json_schema(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                ::asic_rs_pydantic::model_json_schema(cls, kwargs)
            }

            #[pyo3(signature = (**_kwargs: "object") -> "object")]
            pub fn model_dump(
                &self,
                py: ::pyo3::Python<'_>,
                _kwargs: Option<&::pyo3::Bound<'_, ::pyo3::types::PyDict>>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                ::asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_dump")?;
                <Self as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(self, py)
            }

            #[classmethod]
            #[pyo3(signature = (value: "object") -> #name_str)]
            fn _pydantic_validate(
                cls: &::pyo3::Bound<'_, ::pyo3::types::PyType>,
                value: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                Self::model_validate(cls, value, None)
            }

            #[staticmethod]
            #[pyo3(signature = (value: #name_str) -> "object")]
            fn _pydantic_serialize(
                value: &::pyo3::Bound<'_, ::pyo3::PyAny>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::PyAnyMethods as _;
                let model = value.extract::<Self>()?;
                <Self as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(&model, value.py())
            }
        }
    })
}

#[derive(Default)]
struct TaggedUnionOptions {
    discriminator: Option<LitStr>,
    ref_name: Option<LitStr>,
    value_field: Option<LitStr>,
}

impl TaggedUnionOptions {
    fn parse(input: &DeriveInput) -> syn::Result<Self> {
        let mut options = Self::default();

        for attr in &input.attrs {
            if !attr.path().is_ident("pydantic") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("discriminator") {
                    if options.discriminator.is_some() {
                        return Err(meta.error("duplicate pydantic discriminator"));
                    }
                    options.discriminator = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("ref") {
                    if options.ref_name.is_some() {
                        return Err(meta.error("duplicate pydantic ref"));
                    }
                    options.ref_name = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("value") {
                    if options.value_field.is_some() {
                        return Err(meta.error("duplicate pydantic value field"));
                    }
                    options.value_field = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("unknown pydantic tagged union option"))
                }
            })?;
        }

        Ok(options)
    }
}

struct TaggedUnionVariant<'a> {
    ident: &'a syn::Ident,
    ty: &'a syn::Type,
    tag: LitStr,
}

fn parse_tagged_union_variant(variant: &syn::Variant) -> syn::Result<TaggedUnionVariant<'_>> {
    let mut tag = None;
    for attr in &variant.attrs {
        if !attr.path().is_ident("pydantic") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                if tag.is_some() {
                    return Err(meta.error("duplicate pydantic tag"));
                }
                tag = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown pydantic tagged union variant option"))
            }
        })?;
    }

    let tag = tag.ok_or_else(|| {
        syn::Error::new_spanned(
            variant,
            "PyPydanticTaggedUnion variants require #[pydantic(tag = \"...\")]",
        )
    })?;
    let Fields::Unnamed(fields) = &variant.fields else {
        return Err(syn::Error::new_spanned(
            variant,
            "PyPydanticTaggedUnion variants require one unnamed field",
        ));
    };
    if fields.unnamed.len() != 1 {
        return Err(syn::Error::new_spanned(
            variant,
            "PyPydanticTaggedUnion variants require one unnamed field",
        ));
    }
    let ty = &fields.unnamed[0].ty;

    Ok(TaggedUnionVariant {
        ident: &variant.ident,
        ty,
        tag,
    })
}

#[proc_macro_derive(PyPydanticTaggedUnion, attributes(pydantic))]
pub fn derive_py_pydantic_tagged_union(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_py_pydantic_tagged_union(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_py_pydantic_tagged_union(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();
    let name_str_lit = LitStr::new(&name_str, name.span());
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "PyPydanticTaggedUnion does not support generic enums",
        ));
    }
    let options = TaggedUnionOptions::parse(input)?;
    let discriminator = options.discriminator.ok_or_else(|| {
        syn::Error::new_spanned(
            input,
            "PyPydanticTaggedUnion requires #[pydantic(discriminator = \"...\")]",
        )
    })?;
    let ref_name = options.ref_name.unwrap_or_else(|| {
        LitStr::new(
            &format!("asic_rs.{name_str}"),
            proc_macro2::Span::call_site(),
        )
    });
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "PyPydanticTaggedUnion requires an enum",
        ));
    };
    let variants = data
        .variants
        .iter()
        .map(parse_tagged_union_variant)
        .collect::<syn::Result<Vec<_>>>()?;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let schema_choices = if let Some(value_field) = &options.value_field {
        variants
            .iter()
            .map(|variant| {
                let tag = &variant.tag;
                let ty = variant.ty;
                let variant_name = variant.ident.to_string();
                quote! {
                    {
                        let tag_schema = ::asic_rs_pydantic::literal_schema(core_schema, &[#tag])?;
                        let value_schema =
                            <#ty as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                                core_schema,
                                mode,
                            )?;
                        let variant_ref = ::std::format!("{}{}", #ref_name, #variant_name);
                        (
                            #tag,
                            ::asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, &variant_ref, {
                                #discriminator => required(tag_schema),
                                #value_field => required(value_schema),
                            })?,
                        )
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        variants
            .iter()
            .map(|variant| {
                let tag = &variant.tag;
                let ty = variant.ty;
                quote! {
                    (
                        #tag,
                        <#ty as ::asic_rs_pydantic::PyPydanticType>::pydantic_schema(
                            core_schema,
                            mode,
                        )?,
                    )
                }
            })
            .collect::<Vec<_>>()
    };
    let instance_checks = if options.value_field.is_some() {
        Vec::new()
    } else {
        variants
            .iter()
            .map(|variant| {
                let ident = variant.ident;
                let ty = variant.ty;
                quote! {
                    if value.is_instance_of::<#ty>() {
                        return Ok(Self::#ident(value.extract()?));
                    }
                }
            })
            .collect::<Vec<_>>()
    };
    let parse_matches = if let Some(value_field) = &options.value_field {
        variants
            .iter()
            .map(|variant| {
                let ident = variant.ident;
                let ty = variant.ty;
                let tag = &variant.tag;
                quote! {
                    #tag => {
                        let value = ::asic_rs_pydantic::get_required_field(value, #value_field)?;
                        Ok(Self::#ident(
                            <#ty as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(&value)?,
                        ))
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        variants
            .iter()
            .map(|variant| {
                let ident = variant.ident;
                let ty = variant.ty;
                let tag = &variant.tag;
                quote! {
                    #tag => Ok(Self::#ident(
                        <#ty as ::asic_rs_pydantic::PyPydanticType>::from_pydantic(value)?,
                    )),
                }
            })
            .collect::<Vec<_>>()
    };
    let data_matches = if let Some(value_field) = &options.value_field {
        variants
            .iter()
            .map(|variant| {
                let ident = variant.ident;
                let ty = variant.ty;
                let tag = &variant.tag;
                quote! {
                    Self::#ident(value) => {
                        use ::pyo3::types::PyDictMethods as _;
                        let dict = ::pyo3::types::PyDict::new(py);
                        dict.set_item(#discriminator, #tag)?;
                        dict.set_item(
                            #value_field,
                            <#ty as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(value, py)?,
                        )?;
                        Ok(dict.into_any().unbind())
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        variants
            .iter()
            .map(|variant| {
                let ident = variant.ident;
                let ty = variant.ty;
                quote! {
                    Self::#ident(value) =>
                        <#ty as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(value, py),
                }
            })
            .collect::<Vec<_>>()
    };
    let into_pyobject_impl = if options.value_field.is_none() {
        let into_py_matches = variants.iter().map(|variant| {
            let ident = variant.ident;
            quote! {
                Self::#ident(value) => value.into_pyobject(py).map(::pyo3::Bound::into_any),
            }
        });
        let type_hints = variants.iter().map(|variant| {
            let ty = variant.ty;
            quote! {
                <#ty as ::pyo3::PyTypeInfo>::TYPE_HINT
            }
        });
        Some(quote! {
            #[cfg(feature = "python")]
            impl<'py> ::pyo3::IntoPyObject<'py> for #name #ty_generics #where_clause {
                type Target = ::pyo3::PyAny;
                type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
                type Error = ::pyo3::PyErr;

                const OUTPUT_TYPE: ::pyo3::inspect::PyStaticExpr = {
                    use ::pyo3::type_hint_union;
                    type_hint_union!(#(#type_hints),*)
                };

                fn into_pyobject(
                    self,
                    py: ::pyo3::Python<'py>,
                ) -> Result<Self::Output, Self::Error> {
                    use ::pyo3::IntoPyObject as _;
                    match self {
                        #(#into_py_matches)*
                    }
                }
            }
        })
    } else {
        Some(quote! {
            #[cfg(feature = "python")]
            impl<'py> ::pyo3::IntoPyObject<'py> for #name #ty_generics #where_clause {
                type Target = ::pyo3::PyAny;
                type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
                type Error = ::pyo3::PyErr;

                const OUTPUT_TYPE: ::pyo3::inspect::PyStaticExpr =
                    ::pyo3::type_hint_subscript!(
                        ::pyo3::type_hint_identifier!("builtins", "dict"),
                        ::pyo3::type_hint_identifier!("builtins", "str"),
                        ::pyo3::type_hint_identifier!("builtins", "object")
                    );

                fn into_pyobject(
                    self,
                    py: ::pyo3::Python<'py>,
                ) -> Result<Self::Output, Self::Error> {
                    <Self as ::asic_rs_pydantic::PyPydanticType>::to_pydantic_data(
                        &self,
                        py,
                    )
                    .map(|value| value.into_bound(py))
                }
            }
        })
    };
    Ok(quote! {
        impl #impl_generics ::asic_rs_pydantic::PyPydanticType for #name #ty_generics #where_clause {
            fn pydantic_schema<'py>(
                core_schema: &::pyo3::Bound<'py, ::pyo3::PyAny>,
                mode: ::asic_rs_pydantic::PydanticSchemaMode,
            ) -> ::pyo3::PyResult<::pyo3::Bound<'py, ::pyo3::PyAny>> {
                use ::pyo3::types::PyDictMethods as _;
                ::asic_rs_pydantic::tagged_union_schema(
                    core_schema,
                    [#(#schema_choices),*],
                    #discriminator,
                    Some(#ref_name),
                )
            }

            fn from_pydantic(value: &::pyo3::Bound<'_, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
                use ::pyo3::types::PyAnyMethods as _;
                #(#instance_checks)*
                let tag = ::asic_rs_pydantic::get_required_field(value, #discriminator)?
                    .extract::<String>()?;
                match tag.as_str() {
                    #(#parse_matches)*
                    tag => Err(::pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown {} tag: {tag:?}",
                        #name_str_lit,
                    ))),
                }
            }

            fn to_pydantic_data(
                &self,
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                match self {
                    #(#data_matches)*
                }
            }
        }

        #into_pyobject_impl
    })
}

#[derive(Default)]
struct PydanticDataFieldOptions {
    serializer: PydanticDataFieldSerializer,
}

#[derive(Default)]
enum PydanticDataFieldSerializer {
    #[default]
    Default,
    ToString,
    With(Path),
}

impl PydanticDataFieldOptions {
    fn parse(field: &Field) -> syn::Result<Self> {
        let mut options = Self::default();

        for attr in &field.attrs {
            if !attr.path().is_ident("pydantic_data") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("to_string") {
                    options
                        .set_serializer(PydanticDataFieldSerializer::ToString, meta.path.span())?;
                } else if meta.path.is_ident("with") {
                    let value: LitStr = meta.value()?.parse()?;
                    options.set_serializer(
                        PydanticDataFieldSerializer::With(value.parse()?),
                        meta.path.span(),
                    )?;
                } else {
                    return Err(meta.error("unknown pydantic_data option"));
                }

                Ok(())
            })?;
        }

        Ok(options)
    }

    fn set_serializer(
        &mut self,
        serializer: PydanticDataFieldSerializer,
        span: proc_macro2::Span,
    ) -> syn::Result<()> {
        if !matches!(self.serializer, PydanticDataFieldSerializer::Default) {
            return Err(syn::Error::new(
                span,
                "only one pydantic_data serializer option is supported per field",
            ));
        }
        self.serializer = serializer;
        Ok(())
    }
}

#[proc_macro_derive(PyPydanticData, attributes(pydantic_data))]
pub fn derive_py_pydantic_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_py_pydantic_data(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_py_pydantic_data(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "PyPydanticData requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "PyPydanticData requires a struct",
            ));
        }
    };
    let fields = fields
        .iter()
        .map(expand_pydantic_data_field)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #[cfg(feature = "python")]
        impl #name {
            pub fn to_pydantic_data(
                &self,
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                use ::pyo3::types::{
                    PyAnyMethods as _, PyDict, PyDictMethods as _,
                };

                let dict = PyDict::new(py);
                #(#fields)*
                Ok(dict.into_any().unbind())
            }
        }
    })
}

fn expand_pydantic_data_field(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
    let options = PydanticDataFieldOptions::parse(field)?;
    let key = LitStr::new(&ident.to_string(), ident.span());

    Ok(match options.serializer {
        PydanticDataFieldSerializer::Default => {
            quote! {
                dict.set_item(
                    #key,
                    ::asic_rs_pydantic::PyPydanticType::to_pydantic_data(&self.#ident, py)?,
                )?;
            }
        }
        PydanticDataFieldSerializer::ToString => {
            quote! {
                dict.set_item(#key, self.#ident.to_string())?;
            }
        }
        PydanticDataFieldSerializer::With(path) => {
            quote! {
                dict.set_item(#key, #path(&self.#ident, py)?)?;
            }
        }
    })
}
