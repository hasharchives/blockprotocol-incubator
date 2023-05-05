use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{Lifetime, Visibility};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    PropertyTypeReference, ValueOrArray,
};

use crate::{
    analysis::EdgeKind,
    data,
    name::{Location, LocationKind, NameResolver, PropertyName},
};

pub(crate) enum PropertyKind {
    Array,
    Plain,
    Boxed,
}

pub(crate) struct Property {
    pub(crate) name: Ident,
    pub(crate) type_: Ident,

    pub(crate) kind: PropertyKind,

    pub(crate) required: bool,
}

pub(crate) fn properties<'a>(
    id: &VersionedUrl,
    properties: &'a HashMap<BaseUrl, ValueOrArray<PropertyTypeReference>>,
    required: &[BaseUrl],
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> BTreeMap<&'a BaseUrl, Property> {
    properties
        .iter()
        .map(|(base, value)| {
            let url = match value {
                ValueOrArray::Value(value) => value.url(),
                ValueOrArray::Array(value) => value.items().url(),
            };

            let name = Ident::new(&property_names[url].0, Span::call_site());
            let location = &locations[url];

            let type_ = location
                .alias
                .value
                .as_ref()
                .unwrap_or(&location.name.value);
            let type_ = Ident::new(type_, Span::call_site());

            let required = required.contains(base);

            let kind = if matches!(value, ValueOrArray::Array(_)) {
                PropertyKind::Array
            } else if resolver.analyzer().edge(id, url).kind == EdgeKind::Boxed {
                PropertyKind::Boxed
            } else {
                PropertyKind::Plain
            };

            (base, Property {
                name,
                type_,
                kind,
                required,
            })
        })
        .collect()
}

