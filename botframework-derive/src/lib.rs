use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr, Meta, Type};

/// Derive macro for generating ToolParameters implementation from struct fields.
///
/// Each field must have a #[description("...")] attribute.
///
/// Supported types:
/// - &str, String -> "string" type
/// - i64, i32, u64, u32, i16, u16, i8, u8, isize, usize -> "integer" type
/// - f64, f32 -> "number" type
/// - Enums that derive ToolParameters -> "string" type with "enum" field containing variant names
///
/// For enums:
/// - Each variant can have #[serde(rename = "...")] to specify the string value
/// - Generates a static array of variant names accessible via ToolParameters::parameters()
///
/// Example:
/// ```rust
/// #[derive(ToolParameters)]
/// struct CallArgs<'a> {
///     #[description("Assembly minutes text")]
///     minutes: &'a str,
///     #[description("Number of days")]
///     days: i64,
/// }
///
/// #[derive(ToolParameters)]
/// enum Status {
///     #[serde(rename = "active")]
///     Active,
///     #[serde(rename = "inactive")]
///     Inactive,
/// }
/// ```
#[proc_macro_derive(ToolParameters, attributes(description))]
pub fn derive_tool_parameters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match &input.data {
        Data::Struct(data) => {
            let fields = match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => panic!("ToolParameters requires named fields for structs"),
            };

            let properties: Vec<_> = fields
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();

                    // Get description from attribute
                    let mut description = None;
                    for attr in &field.attrs {
                        if attr.path().is_ident("description") {
                            // Handle #[description("...")] as Meta::List
                            if let Meta::List(list) = &attr.meta {
                                let lit: LitStr = list
                                    .parse_args()
                                    .expect("description must be a string literal");
                                description = Some(lit.value());
                            }
                        }
                    }

                    let desc = description.unwrap_or_else(|| {
                        panic!("Field `{}` must have #[description(\"...\")]", field_name)
                    });

                    // Determine type from Rust type
                    let ty = &field.ty;

                    if is_string_type(ty) {
                        quote! {
                            botframework::telegram::Property::string(#field_name_str, #desc)
                        }
                    } else if is_integer_type(ty) {
                        quote! {
                            botframework::telegram::Property::integer(#field_name_str, #desc)
                        }
                    } else if is_number_type(ty) {
                        quote! {
                            botframework::telegram::Property::number(#field_name_str, #desc)
                        }

                    }else if is_bool_type(ty){
                        quote!{botframework::telegram::Property::boolean(#field_name_str, #desc)}
                    } else if is_vec_type(ty) {
                        // For Vec<T>, generate array type property
                        // Note: Vec support is simplified - items type info would require more complex handling
                        quote! {
                            botframework::telegram::Property::string(#field_name_str, #desc)
                        }
                    } else if is_enum_type(ty) {
                        // For enum types, use the type's own parameters() to get enum values
                        quote! {
                            {
                                let enum_props = <#ty as botframework::telegram::ToolParameters>::parameters();
                                if let Some(first) = enum_props.first() {
                                    if let Some(values) = first.kind.enum_values() {
                                        botframework::telegram::Property::string_enum(#field_name_str, #desc, values)
                                    } else {
                                        botframework::telegram::Property::string(#field_name_str, #desc)
                                    }
                                } else {
                                    botframework::telegram::Property::string(#field_name_str, #desc)
                                }
                            }
                        }
                    } else {
                        panic!(
                            "Unsupported type for field `{field_name}` of type {:?}",
                            quote!(#ty).to_string()
                        )
                    }
                })
                .collect();

            let expanded = quote! {
                impl #impl_generics botframework::telegram::ToolParameters for #name #ty_generics #where_clause {
                    fn parameters() -> botframework::telegram::Properties {
                        vec![#(#properties),*]
                    }
                }
            };

            TokenStream::from(expanded)
        }
        Data::Enum(data) => {
            // Collect variant names, respecting #[serde(rename = "...")] attributes
            let variant_names: Vec<_> = data
                .variants
                .iter()
                .map(|variant| {
                    let mut name_str = variant.ident.to_string();

                    // Check for #[serde(rename = "...")] attribute
                    for attr in &variant.attrs {
                        if attr.path().is_ident("serde") {
                            if let Meta::List(list) = &attr.meta {
                                // Parse the content: rename = "value"
                                let content = list.tokens.to_string();
                                // Simple parsing to extract rename value
                                if let Some(start) = content.find("rename") {
                                    let after_rename = &content[start + 6..];
                                    if let Some(eq_pos) = after_rename.find('=') {
                                        let after_eq = &after_rename[eq_pos + 1..].trim();
                                        if let Some(quote_start) = after_eq.find('"') {
                                            let after_quote = &after_eq[quote_start + 1..];
                                            if let Some(quote_end) = after_quote.find('"') {
                                                name_str = after_quote[..quote_end].to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    name_str
                })
                .collect();

            // Create a static array of the variant names
            let variant_name_strs: Vec<_> = variant_names.iter().map(|n| quote!(#n)).collect();

            // For enums, parameters() returns a single Property describing the enum values
            let expanded = quote! {
                impl botframework::telegram::ToolParameters for #name {
                    fn parameters() -> botframework::telegram::Properties {
                        // Static array of enum variant names
                        static VARIANT_NAMES: &[&str] = &[#(#variant_name_strs),*];

                        vec![botframework::telegram::Property {
                            kind: botframework::telegram::PropertyKind::Enum(VARIANT_NAMES),
                            name: stringify!(#name),
                            description: "",
                        }]
                    }
                }
            };

            TokenStream::from(expanded)
        }
        Data::Union(_) => panic!("ToolParameters cannot be derived for unions"),
    }
}

fn is_string_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    matches!(
        type_str.as_str(),
        "& 'a str" | "&str" | "String" | "std :: string :: String" | "Option < & 'a str >"
    )
}

fn is_integer_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    matches!(
        type_str.as_str(),
        "i64" | "i32" | "u64" | "u32" | "i16" | "u16" | "i8" | "u8" | "isize" | "usize"
    )
}

fn is_number_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    matches!(type_str.as_str(), "f64" | "f32")
}

fn is_vec_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    type_str.starts_with("Vec <") || type_str.starts_with("std :: vec :: Vec <")
}

fn is_bool_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    type_str.starts_with("bool")
}

fn is_enum_type(ty: &Type) -> bool {
    // Check if the type implements ToolParameters by looking for capitalized type names
    // (simple heuristic - proper check would require type resolution)
    let type_str = quote!(#ty).to_string();
    // Check if it looks like a custom type (not a primitive)
    !is_string_type(ty)
        && !is_integer_type(ty)
        && !is_number_type(ty)
        && !type_str.starts_with("Option <")
        && !type_str.starts_with("Vec <")
        && !type_str.starts_with("&")
}
