# json5_nodes

This Rust library parses JSON5 into `JsonNode` structures that contain the JSON value *and* the location of the data in the original string. This allows you to use JSON5 as a configuration format and refer back to the location of _semantic_ errors in the original JSON5 as opposed to just reporting _syntactic_ errors when reading the file.

## Implementation

We use [`IndexHashMap`](https://crates.io/crates/hashlink) instead of a plain [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) because JavaScript [mostly preserves the order of insertion into objects](https://stackoverflow.com/a/38218582). This libraries JSON5 parser currently only allows string based keys, so the rules are simplified.