pub(crate) fn imports<'a>(
    references: impl IntoIterator<Item = &'a &'a VersionedUrl> + 'a,
    locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,
) -> impl Iterator<Item = TokenStream> + 'a {
    // explicit type not needed here, but CLion otherwise complains

    references.into_iter().map(|reference: &&VersionedUrl| {
        let location = &locations[reference];

        // shortcut for builtin data-types as they are handled in a special way
        if let Some(builtin) = data::find_builtin(reference) {
            let mut tokens = builtin.to_token_stream();

            if let Some(alias) = &location.alias.value {
                let alias = Ident::new(alias, Span::call_site());

                tokens = quote!(#tokens as #alias);
            }

            return quote!(#tokens;);
        }

        let mut path: Vec<_> = location
            .path
            .directories()
            .iter()
            .map(|directory| Ident::new(directory.name(), Span::call_site()))
            .collect();

        // only add to path if we're not a mod.rs file, otherwise it will lead to import errors
        if !location.path.file().is_mod() {
            path.push(Ident::new(location.path.file().name(), Span::call_site()));
        }

        let mut name = Ident::new(&location.name.value, Span::call_site()).to_token_stream();

        if let Some(alias) = &location.alias.value {
            let alias = Ident::new(alias, Span::call_site());
            name = quote!(#name as #alias);
        }

        quote! {
            use crate #(:: #path)* :: #name;
        }
    })
}

pub(crate) fn generate_mod(kind: &LocationKind, resolver: &NameResolver) -> Option<TokenStream> {
    let LocationKind::Latest {other} = kind else {
        return None;
    };

    let statements = other.iter().map(|url| {
        let location = resolver.location(url);
        let file = Ident::new(location.path.file().name(), Span::call_site());

        let name = Ident::new(&location.name.value, Span::call_site());

        // we do not surface the ref or mut variants, this is intentional, as they
        // should be accessed through `::Ref` and `::Mut` instead!
        // TODO: rethink this strategy!

        // optional aliases
        let name_alias = location.name.alias.as_ref().map(|alias| {
            let alias = Ident::new(alias, Span::call_site());
            quote!(pub use #file::#alias;)
        });

        quote! {
            pub mod #file;
            pub use #file::#name;
            #name_alias
        }
    });

    Some(quote!(#(#statements)*))
}

pub(crate) enum IncludeLifetime {
    Yes,
    No,
}

impl IncludeLifetime {
    fn into_bool(self) -> bool {
        match self {
            Self::Yes => true,
            Self::No => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Variant {
    Owned,
    Ref,
    Mut,
}

impl Variant {
    pub(crate) fn into_reference(self, with_lifetime: IncludeLifetime) -> Option<TokenStream> {
        let lifetime = with_lifetime
            .into_bool()
            .then(|| self.into_lifetime())
            .flatten();

        match self {
            Self::Owned => None,
            Self::Ref => Some(quote!(& #lifetime)),
            Self::Mut => Some(quote!(& #lifetime mut)),
        }
    }

    pub(crate) fn into_lifetime(self) -> Option<Lifetime> {
        match self {
            Self::Owned => None,
            Self::Ref | Self::Mut => Some(Lifetime::new("'a", Span::call_site())),
        }
    }

    fn into_keyword(self) -> Option<TokenStream> {
        match self {
            Self::Owned => None,
            Self::Ref => Some(quote!(ref)),
            Self::Mut => Some(quote!(mut)),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Import {
    pub(crate) vec: bool,
    pub(crate) box_: bool,
    pub(crate) phantom_data: bool,
}

fn generate_fold(properties: &BTreeMap<&BaseUrl, Property>) -> TokenStream {
    let mut fold = vec![];
    let mut unfold = vec![];

    let mut chunks = properties
        .values()
        .map(|Property { name, .. }| name)
        .array_chunks::<16>();

    let mut index = 0;
    // in theory we could merge even more, currently "just" 16 fields are batched together
    for chunk in &mut chunks {
        let result = format_ident!("__report{index}");
        fold.push(quote!(let #result = turbine::fold_tuple_reports((#(#chunk,)*))));
        unfold.push((quote!((#(#chunk,)*)), result));
        index += 1;
    }

    if let Some(remainder) = chunks.into_remainder() {
        let result = format_ident!("__report{index}");
        let chunk: Vec<_> = remainder.collect();

        fold.push(quote!(let #result = turbine::fold_tuple_reports((#(#chunk,)*))));
        unfold.push((quote!((#(#chunk,)*)), result));
    }

    // this creates an implicit limit of 16*16 elements (~> 256 element)
    // o god, the monomorphised code must be huge D:
    let (unfold_lhs, unfold_rhs): (Vec<_>, Vec<_>) = unfold.into_iter().unzip();

    quote! {
        #(#fold;)*

        let (#(#unfold_lhs,)*) = turbine::fold_tuple_reports((#(#unfold_rhs,)*))?;
    }
}

pub(crate) fn generate_properties_try_from_value(
    variant: Variant,
    properties: &BTreeMap<&BaseUrl, Property>,
    error: &Ident,
    type_: &TokenStream,
) -> TokenStream {
    // fundamentally we have 3 phases:
    // 1) get all values (as Result)
    // 2) merge them together using `turbine::fold_tuple_reports`
    // 3) merge all values together

    // makes use of labelled breaks in blocks (introduced in 1.65)
    let values = properties.iter().map(
        |(
            base,
            Property {
                name,
                type_,
                kind,
                required,
            },
        )| {
            let index = base.as_str();

            let type_ = match variant {
                Variant::Owned => type_.to_token_stream(),
                Variant::Ref => quote!(<#type_ as Type>::Ref<'a>),
                Variant::Mut => quote!(<#type_ as Type>::Mut<'a>),
            };

            // TODO: keep mutable reference on entity as safeguard
            let access = match variant {
                Variant::Owned => quote!(let value = properties.remove(#index);),
                Variant::Ref => quote!(let value = properties.get(#index);),
                Variant::Mut => quote! {
                    // Note: This is super sketch
                    // SAFETY: We already have &mut access, meaning that no one else has mut access
                    //  the urls are unique and are not colliding, meaning that there's no overlap
                    //  and we always have a single mutable reference to each value.
                    //  In theory whenever a new value is added or removed the reference could get
                    //  invalidated, to circumvent this `EntityMut` variant is holding a mutable
                    //  reference.
                    //  Heavy inspiration has been taken from https://stackoverflow.com/a/53146512/9077988
                    //  THIS IS CURRENTLY UNTESTED (god I am scared)
                    //  This is very similar to `get_mut_many`, but we need to know if a value
                    //  does not exist and if it doesn't exist which, we would also need to convert
                    //  serde_json HashMap into a hashbrown::HashMap for it to work.
                    let value = unsafe {
                        let value = properties.get_mut(#index);
                        let value = value.map(|value| value as *mut _);

                        value.map(|value: *mut serde_json::Value| &mut *value)
                    };
                },
            };

            let unwrap = if *required {
                quote! {
                    let Some(value) = value else {
                        break 'property Err(Report::new(#error::ExpectedProperty(#index)));
                    };
                }
            } else {
                // the value is wrapped in `Option<>` and can be missing!
                quote! {
                    let Some(value) = value else {
                        break 'property Ok(None);
                    };

                    if value.is_null() {
                        break 'property Ok(None)
                    };
                }
            };

            let apply = match kind {
                PropertyKind::Array => {
                    let suffix = match variant {
                        Variant::Ref => Some(quote!(.map(|array| array.into_boxed_slice()))),
                        _ => None,
                    };

                    quote! {
                        let value = if let serde_json::Value::Array(value) = value {
                            turbine::fold_iter_reports(
                                value.into_iter().map(|value| <#type_>::try_from_value(value))
                            )
                                #suffix
                                .change_context(#error::Property(#index)
                            )
                        } else {
                            Err(Report::new(#error::ExpectedArray(#index)))
                        };
                    }
                }
                PropertyKind::Plain => quote! {
                    let value = <#type_>::try_from_value(value)
                        .change_context(#error::Property(#index));
                },
                PropertyKind::Boxed => quote! {
                    let value = <#type_>::try_from_value(value)
                        .map(Box::new)
                        .change_context(#error::Property(#index));
                },
            };

            let ret = if *required {
                quote!(value)
            } else {
                quote!(value.map(Some))
            };

            quote! {
                let #name = 'property: {
                    #access

                    #unwrap

                    #apply

                    #ret
                };
            }
        },
    );

    let fold = generate_fold(properties);

    let fields = properties
        .values()
        .map(|Property { name, .. }| name.to_token_stream());

    quote! {
        #(#values)*

        #fold

        // merge all values together, once we're here all errors have been cleared
        let this = #type_ {
            #(#fields),*
        };

        Ok(this)
    }
}

pub(crate) fn generate_property(
    base: &BaseUrl,
    Property {
        name,
        type_,
        kind,
        required,
    }: &Property,
    variant: Variant,
    visibility: Option<&Visibility>,
    import: &mut Import,
) -> TokenStream {
    let url = base.as_str();

    let type_ = match variant {
        Variant::Owned => type_.to_token_stream(),
        Variant::Ref => quote!(<#type_ as Type>::Ref<'a>),
        Variant::Mut => quote!(<#type_ as Type>::Mut<'a>),
    };

    let mut type_ = match kind {
        PropertyKind::Array if variant == Variant::Owned || variant == Variant::Mut => {
            import.vec = true;
            quote!(Vec<#type_>)
        }
        PropertyKind::Array => {
            import.box_ = true;
            quote!(Box<[#type_]>)
        }
        PropertyKind::Plain => type_.to_token_stream(),
        PropertyKind::Boxed => {
            import.box_ = true;
            quote!(Box<#type_>)
        }
    };

    if !required {
        type_ = quote!(Option<#type_>);
    }

    quote! {
        #[serde(rename = #url)]
        #visibility #name: #type_
    }
}
