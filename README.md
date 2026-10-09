# EO-Texrip

Extract textures from decrypted Nintendo 3DS Etrian Odyssey games, organize them into useful folders, and prepare editable masters and an Azahar replacement pack for HD play on PC and Steam Deck.

**Reconstruction status:** this repository currently contains the project requirements recovered from the earlier EO-Texrip conversations and a rebuild plan. The extractor, Rust desktop application, and release binaries have not been restored here. Historical release and test results describe the lost project, not code in this repository.

The last stable release recorded in that history was **0.73.0**. Its final folder decision is the rebuild baseline: familiar category folders, with the later automatic categorization routing removed.

## Intended workflow

1. Open a supported decrypted game dump in a standalone Windows GUI.
2. Extract and decode 2D artwork, model textures, dungeon assets, effects, and fonts offline, without completing a playthrough to collect textures.
3. Deduplicate the results and place them in simple category folders.
4. Edit or upscale the PNGs in `azahar_pack_master/`.
5. Build `azahar_pack/` with the game's texture hash mappings and `pack.json` for emulator use.

Reruns must preserve edited masters, manual names, and category choices. Images without established Azahar mappings belong in `non-azahar/<category>/`, outside the normal replacement pack.

## Scope

The target is all seven 3DS titles: Etrian Odyssey IV, Untold, 2 Untold, V, Nexus, Etrian Mystery Dungeon, and Etrian Mystery Dungeon 2. The history records extraction and visible replacement tests for the first five; the Mystery Dungeon pair remains a separate research target.

The standard categories are `characters`, `monsters`, `ui`, `icons`, `maps`, `dungeon`, `backgrounds`, `effects`, `fonts`, and `misc`. The final decision rejects nested owner/role routing and categorization-created `unclassified` folders.

## Recovered project documentation

- [Project brief](docs/PROJECT_BRIEF.md): requirements, technical findings, final decisions, and limits of the recovered evidence.
- [Rebuild plan](docs/REBUILD_PLAN.md): implementation stages and practical acceptance criteria.

The intended application bundles its extraction dependencies. End users should not need Python, Texture Forge downloads, Cargo, Git, or a developer validation workflow to extract textures.
