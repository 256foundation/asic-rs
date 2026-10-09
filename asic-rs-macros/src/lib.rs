use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Path, parse_macro_input, spanned::Spanned};

const HASH_ALGORITHM_VARIANTS: &[&str] = &[
    "SHA256",
    "Scrypt",
    "X11",
    "Blake2S256",
    "Blake2b",
    "Kadena",
    "KHeavyHash",
    "Eaglesong",
    "EtHash",
    "Equihash",
    "Handshake",
    "Blake256R14",
    "Unknown",
];

/// Implements `MinerModelAlgorithm` from typed per-variant algorithm attributes.
///
/// Every variant must declare exactly one algorithm:
///
/// ```ignore
/// #[algorithm(HashAlgorithm::SHA256)]
/// S21,
/// ```
///
/// Missing attributes fail during macro expansion. Invalid paths or values fail
/// type checking because every match arm must return `HashAlgorithm`.
#[proc_macro_derive(ModelAlgorithm, attributes(algorithm))]
pub fn derive_model_algorithm(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_model_algorithm(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_model_algorithm(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(Error::new(
                input.span(),
                "ModelAlgorithm can only be derived for enums",
            ));
        }
    };

    let mut arms = Vec::with_capacity(enum_data.variants.len());
    for variant in &enum_data.variants {
        let attributes = variant
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("algorithm"))
            .collect::<Vec<_>>();

        let attribute = match attributes.as_slice() {
            [] => {
                return Err(Error::new(
                    variant.span(),
                    "missing #[algorithm(HashAlgorithm::Variant)] attribute",
                ));
            }
            [attribute] => attribute,
            _ => {
                return Err(Error::new(
                    variant.span(),
                    "only one #[algorithm(...)] attribute is allowed per variant",
                ));
            }
        };
        let algorithm = attribute.parse_args::<Path>()?;
        let Some(algorithm_variant) = algorithm.segments.last() else {
            return Err(Error::new(
                algorithm.span(),
                "algorithm path cannot be empty",
            ));
        };
        if !HASH_ALGORITHM_VARIANTS.contains(&algorithm_variant.ident.to_string().as_str()) {
            return Err(Error::new(
                algorithm_variant.ident.span(),
                format!(
                    "unknown HashAlgorithm variant `{}`",
                    algorithm_variant.ident
                ),
            ));
        }

        let variant_name = &variant.ident;
        let pattern = match &variant.fields {
            Fields::Unit => quote!(Self::#variant_name),
            Fields::Unnamed(_) => quote!(Self::#variant_name(..)),
            Fields::Named(_) => quote!(Self::#variant_name { .. }),
        };
        arms.push(quote!(#pattern => #algorithm));
    }

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::asic_rs_core::traits::model::MinerModelAlgorithm
            for #name #type_generics #where_clause
        {
            fn hash_algorithm(&self) -> ::asic_rs_core::data::device::HashAlgorithm {
                match self {
                    #(#arms,)*
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn accepts_typed_algorithm_paths() {
        let input: DeriveInput = parse_quote! {
            enum Model {
                #[algorithm(HashAlgorithm::SHA256)]
                Known,
                #[algorithm(HashAlgorithm::Unknown)]
                Unknown(String),
            }
        };

        assert!(expand_model_algorithm(&input).is_ok());
    }

    #[test]
    fn rejects_missing_algorithm() {
        let input: DeriveInput = parse_quote! {
            enum Model {
                Missing,
            }
        };

        assert!(expand_model_algorithm(&input).is_err());
    }

    #[test]
    fn rejects_duplicate_algorithms() {
        let input: DeriveInput = parse_quote! {
            enum Model {
                #[algorithm(HashAlgorithm::SHA256)]
                #[algorithm(HashAlgorithm::Scrypt)]
                Duplicate,
            }
        };

        assert!(expand_model_algorithm(&input).is_err());
    }

    #[test]
    fn rejects_unknown_algorithm_variants() {
        let input: DeriveInput = parse_quote! {
            enum Model {
                #[algorithm(HashAlgorithm::NotAnAlgorithm)]
                Invalid,
            }
        };

        let error = expand_model_algorithm(&input).expect_err("unknown algorithm must fail");
        assert_eq!(
            error.to_string(),
            "unknown HashAlgorithm variant `NotAnAlgorithm`"
        );
    }
}
