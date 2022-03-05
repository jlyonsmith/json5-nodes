# json5_nodes

This Rust library parses JSON5 into `JsonNode` structures that contain both the JSON value and the location of the data in the original string. This allows you to use JSON5 as a configuration format and refer back to the location of semantic errors in the original JSON5 as opposed to just reporting syntactic errors when reading the file.

See the [`stampver-rs`](https://github.com/jlyonsmith/stampver-rs) tool for an example of how to use the library.

## Implementation

We use [`IndexHashMap`](https://crates.io/crates/hashlink) instead of a plain [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) because JavaScript [mostly preserves the order of insertion into objects](https://stackoverflow.com/a/38218582). This libraries JSON5 parser currently only allows string based keys, so the rules are simplified.

All JSON breaks down into nodes of different types, `JsonNode::Null`, `JsonNode::String`, `JsonNode::Bool`, `JsonNode::Integer`, `JsonNode::Float`, `JsonNode::Array` and `JsonNode::Object`.  We define and parse two types of numbers because it's more important in statically typed languages to specifically pick one or the other.

## To Do

This library is a work in progress.  The following are some things that still need to be done:

- [ ] Get closer to 100% code coverage with the unit tests.
- [ ] Rewrite the hex conversions to avoid the pathological `Err` cases; the values are already parsed to be valid input.
- [ ] Maybe ensure that what's read by `parse` can be written back out in `stringify` with full fidelity.  In particular escape codes are not handled at all and hex numbers don't round-trip.
- [ ] A better README!
