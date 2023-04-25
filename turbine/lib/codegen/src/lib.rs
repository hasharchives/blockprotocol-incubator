// what we need to do:
// 1) Configuration:
//      - URL to get entity types
//      - style of module (mod.rs vs. module.rs)
//
// 2) fetch all types
// 3) categorize into:
//      - data types (if built-in refer to those, otherwise error out)
//      - property types
//      - entity types
// 4) create modules for each type, they are designated by
//      if hash: url base (backwards) / org / entity|property / id.rs
//      if blockprotocol: bp / org / entity|property / id.rs
// 4) when referring to those just use crate::<URL>::...
// 5) generate the code required: 2 variants: Owned and Ref (ref is lightweight)
//      with proper accessors, id converted to snake_case,
//          if duplicate error out,
//              sort properties,
//                  then increment
//              same for import problems, just alias with the name we want
//
// internally we also need to keep track which entity is in which file
// todo: generate code, that selects Ref out of all verticies of a specific type, should not be
// generated, but generic code instead
//
// result: BTreeMap<File, String>
// where File is the module name as a Path, so it can be created by e.g. CLI
//
// Problematic: multi layered objects/properties (validating them correctly ~> needs intermediate
// types (with names (?)))
//
// If multiple versions, the latest version is named Example, while the others are called ExampleV1
//
// TODO: entities can also have link data associated with them! (important on self?)
//
// TODO: tests?
fn fetch() {}
