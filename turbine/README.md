# Turbine

A very experimental, very alpha Rust code generator for blockprotocol graph types.

This project will gradually add new functionality and tests, be aware that for now periodic breakage and poor test coverage are to be expected (which will hopefully change soon).

This workspace consists of several crates:

* `bin/cli`: The main entry-point for generation
* `lib/codegen`: Code generator of the code, input is a collection of types, output is a map of path to content
* `lib/skeletor`: Takes the output from codegen and bootstraps a new `no-std` library crate
* `lib/turbine`: The underlying library which includes all types and traits that are needed and references in the generated code
