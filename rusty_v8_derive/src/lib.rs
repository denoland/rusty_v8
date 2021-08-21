extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, token::Comma, FnArg,
  GenericArgument, ItemFn, PathArguments, ReturnType, Type, TypeTuple,
};

/// ```rust
/// #[magic]
/// fn fast(a: u32, b: u32) -> u32 {
///     a + b
/// }
/// ```
#[proc_macro_attribute]
pub fn magic(_args: TokenStream, item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as ItemFn);

  let fn_name = input.sig.ident.clone();
  let fn_args = input.sig.inputs.clone();
  let fn_body = input.block.clone();

  let fn_return = match input.sig.output {
    ReturnType::Default => Type::Tuple(TypeTuple {
      paren_token: Default::default(),
      elems: Default::default(),
    }),
    ReturnType::Type(_, t) => (*t).clone(),
  };

  let fn_arg_types: Punctuated<_, Comma> = fn_args
    .iter()
    .map(|a| match a {
      FnArg::Typed(p) => *p.ty.clone(),
      _ => unreachable!(),
    })
    .collect();

  let fn_type = quote! { fn(#fn_arg_types) -> #fn_return };

  let map_type = |t: &Type| match t {
    Type::Tuple(t) if t.elems.len() == 0 => {
      quote! { ::rusty_v8::V8CType::Void }
    }
    Type::Path(p) => {
      let is_v8_value = {
        let segment = p.path.segments.iter().last().unwrap();
        segment.ident == "Local"
          && match segment.arguments {
            PathArguments::AngleBracketed(ref args) => {
              match args.args.iter().last().unwrap() {
                GenericArgument::Type(Type::Path(p)) => {
                  p.path.segments.iter().last().unwrap().ident == "Value"
                }
                _ => false,
              }
            }
            _ => false,
          }
      };
      if is_v8_value {
        quote! { ::rusty_v8::V8CType::V8Value }
      } else {
        let ty = p.path.get_ident().unwrap().to_string();
        match ty.as_str() {
          "bool" => quote! { ::rusty_v8::V8CType::Bool },
          "u32" => quote! { ::rusty_v8::V8CType::Uint32 },
          "i32" => quote! { ::rusty_v8::V8CType::Int32 },
          "u64" => quote! { ::rusty_v8::V8CType::Uint64 },
          "i64" => quote! { ::rusty_v8::V8CType::Int64 },
          "f32" => quote! { ::rusty_v8::V8CType::Float32 },
          "f64" => quote! { ::rusty_v8::V8CType::Float64 },
          _ => panic!("Unsupported type: {}", ty),
        }
      }
    }
    _ => panic!("Unsupported type: {:?}", t),
  };

  let argument_info: Punctuated<_, Comma> =
    fn_arg_types.iter().map(map_type).collect();
  let return_info = map_type(&fn_return);

  let output = quote! {
      #[allow(non_camel_case_types)]
      struct #fn_name;

      impl ::core::ops::Deref for #fn_name {
          type Target = #fn_type;

          fn deref(&self) -> &#fn_type {
              const FN: #fn_type = |#fn_args| { #fn_body };
              return &FN;
          }
      }

      impl ::rusty_v8::FastFunctionInfo for #fn_name {
          fn signature(&self) -> (&'static [::rusty_v8::V8CType], ::rusty_v8::V8CType) {
              const ARGUMENT_INFO: &'static [::rusty_v8::V8CType] = &[#argument_info];
              const RETURN_INFO: ::rusty_v8::V8CType = #return_info;

              return (ARGUMENT_INFO, RETURN_INFO);
          }

          fn function(&self) -> *const ::core::ffi::c_void {
              *::core::ops::Deref::deref(self) as _
          }
      }
  };

  TokenStream::from(output)
}
