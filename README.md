# EO-Texrip

A native desktop app for extracting Nintendo 3DS Etrian Odyssey textures,
organizing and renaming them, protecting editable masters, and preparing Azahar
replacement packs for PC and Steam Deck.

**The app has been rebuilt from scratch in Rust.** Version `0.1.0-alpha.1`
includes a Windows GUI, CLI, native parsers/decoder, provenance-based category
rules, texture previews, saved corrections, and staged pack publication.
The lost project's recorded `0.73.0` release is historical evidence, not this
rebuild's version or a compatibility guarantee.

## Download and use

Download the Windows x64 ZIP from the newest successful
[Windows package workflow](https://github.com/skypie0102/EOTexrip/actions/workflows/build.yml?query=branch%3Arebuild%2Fnative-app):
open the run, then download its `EO-Texrip-0.1.0-alpha.1-windows-x64` artifact.
Extract the outer artifact ZIP and then the app ZIP inside it. Open
`EO-Texrip.exe`. End users do not install Python, Cargo, Git, or separate
extraction tools. The package includes a synthetic demo. GitHub requires a
signed-in account for artifact downloads; artifacts expire after 30 days.
The workflow can rebuild the package from the committed source.

Automatic release publication was rejected by GitHub with HTTP 403, so this
alpha uses tested build artifacts. Tagged releases can be published separately
with an account or token that has release permission.

1. Select the game, a decrypted dump or resource folder, and a workspace.
2. Extract and organize the textures offline.
3. Inspect previews and source evidence in the review list. Confirm a name or
   category once; the correction persists through later extractions.
4. Edit/upscale PNGs in `azahar_pack_master/`.
5. Rebuild `azahar_pack/` and copy its title folder to Azahar's custom textures directory.

The categories stay flat: `characters`, `monsters`, `ui`, `icons`, `maps`,
`dungeon`, `backgrounds`, `effects`, `fonts`, and `misc`.

## Naming and category reliability

The classifier uses original archive paths, embedded texture paths, typed
model owners, and anchored game namespaces. It records the rule behind each
assignment. Conflicting evidence and unresolved identities enter the review
list. A source identifier such as `NPC33` is retained until its readable
identity is confirmed. Filename collisions receive stable suffixes.

Shared images keep all source identities and runtime hashes. Saved corrections,
live pack renames, and edited/upscaled pixels survive reruns. Damaged metadata,
missing masters, and failed jobs stop publication instead of overwriting edits.
No category is assigned by reusing an old output folder as evidence.

The recovered five-game metadata contains **14,456 records**. Its offline replay
is documented in [the aggregate report](docs/CALIBRATION.md); assignment coverage
is measured separately from verified accuracy. Full retail pixel/emulator
validation of this rebuilt version is still pending.

## Support and development

- [User guide](docs/USER_GUIDE.md): extraction, review, naming, edits, and emulator use.
- [Support boundaries](docs/SUPPORT.md): implemented formats and explicit gaps.
- [Recovery brief](docs/PROJECT_BRIEF.md): requirements recovered from project history.
- [Rebuild status](docs/REBUILD_PLAN.md): completed work and remaining development.
- [Third-party notices](THIRD_PARTY.md).

The target remains the five mainline 3DS games and both Etrian Mystery Dungeon
titles. Mystery Dungeon requires independent format research and is explicitly
disabled in this alpha.

Developers can run `cargo test --all-targets --locked`, `cargo run --bin EO-Texrip`,
or `cargo run --no-default-features --bin eotexrip -- --help`. CI checks formatting,
Clippy, and meaningful extraction/preservation fixtures on Linux and Windows,
then builds and exercises a portable Windows package.
