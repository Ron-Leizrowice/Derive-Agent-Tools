extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataStruct, DeriveInput, Fields, LitStr, Type,
};

// Macro entry points -------------------------------------------------------
//
// `derive_agent_tool` drives the bulk of the code generation. The macro keeps
// the expansion self-contained so downstream crates only need the
// `derive_agent_tools` facade crate.

#[proc_macro_derive(AgentTool, attributes(tool))]
pub fn derive_agent_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_agent_tool(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(AgentToolParameter, attributes(tool))]
pub fn derive_agent_tool_parameter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_agent_tool_parameter(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_agent_tool(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = &input.data
    else {
        return Err(syn::Error::new(
            input.span(),
            "AgentTool can only be derived for structs with named fields",
        ));
    };

    // Parse struct-level attributes: name, description
    let mut tool_name: Option<String> = None;
    let mut tool_description: Option<String> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("tool") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let lit: LitStr = meta.value()?.parse()?;
                tool_name = Some(lit.value());
                Ok(())
            } else if meta.path.is_ident("description") {
                let lit: LitStr = meta.value()?.parse()?;
                tool_description = Some(lit.value());
                Ok(())
            } else {
                Ok(())
            }
        })?;
    }

    let computed_tool_name = tool_name.unwrap_or_else(|| ident.to_string());
    let description_tokens = if let Some(desc) = tool_description {
        quote! { Some(#desc) }
    } else {
        quote! { None::<&'static str> }
    };

    // Per-field metadata
    struct FieldMeta {
        name: String,
        description: Option<String>,
        required: bool,
        json_type: String,
        items_type: Option<String>,
    }

    let mut field_metas: Vec<FieldMeta> = Vec::new();
    for field in fields.named.iter() {
        let Some(field_ident) = &field.ident else {
            continue;
        };
        let field_name = field_ident.to_string();

        let mut required = false;
        let mut description: Option<String> = None;
        for attr in &field.attrs {
            if !attr.path().is_ident("tool") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("required") {
                    required = true;
                    Ok(())
                } else if meta.path.is_ident("description") {
                    let lit: LitStr = meta.value()?.parse()?;
                    description = Some(lit.value());
                    Ok(())
                } else {
                    Ok(())
                }
            })?;
        }

        let (json_type, items_type) = infer_json_type(&field.ty);

        field_metas.push(FieldMeta {
            name: field_name,
            description,
            required,
            json_type,
            items_type,
        });
    }

    // Build JSON schema entries (serde_json) and Document schema entries (Bedrock)
    let json_props_entries = field_metas.iter().map(|fm| {
        let name = &fm.name;
        let typ = &fm.json_type;
        let desc_tokens = fm
            .description
            .as_ref()
            .map(|d| quote! { Some(#d) })
            .unwrap_or_else(|| quote! { None::<&str> });
        let items_clause = if let Some(items_t) = &fm.items_type {
            quote! {
                map.insert(
                    "items".to_string(),
                    ::derive_agent_tools::__macro_support::serde_json::json!({ "type": #items_t })
                );
            }
        } else {
            quote! {}
        };

        quote! {
            let mut map = ::derive_agent_tools::__macro_support::serde_json::Map::<String, ::derive_agent_tools::__macro_support::serde_json::Value>::new();
            map.insert("type".to_string(), ::derive_agent_tools::__macro_support::serde_json::Value::String(#typ.to_string()));
            if let Some(desc) = #desc_tokens { map.insert("description".to_string(), ::derive_agent_tools::__macro_support::serde_json::Value::String(desc.to_string())); }
            #items_clause
            props.insert(#name.to_string(), ::derive_agent_tools::__macro_support::serde_json::Value::Object(map));
        }
    });

    let doc_props_entries = field_metas.iter().map(|fm| {
        let name = &fm.name;
        let typ = &fm.json_type;
        let desc_tokens = fm
            .description
            .as_ref()
            .map(|d| quote! { Some(#d) })
            .unwrap_or_else(|| quote! { None::<&str> });
        let items_clause = if let Some(items_t) = &fm.items_type {
            quote! {
                let mut items_map = ::std::collections::HashMap::<::std::string::String, ::derive_agent_tools::__macro_support::aws_smithy_types::Document>::new();
                items_map.insert(
                    "type".to_string(),
                    ::derive_agent_tools::__macro_support::aws_smithy_types::Document::String(#items_t.to_string())
                );
                map.insert(
                    "items".to_string(),
                    ::derive_agent_tools::__macro_support::aws_smithy_types::Document::Object(items_map)
                );
            }
        } else {
            quote! {}
        };
        quote! {
            let mut map = ::std::collections::HashMap::<::std::string::String, ::derive_agent_tools::__macro_support::aws_smithy_types::Document>::new();
            map.insert("type".to_string(), ::derive_agent_tools::__macro_support::aws_smithy_types::Document::String(#typ.to_string()));
            if let Some(desc) = #desc_tokens { map.insert("description".to_string(), ::derive_agent_tools::__macro_support::aws_smithy_types::Document::String(desc.to_string())); }
            #items_clause
            props.insert(#name.to_string(), ::derive_agent_tools::__macro_support::aws_smithy_types::Document::Object(map));
        }
    });

    let required_fields: Vec<syn::LitStr> = field_metas
        .iter()
        .filter(|fm| fm.required)
        .map(|fm| syn::LitStr::new(&fm.name, proc_macro2::Span::call_site()))
        .collect();

    let json_required_section = if required_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            let required = vec![ #( #required_fields.to_string() ),* ];
            schema.insert(
                "required".to_string(),
                ::derive_agent_tools::__macro_support::serde_json::json!(required)
            );
        }
    };

    let doc_required_section = if required_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            let required: ::std::vec::Vec<_> = vec![
                #( ::derive_agent_tools::__macro_support::aws_smithy_types::Document::String(#required_fields.to_string()) ),*
            ];
            schema.insert(
                "required".to_string(),
                ::derive_agent_tools::__macro_support::aws_smithy_types::Document::Array(required)
            );
        }
    };

    // Implementations
    let tool_impl = quote! {
        impl #ident {
            const __AGENT_TOOL_NAME: &'static str = #computed_tool_name;
            const __AGENT_TOOL_DESCRIPTION: Option<&'static str> = #description_tokens;

            /// Returns the logical name of this tool.
            pub fn tool_name() -> &'static str {
                Self::__AGENT_TOOL_NAME
            }

            /// Returns the JSON Schema for this tool's input in serde_json::Value form.
            #[cfg(feature = "serde-json")]
            pub fn tool_schema_json() -> ::derive_agent_tools::__macro_support::serde_json::Value {
                let mut props = ::derive_agent_tools::__macro_support::serde_json::Map::<String, ::derive_agent_tools::__macro_support::serde_json::Value>::new();
                #( #json_props_entries ; )*
                let mut schema = ::derive_agent_tools::__macro_support::serde_json::Map::<String, ::derive_agent_tools::__macro_support::serde_json::Value>::new();
                schema.insert("type".to_string(), ::derive_agent_tools::__macro_support::serde_json::json!("object"));
                schema.insert("properties".to_string(), ::derive_agent_tools::__macro_support::serde_json::Value::Object(props));
                #json_required_section
                ::derive_agent_tools::__macro_support::serde_json::Value::Object(schema)
            }

            /// Builds an AWS Bedrock ToolSpecification for this tool based on the schema.
            #[cfg(feature = "bedrock")]
            pub fn tool_spec() -> ::derive_agent_tools::__macro_support::aws_sdk_bedrockruntime::types::ToolSpecification {
                let mut props = ::std::collections::HashMap::<::std::string::String, ::derive_agent_tools::__macro_support::aws_smithy_types::Document>::new();
                #( #doc_props_entries ; )*
                let mut schema = ::std::collections::HashMap::<::std::string::String, ::derive_agent_tools::__macro_support::aws_smithy_types::Document>::new();
                schema.insert(
                    "type".to_string(),
                    ::derive_agent_tools::__macro_support::aws_smithy_types::Document::String("object".to_string())
                );
                schema.insert(
                    "properties".to_string(),
                    ::derive_agent_tools::__macro_support::aws_smithy_types::Document::Object(props)
                );
                #doc_required_section

                let input_schema = ::derive_agent_tools::__macro_support::aws_sdk_bedrockruntime::types::ToolInputSchema::Json(
                    ::derive_agent_tools::__macro_support::aws_smithy_types::Document::Object(schema)
                );

                ::derive_agent_tools::__macro_support::aws_sdk_bedrockruntime::types::ToolSpecification::builder()
                    .name(Self::__AGENT_TOOL_NAME)
                    .set_description(Self::__AGENT_TOOL_DESCRIPTION.map(|s| s.to_string()))
                    .input_schema(input_schema)
                    .build()
                    .expect("valid ToolSpecification")
            }
        }
    };

    // Implement TryFrom<&Document> using serde conversion path
    let err_ident = format_ident!("{}AgentToolParseError", ident);
    let try_from_impl = quote! {
        #[cfg(all(feature = "bedrock", feature = "serde-json"))]
        #[derive(Debug, Clone)]
        pub struct #err_ident(pub ::std::string::String);
        #[cfg(all(feature = "bedrock", feature = "serde-json"))]
        impl ::std::fmt::Display for #err_ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        #[cfg(all(feature = "bedrock", feature = "serde-json"))]
        impl ::std::error::Error for #err_ident {}

        #[cfg(all(feature = "bedrock", feature = "serde-json"))]
        impl<'a> ::std::convert::TryFrom<&'a ::derive_agent_tools::__macro_support::aws_smithy_types::Document> for #ident {
            type Error = #err_ident;
            fn try_from(doc: &'a ::derive_agent_tools::__macro_support::aws_smithy_types::Document) -> ::std::result::Result<Self, Self::Error> {
                fn doc_to_json(doc: &::derive_agent_tools::__macro_support::aws_smithy_types::Document) -> ::std::result::Result<::derive_agent_tools::__macro_support::serde_json::Value, ::std::string::String> {
                    use ::derive_agent_tools::__macro_support::aws_smithy_types::Document as D;
                    match doc {
                        D::Null => Ok(::derive_agent_tools::__macro_support::serde_json::Value::Null),
                        D::Bool(b) => Ok(::derive_agent_tools::__macro_support::serde_json::Value::Bool(*b)),
                        D::String(s) => Ok(::derive_agent_tools::__macro_support::serde_json::Value::String(s.clone())),
                        D::Number(n) => {
                            use ::derive_agent_tools::__macro_support::serde_json::Number as JNum;
                            let jn = match *n {
                                ::derive_agent_tools::__macro_support::aws_smithy_types::Number::PosInt(u) => JNum::from(u),
                                ::derive_agent_tools::__macro_support::aws_smithy_types::Number::NegInt(i) => JNum::from(i),
                                ::derive_agent_tools::__macro_support::aws_smithy_types::Number::Float(f) => JNum::from_f64(f).ok_or_else(|| "invalid f64 value in Document::Number".to_string())?,
                            };
                            Ok(::derive_agent_tools::__macro_support::serde_json::Value::Number(jn))
                        }
                        D::Array(arr) => {
                            let mut out = ::std::vec::Vec::with_capacity(arr.len());
                            for v in arr.iter() { out.push(doc_to_json(v)?); }
                            Ok(::derive_agent_tools::__macro_support::serde_json::Value::Array(out))
                        }
                        D::Object(map) => {
                            let mut m = ::derive_agent_tools::__macro_support::serde_json::Map::new();
                            for (k, v) in map.iter() { m.insert(k.clone(), doc_to_json(v)?); }
                            Ok(::derive_agent_tools::__macro_support::serde_json::Value::Object(m))
                        }
                    }
                }
                let json = doc_to_json(doc).map_err(#err_ident)?;
                let obj: Self = ::derive_agent_tools::__macro_support::serde_json::from_value(json).map_err(|e| #err_ident(e.to_string()))?;
                Ok(obj)
            }
        }
    };

    Ok(quote! {
        #tool_impl
        #try_from_impl
    })
}

fn impl_agent_tool_parameter(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    Ok(quote! {
        impl #ident {
            #[cfg(feature = "serde-json")]
            pub fn parameter_schema_json() -> ::derive_agent_tools::__macro_support::serde_json::Value {
                ::derive_agent_tools::__macro_support::serde_json::json!({ "type": "object" })
            }
        }
    })
}

fn infer_json_type(ty: &Type) -> (String, Option<String>) {
    if let Some(inner) = extract_generic(ty, "Option") {
        return infer_json_type(&inner);
    }
    if let Some(inner) = extract_generic(ty, "Vec") {
        let (inner_ty, _) = infer_json_type(&inner);
        return ("array".to_string(), Some(inner_ty));
    }

    match ty_to_ident(ty).as_deref() {
        Some("bool") => ("boolean".to_string(), None),
        Some("i8") | Some("i16") | Some("i32") | Some("i64") | Some("isize") | Some("u8")
        | Some("u16") | Some("u32") | Some("u64") | Some("usize") => ("integer".to_string(), None),
        Some("f32") | Some("f64") => ("number".to_string(), None),
        Some("String") | Some("&str") => ("string".to_string(), None),
        _ => ("object".to_string(), None),
    }
}

fn ty_to_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Type::Reference(r) => ty_to_ident(&r.elem),
        _ => None,
    }
}

fn extract_generic(ty: &Type, ident: &str) -> Option<Type> {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if seg.ident == ident {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}
