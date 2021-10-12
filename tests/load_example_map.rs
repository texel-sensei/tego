use std::path::Path;

use tego::*;

#[test]
fn load_defaul_example_map() {
    let map = Map::from_file(Path::new("example-maps/default/default_map.tmx")).unwrap();

    assert_eq!(map.version, Version(1,5,None));
    assert_eq!(map.editor_version, Some(Version(1, 7, Some(2))));
}
