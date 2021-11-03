use std::path::Path;

use tego::*;

#[test]
fn load_default_example_map() {
    let map = Map::from_file(Path::new("example-maps/default/default_map.tmx")).unwrap();

    assert_eq!(map.version, Version(1,5,None));
    assert_eq!(map.editor_version, Some(Version(1, 7, Some(2))));
    assert_eq!(map.layers.len(), 1);

    assert_eq!(map.tilesets.len(), 1);
    assert!(matches!(
        &map.tilesets[0].image,
        ImageStorage::SpriteSheet(path) if path.downcast_ref::<String>().unwrap() == "tiles.png"
    ));

    if let Layer::Tile(layer) = &map.layers[0] {
        assert_eq!(layer.size, math::ivec2::new(16, 16));
        for (pos, gid) in layer.tiles_in_renderorder(&map) {
            if gid.is_none() {
                continue;
            }
            let item = map.tile_image(gid.unwrap());
            // We must be able to resolve all GIDs that we find in a valid map
            assert!(item.is_some());

            let (img, src) = item.unwrap();
            println!("Tile {:?} uses pixels {:?} in {:?}", pos, src, img);
        }
    }
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

#[test]
fn load_object_example_map() {
    let map = Map::from_file(Path::new("example-maps/default/objects.tmx")).unwrap();

    assert_eq!(map.layers.len(), 2);

    let object_layers: Vec<_> = map
        .iter_layers()
        .filter_map(|(l,_)| match l { Layer::Object(x) => Some(x), _ => None})
        .collect()
    ;

    assert_eq!(object_layers.len(), 2);

    let objects = &object_layers[0].content;
    assert_eq!(objects.len(), 3);
    assert!(matches!(&objects[0].kind, ObjectKind::Ellipse));
    assert!(matches!(&objects[1].kind, ObjectKind::Polygon{..}));

    if let ObjectKind::Text{ content, .. } = &objects[2].kind {
        assert_eq!(content, "Hello World");
    } else {
        panic!("Expected object to be a text object");
    }

    let objects = &object_layers[1].content;
    assert_eq!(objects.len(), 2);
    assert!(matches!(&objects[0].kind, ObjectKind::Rect));
    assert!(matches!(&objects[1].kind, ObjectKind::Point));
}
