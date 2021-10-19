use std::path::Path;

use tego::*;

#[test]
fn load_default_example_map() {
    let map = Map::from_file(Path::new("example-maps/default/default_map.tmx")).unwrap();

    assert_eq!(map.version, Version(1,5,None));
    assert_eq!(map.editor_version, Some(Version(1, 7, Some(2))));
    assert_eq!(map.layers.len(), 1);
}

#[test]
fn load_group_example_map() {
    let map = Map::from_file(Path::new("example-maps/default/groups.tmx")).unwrap();

    assert_eq!(map.layers.len(), 2);
    assert!(matches!(map.layers[0], Layer::Tile(_)));
    assert!(matches!(map.layers[1], Layer::Group(_)));

    if let Layer::Group(ref g) = map.layers[1] {
        assert_eq!(g.name, "Objects");
        assert_eq!(g.content.len(), 3);
    }
}
