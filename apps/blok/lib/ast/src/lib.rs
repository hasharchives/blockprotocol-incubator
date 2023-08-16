use std::{collections::BTreeMap, marker::PhantomData};

use text_size::TextRange;

pub struct Id(String);

pub struct NodeInfo {
    pub id: Id,
    pub ident: String,

    pub title: String,
    pub description: Option<String>,

    pub version: Option<u32>,
    pub archived: bool,

    pub position: TextRange,
}

// TODO: cannot be currently defined!
pub struct DataType {
    info: NodeInfo,
}

pub struct Reference<T> {
    pub to: String,
    _marker: PhantomData<T>,
}

pub struct Object {
    pub properties: Vec<Reference<PropertyType>>,
}

pub enum OneOfArray {
    Object(Object),
    Array(Box<Array<OneOfArray>>),
}

pub struct Array<T> {
    pub item: T,

    pub min_items: Option<u32>,
    pub max_items: Option<u32>,
}

pub enum OneOf {
    Object(Object),
    Array(Array<OneOfArray>),
    Reference(Reference<DataType>),
}

pub struct PropertyType {
    info: NodeInfo,

    one_of: Vec<OneOf>,
}

pub struct Link {
    to: Reference<EntityType>,

    min_items: Option<u32>,
    max_items: Option<u32>,
}

pub struct EntityType {
    info: NodeInfo,

    pub properties: Vec<Reference<PropertyType>>,
}

pub struct Versioned<T> {
    pub versions: BTreeMap<u32, T>,
}

pub enum Node {
    DataType(DataType),
    PropertyType(PropertyType),
    EntityType(EntityType),

    Versioned(VersionedNode),
}

pub enum VersionedNode {
    DataType(Versioned<DataType>),
    PropertyType(Versioned<PropertyType>),
    EntityType(Versioned<EntityType>),
}

pub struct Import {
    from: String,
    name: String,
}

pub struct Export {
    name: String,
}

pub struct Module {
    pub id: Id,

    pub imports: Vec<Import>,
    pub exports: Vec<Export>,

    pub nodes: Vec<Node>,
}
