# tego

[![Documentation](https://docs.rs/tego/badge.svg)](https://docs.rs/tego)
[![Crates.io](https://img.shields.io/crates/v/tego.svg)](https://crates.io/crates/tego)
![License](https://img.shields.io/crates/l/tego.svg)

[tego](https://github.com/texel-sensei/tego) is a library for parsing and
loading [Tiled](https://www.mapeditor.org/) maps.

## Goals

The main goal of tego is to provide a foundation for loading tmx maps,
independent of game engine or storage medium.
It should not matter if the map is stored as a file on disk,
or loaded on the fly from an asset bundle.

Furthermore common operations should be made easy with sane defaults,
but not at the expense of flexibility.


## Example

Load a map and pretty print the layers included in it:

```rust
use std::path::Path;
extern crate tego;

fn main() -> tego::Result<()> {
    // Load a tmx file.
    // Map::from_file() is the easiest, but least flexible method for loading a map.
    // Images referenced by the map are not loaded, instead only the path is returned as string.
    let map = tego::Map::from_file(Path::new("example-maps/default/groups.tmx"))?;

    // Keep track how much we need to indent for some nice pretty printing
    let mut indent = 0;

    for (layer, groups_left) in map.iter_layers() {
        // Reduce indentation by the amount of groups left
        indent -= groups_left;

        // print indentation to highlight hierarchy
        print!("{}", "  ".repeat(indent));

        use tego::Layer::*;
        match layer {
            Tile(layer) => {
                println!("Layer '{}' with {}x{} tiles", layer.name, layer.size.x, layer.size.y);
            },
            Group(layer) => {
                println!("Group layer '{}' with {} sub-layers", layer.name, layer.content.len());

                // increase indentation for all layers part of this group
                indent += 1;
            },
            Object(layer) => {
                println!("Layer '{}' containing {} objects", layer.name, layer.content.len());
            },
        }
    }
    Ok(())
}
```

> You can run this example with `cargo run -q --example layer_printer`.

## Feature support

The following TMX features are implemented ✅, partially supported 🚧 or
missing ❌ in tego.
This is not an exhaustive list.

* 🚧 Loading of maps with metadata:
    * ✅ Orthogonal & Isometric maps
    * ❌ Hexagonal & staggered maps
    * ❌ Editor related metadata
    * ✅ Color information

* 🚧 Tile Sets
    * ✅ Metadata
    * ✅ Sprite Sheet lookup with spacing/margin
    * ✅ External tile set files (`*.tsx`)
    * ❌ Image collection Tile Sets
    * ❌ Object Alignment information

* 🚧 Tile layers
    * ✅ uncompressed/zlib/gzip base64 data
    * ❌ csv loading
    * ❌ `<tile>` loading
    * ✅Tile flipping

* ❌ Infinite maps

* 🚧 Object layers
    * ✅ Basic Rect/Ellipse/Point object
    * ✅ Polygons & Polylines
    * 🚧 Text (Some metadata is still not supported, e.g. haling/valign)
    * ❌ Object Templates

* ❌ Image layers

* ✅ Properties
