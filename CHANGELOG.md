# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Added
- Load most attributes on text objects (except for halign/valign)
- Support for object templates
- Objects (and properties) now implement `Debug`

### Changed
- ImageStorage::SpriteSheet now holds the image data as an Rc instead of a Box.
  This allows for multiple tilesets to share the same image.
- A ResourceManager now caches the images that have been loaded with it.
- The order of properties returned by PropertyContainer::iter() is no longer
  necessarily the same order as it is in the tmx file.
- Indexing a `PropertyContainer` now returns `PropertyValue`s instead of
  Property objects.
- If multiple properties have the same name, they are no longer all included in
  the output of PropertyContainer::iter(), only the last one. This was changed
  because the semantic of having multiple properties combined with objects was
  unclear.

## [0.4.1] - 2021-11-17
### Added
- `PropertyContainer` now supports access via `[]` (Index)
- Properties now provide convenience functions for type casting

## [0.4.0] - 2021-11-14
### Added
- The `backgroundcolor` property of the Map is now loaded
- The `tintcolor` of tile layers is now loaded
- The `color` and `tintcolor` of object layers is loaded
- The `tintcolor` of group layers is now loaded
- It is now possible to lookup if a tile was flipped:
	`GID::flip_horizontal()`, `GID::flip_vertical()`, `GID::flip_diagonal()`
- Support loading custom properties
- ResourceManager class for better handling of external resources
- Support for external tileset files (`*.tsx`)

### Changed
- Most structs with public fields are now marked `#[non_exhaustive]`
- The functions `Map::from_file_with_loader()` and `Map::from_xml_str()` now take an ResourceManager
  parameter instead of an ImageLoader
- The `id` attribute of the different layers variants is now optional. This allows loading of maps
  created with Tiled prior to version 1.2

### Fixed
- `Map::tile_image()` no longer panics if the map contains tiles that have been flipped
- Loading maps now correctly calculates relative paths in relation to the map file and not the
  current working directory

## [0.3.1] - 2021-11-04
### Changed
- Fixed ugly indentation in example code

## [0.3.0] - 2021-11-04
### Added
- Support for loading objects stored in a map. Loading of text objects is only
  partially supported (only text contents, no metadata)
- Added Map::iter_layers() for a simple way to iterate the layers in a map
- math::ivec2 now implements [Default]

### Changed
- Replaced width/height pairs by math::ivec2

## [0.2.0] - 2021-10-25
### Added
- Implemented image lookup and tile iteration. This marks the first version of
  the library that can be used for rendering simple maps
- Introduced a Changelog
- Improved documentation of the crate
