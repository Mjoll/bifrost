use proc_macro::TokenStream;
use quote::{
  quote,
  format_ident,
};

#[proc_macro_derive(MemoryMappable)]
pub fn derive_from_raw(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as syn::DeriveInput);

  if let syn::Data::Struct(struct_data) = input.data {
    if let syn::Fields::Named(named_fields) = struct_data.fields {
      return derive_from_fields(input.ident, input.generics, named_fields);
    }
  }

  TokenStream::from(
    syn::Error::new(
      input.ident.span(),
      "Only structs with named fields can derive MemoryMappable"
    ).to_compile_error()
  )
}

fn derive_from_fields(
  struct_name: syn::Ident,
  struct_generics: syn::Generics,
  fields: syn::FieldsNamed
) -> TokenStream {

  let found_crate = proc_macro_crate::crate_name("bifrost").expect("Expected bifrost to be found in Cargo.toml");

  let bifrost_path = match found_crate {
    proc_macro_crate::FoundCrate::Itself => quote!( crate ),
    proc_macro_crate::FoundCrate::Name(name) => {
      let ident : syn::Ident = syn::parse_str(&name).unwrap();
      quote!( ::#ident )
    }
  };

  let mut ref_generics = struct_generics.clone();
  let lifetime_parameter : syn::GenericParam = syn::parse_quote!('t);

  ref_generics.params.insert(0, lifetime_parameter);
  let (impl_generics, ty_generics, _where_clause) = struct_generics.split_for_impl();
  let (ref_impl_generics, ref_ty_generics, _where_clause) = ref_generics.split_for_impl();

  let ref_struct_name = format_ident!("{}Ref", struct_name);
  let ref_field_values = fields.named.iter().map(|field| {
    let name = &field.ident;
    let ty = &field.ty;
    quote!(pub #name: <#ty as #bifrost_path :: MemoryMappable>::Ref<'t>)
  });

  let mut_struct_name = format_ident!("{}Mut", struct_name);
  let mut_field_values = fields.named.iter().map(|field| {
    let name = &field.ident;
    let ty = &field.ty;
    quote!(pub #name: <#ty as #bifrost_path :: MemoryMappable>::Mut<'t>)
  });

  let memory_layout_body = fields.named.iter().map(|field| {
    let name = &field.ident;
    quote!(.with_memory_layout(self.#name.memory_layout()))
  });

  let initialize_body = fields.named.iter().map(|field| {
    let name = &field.ident;
    quote!(let slicer = self.#name.initialize(slicer)?;)
  });

  let map_mut_body_slicing = fields.named.iter().map(|field| {
    let name = &field.ident;
    let ty = &field.ty;
    quote!(let (slicer, #name) = <#ty as #bifrost_path :: MemoryMappable>::map_mut(slicer)?;)
  });

  let map_mut_body_build_struct = fields.named.iter().map(|field| {
    let name = &field.ident;
    quote!(#name,)
  });

  let to_immutable_body = fields.named.iter().map(|field| {
    let name = &field.ident;
    let ty = &field.ty;
    quote!(#name: <#ty as #bifrost_path :: MemoryMappable>::to_immutable(mutable.#name),)
  });

  let reborrow_mut_body = fields.named.iter().map(|field| {
    let name = &field.ident;
    let ty = &field.ty;
    quote!(#name: <#ty as #bifrost_path :: MemoryMappable>::reborrow_mut(&mut mutable.#name),)
  });

  let syn_output =
    quote!(
      pub struct #ref_struct_name #ref_impl_generics {
        #(#ref_field_values,)*
      }

      pub struct #mut_struct_name #ref_impl_generics {
        #(#mut_field_values,)*
      }

      impl #impl_generics #bifrost_path :: MemoryMappable for #struct_name #ty_generics {
        type Ref<'t> = #ref_struct_name #ref_ty_generics;
        type Mut<'t> = #mut_struct_name #ref_ty_generics;

        fn memory_layout(&self) -> #bifrost_path :: MemoryLayout {
          #bifrost_path :: MemoryLayout::new()
          #(#memory_layout_body)*
        }

        fn initialize<'t>(&self, slicer: #bifrost_path :: SlicerMut<'t>) -> Result<#bifrost_path :: SlicerMut<'t>, #bifrost_path :: Error> {
          #(#initialize_body)*
          Ok(slicer)
        }

        fn map_mut<'t>(slicer: #bifrost_path :: SlicerMut<'t>) -> Result<(#bifrost_path :: SlicerMut<'t>, Self::Mut<'t>), #bifrost_path :: Error> {
          #(#map_mut_body_slicing)*
          let mutable = Self::Mut {
            #(#map_mut_body_build_struct)*
          };
          Ok((slicer, mutable))
        }

        fn to_immutable<'t>(mutable: Self::Mut<'t>) -> Self::Ref<'t> {
          #ref_struct_name {
            #(#to_immutable_body)*
          }
        }

        fn reborrow_mut<'b, 'm>(mutable: &'b mut Self::Mut<'m>) -> Self::Mut<'b> {
          #mut_struct_name {
            #(#reborrow_mut_body)*
          }
        }
      }
    );

  // Uncomment to see the generated code during compilation. Useful for
  // debugging. Take the outputted code and paste it in to the source
  // file instead of the derive to use language server to highlight issues.
  //println!("{}", syn_output);

  TokenStream::from(syn_output)
}
