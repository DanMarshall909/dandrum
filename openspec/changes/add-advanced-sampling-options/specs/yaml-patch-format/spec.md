## ADDED Requirements

### Requirement: Sample assets are declared beside the root graph

Patch and defined-module YAML SHALL accept an `assets` section containing `sample_sources` and `sample_maps`. A source SHALL declare a stable ID and relative resource path and SHALL accept named regions, simple loop metadata, explicit slices, cues, and optional timing metadata. A map SHALL declare a stable ID and zones with region references, key and velocity ranges, selection metadata, and optional choke groups. Loading SHALL preserve the declarations and each source path's document or package provenance for preparation. The schema SHALL reject unknown sampling fields.

#### Scenario: Patch declares source and map assets

- **WHEN** a valid patch declares a source with a region, loop, slice, and cue plus a map with velocity zones and a choke group
- **THEN** YAML loading SHALL accept and preserve those declarations for preparation

#### Scenario: Packaged source keeps its resource origin

- **WHEN** a packaged defined module declares a relative sample source path
- **THEN** YAML loading SHALL retain the package-version root with that path

#### Scenario: Unknown sampling field fails schema validation

- **WHEN** a source, region, slice, map, or zone declares an unsupported field
- **THEN** patch loading SHALL reject the document before graph construction
