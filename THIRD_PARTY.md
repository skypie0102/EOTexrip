# Third-party dependencies and format references

The Rust application bundles code from the packages pinned in `Cargo.lock`.
Direct dependencies are anyhow (MIT/Apache-2.0), clap (MIT/Apache-2.0), serde
(MIT/Apache-2.0), serde_json (MIT/Apache-2.0), sha2 (MIT/Apache-2.0), png
(MIT/Apache-2.0), walkdir (MIT/Unlicense), cityhasher (MIT/Apache-2.0), fs2
(MIT/Apache-2.0), eframe/egui (MIT/Apache-2.0), and rfd (MIT).
The Windows package includes `DEPENDENCY_LICENSES.yml`, generated from the
pinned Cargo dependency license texts during packaging. Supplemental upstream
GUI, clipboard, profiling, and font notices are included in `licenses/`. `tempfile` is used only for tests.

The implementations in this repository were written in Rust using format
layouts and behavioral references from:

- [UntoldUnpack](https://github.com/xdanieldzd/UntoldUnpack): HPI/HPB, ACMP, and STEX.
- [SPICA](https://github.com/gdkchan/SPICA): CGFX image descriptors, BCH commands/relocations, and PICA layout.
- [3DS Texture Forge](https://github.com/ZoomiesZaggy/3DS-Texture-Forge): independent parser comparison; no Python runtime is shipped.
- [FARC/SIR0 research](https://gist.github.com/mid-kid/8279635ee8dc57560e96): archive table layout.
- [CTPK layout](https://www.3dbrew.org/wiki/CTPK).
- [Azahar](https://github.com/azahar-emu/azahar): custom texture configuration and CityHash64 behavior.
- [Google CityHash](https://github.com/google/cityhash): hash reference vectors.

No Nintendo/Atlus game data, private calibration bundles, or historical chat
transcripts are distributed. The bundled demo contains generated test patterns.
