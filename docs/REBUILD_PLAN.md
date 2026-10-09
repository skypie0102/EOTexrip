# EO-Texrip rebuild status

The original extraction → organized masters → emulator pack goal is unchanged.
The latest requirement makes reliable initial categorization and renaming the
main improvement criterion. The old 0.73 rollback informs the flat folder
layout; it does not require keeping the old classifier's mistakes.

## Implemented in 0.1.0-alpha.1

- Native Rust desktop app and CLI, with shared extraction/classification logic.
- Decrypted ROM/RomFS traversal; HPI/HPB/ACMP and structural archive scanning.
- STEX, CGFX, BCH, BCFNT, standard CTPK and all fourteen PICA pixel formats.
- Provenance/embedded-path/model-owner decisions with visible evidence.
- Stable source identities, readable names, collision handling, shared-image origins.
- PNG previews, review/search/category filters, individual and exact-source category corrections.
- Persistent confirmed choices and offline replay of all five recovered metadata bundles.
- Protected edited masters, live pack rename import, hash mappings and staged publication/recovery.
- Linux/Windows checks, synthetic end-to-end fixtures, portable Windows packaging and demo.

See [support boundaries](SUPPORT.md) for what these implementations have and
have not validated. The first alpha does not claim complete seven-game support.

## Next development priorities

1. Independently label unresolved metadata families and verified English entity
   identities. Reduce incorrect assignments and unresolved names using the saved
   captures; do not request another five-ROM calibration cycle.
2. Validate retail pixel spans, archive variants, BCH raw-extension relocation,
   material-bound alpha masks and in-emulator replacement on suitable existing data.
3. Restore TMX as PNG-only and investigate the recorded nonstandard CTPK resources.
4. Verify interactive Windows GUI launch and Steam Deck deployment behavior.
5. Research both Mystery Dungeon games separately before enabling those profiles.

Acceptance is correct outputs, persistent edits/corrections, justified hashes,
explicit unresolved cases, and a usable standalone package. Assigned-folder
coverage alone is not accuracy and is not the completion criterion.
