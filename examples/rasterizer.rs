use std::{error::Error, path::Path};
use image::{RgbaImage, GenericImageView, GenericImage};


fn render_layer(
    map: &tego::Map, layer: &tego::Layer, buffer: &mut RgbaImage
) -> Result<(), Box<dyn Error>> {
    use tego::Layer::*;
    match layer {
        Group(group) => for l in &group.content { render_layer(map, &l, buffer)?;},
        Tile(tiles) => {
            for (pos, gid) in tiles.tiles_in_renderorder(map) {
                if gid.is_none() { continue; }
                // TODO(texel, 2021-10-23): Make it possible to insert own loader
                let (img_path, src_rect) = map.tile_image(gid.unwrap()).unwrap();
                let img_path = img_path.downcast_ref::<String>().unwrap();

                // TODO(texel, 2021-10-23): this opens the spritesheet for every single tile,
                // implement some caching (ideally inside of the map loader)
                let tile_image = image::open(Path::new(img_path))?.to_rgba8();

                let tile_sprite = tile_image.view(
                    src_rect.upper_left.x as u32, src_rect.upper_left.y as u32,
                    src_rect.size.x as u32, src_rect.size.y as u32
                );

                buffer.copy_from(
                    &tile_sprite,
                    pos.x as u32 * map.tilewidth as u32, pos.y as u32 * map.tileheight as u32
                )?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <path/to/tmx> <output>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    let map = tego::Map::from_file(Path::new(input))?;

    let mut buffer = RgbaImage::new((map.width * map.tilewidth) as u32, (map.height * map.tileheight) as u32);

    for layer in &map.layers {
        render_layer(&map, &layer, &mut buffer)?;
    }

    buffer.save(output)?;

    Ok(())
}