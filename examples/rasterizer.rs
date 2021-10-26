use std::{error::Error, path::Path};
use image::{RgbaImage, GenericImageView, GenericImage};

struct ImageLoader {}

impl ImageLoader {
    fn new() -> Self { Self {  } }
}

impl tego::ImageLoader for ImageLoader {
    fn load(&mut self, path: &str) -> tego::Result<Box<dyn std::any::Any>> {
        let image =
            image::open(Path::new(path))
            .map_err(|e| tego::Error::ParseError(Box::new(e)))?
            .to_rgba8()
        ;
        Ok(Box::new(image))
    }
}

fn render_layer(
    map: &tego::Map, layer: &tego::Layer, buffer: &mut RgbaImage
) -> Result<(), Box<dyn Error>> {
    use tego::Layer::*;
    match layer {
        Group(group) => for l in &group.content { render_layer(map, &l, buffer)?;},
        Tile(tiles) => {
            for (pos, gid) in tiles.tiles_in_renderorder(map) {
                if gid.is_none() { continue; }
                let (img_path, src_rect) = map.tile_image(gid.unwrap()).unwrap();

                // TODO(texel, 2021-10-23): this opens the spritesheet for every single tile,
                // implement some caching (ideally inside of the map loader)
                let tile_image = img_path.downcast_ref::<RgbaImage>().unwrap();

                let tile_sprite = tile_image.view(
                    src_rect.upper_left.x as u32, src_rect.upper_left.y as u32,
                    src_rect.size.x as u32, src_rect.size.y as u32
                );

                let origin = pos *  map.tile_size;

                buffer.copy_from(
                    &tile_sprite,
                    origin.x as u32, origin.y as u32
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

    let map = tego::Map::from_file_with_loader(Path::new(input), &mut ImageLoader::new())?;

    let resolution = map.size * map.tile_size;
    let mut buffer = RgbaImage::new(resolution.x as u32, resolution.y as u32);

    for layer in &map.layers {
        render_layer(&map, &layer, &mut buffer)?;
    }

    buffer.save(output)?;

    Ok(())
}
