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
                println!(
                    "Layer '{}' with {}x{} tiles",
                    layer.name, layer.size.x, layer.size.y
                );
            }
            Group(layer) => {
                println!(
                    "Group layer '{}' with {} sub-layers",
                    layer.name,
                    layer.content.len()
                );

                // increase indentation for all layers part of this group
                indent += 1;
            }
            Object(layer) => {
                println!(
                    "Layer '{}' containing {} objects",
                    layer.name,
                    layer.content.len()
                );
            }
            _ => {
                println!("Unknown layer");
            }
        }
    }
    Ok(())
}
