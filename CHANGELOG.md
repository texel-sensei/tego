# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Added
- The `backgroundcolor` property of the Map is now loaded
- The `tintcolor` of tile layers is now loaded
- The `color` and `tintcolor` of object layers is loaded
- The `tintcolor` of group layers is now loaded

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
