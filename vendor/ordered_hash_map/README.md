# OrderedHashMap

An ordered hash map implementation for Rust

This crate provides a data structure that combines the features of a hash map and a linked list. It maintains the order of insertion while allowing fast key lookups. Features:

- Fast Lookup: HashMap insertions and fetches are as quick as HashMap
- Ordered Iteration: Iterators walk in insertion order
- Serde Support: Serialize/Deserialize a map while preserving order

The aim is to match the standard library HashMap/Set with additional LinkedList style methods and ordered-iterators.

## [Changelog](CHANGELOG.md)

## Features
`serde` - Enable serde Serialization and Deserialization

## Usage
```rust
use ordered_hash_map::OrderedHashMap;

fn main() {
    let mut map = OrderedHashMap::new();

    map.insert("apple", 5);
    map.insert("banana", 3);
    map.insert("cherry", 8);

    // Map access
    println!("banana: {}", map.get("banana").unwrap());

    // Insertion-order iteration
    for (k, v) in map.iter() {
        println!("{}: {}", k, v);
    }
}
```

## Details

This crate is powered by [hashbrown](https://github.com/rust-lang/hashbrown), leveraging the already well vetted HashMap implementation and added the bits required to implement a LinkedList within the map. The additional memory footprint of the OrderedHashMap over the hashbrown HashMap is 2 pointers upon making a collection + 3 pointers per element. The pointers arise from the LinkedList where the collection itself has a pointer to the head and the tail, each node has a pointer to the next and previous node, and the Key in the map is a pointer into the Value where it lives.
