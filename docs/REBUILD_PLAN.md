# EO-Texrip rebuild plan

This plan implements the [recovered project brief](PROJECT_BRIEF.md), using the final 0.73 folder decision as the baseline. It is a proposed implementation sequence; only the requirements recovery is complete in this repository.

## 0. Preserve the recovered baseline — complete

- Record the seven-game scope, native Rust application, offline workflow, compact output, preservation rules, CityHash64 mode, and final routing rollback.
- Separate historical test/release claims from current implementation status.
- Keep private transcripts and calibration provenance out of the public repository.

**Delivered:** README, project brief, and this implementation plan.

## 1. Establish the native application and reference data

- Create a Rust workspace with distinct core/catalog, ROM, texture/archive, game-profile, export, CLI, and desktop responsibilities. Reuse shared parsers through explicit profiles.
- Implement the Windows GUI shell: game-dump selection/drag-and-drop, output picker, background job status, progress, warnings, and output opening.
- Complete local recovery of the existing five calibration attachments where available. Record schema/build/commit identity and retain the original metadata privately for offline use.
- Turn relevant metadata findings into minimal synthetic regression cases. Recover the older Python ZIP only as optional parser/reference material; it is not an end-user dependency or a parity gate.
- Use one source of version information and deterministic dependency versions. Document developer builds separately from user instructions.

**Acceptance:** a packaged Windows executable opens reliably, reports failures visibly, and can exercise the pipeline with synthetic data. End users do not install development dependencies.

## 2. Restore correct offline extraction for the two Untold games

- Implement decrypted NCSD/NCCH/CIA and filesystem reading, game identity, and explicit unsupported/encrypted-input errors.
- Restore bounded HPI/HPB/ACMP, FARC/SIR0, and embedded-resource traversal, selected by actual structure.
- Implement STEX, Untold ATBC/BAM to CGFX, 2 Untold BAM2 to BCH/H3D, font sheets, and validated supported texture containers.
- Decode all fourteen PICA formats with correct padding, Morton layout, channel order, alpha, and mip boundaries.
- Retain material binding evidence and distinguish true separate masks from diagnostic previews.
- Report unsupported resources and failed candidates without emitting arbitrary garbled PNGs.

**Acceptance:** independent synthetic vectors cover key decoder and container boundaries, and extraction reports account for delivered outputs and failures. Previously recorded Untold coverage issues become focused regressions.

## 3. Restore safe masters and Azahar deployment

- Create stable catalog identities independent of filenames; deduplicate image masters while preserving every established runtime mapping.
- Generate CityHash64 from the appropriate runtime byte span, retain hash evidence, and write per-title `pack.json` with `use_new_hash=true`.
- Keep meaningful, unambiguous filenames. Map several hashes to one master when appropriate.
- Build `azahar_pack/` from `azahar_pack_master/`. Keep PNG-only/unmapped assets in `non-azahar/<category>/` and exclude them from the normal deployment mapping.
- Inventory existing edits and live pack mappings before changes. Stage outputs, validate references, and publish transactionally with rollback; do not delete unrelated folders.
- Retain lightweight metadata and remove expensive temporary copies after successful completion.

**Acceptance:** edited/upscaled and renamed masters survive reruns, category choices persist, corrupted metadata cannot overwrite them, failed jobs preserve the previous deployment, and generated mappings resolve to the intended files.

## 4. Extend profiles and keep simple categories

- Add EO IV, EO V, and Nexus using their recorded resource families and shared decoder stack.
- Restore discovery of embedded/unnamed valid resources, supplemental TMX as PNG-only, and STEX hidden by rejected archive probes.
- Leave nonstandard `.ctpk`-named payloads and unproven TTD/TGD handling explicitly unresolved until structural evidence supports them.
- Assign initial categories within `characters`, `monsters`, `ui`, `icons`, `maps`, `dungeon`, `backgrounds`, `effects`, `fonts`, and `misc`.
- Use source/provenance and logical-name structure rather than generic substring guesses. Add the recorded `bfNN_*` dungeon misclassifications as regression cases.
- Keep diagnostics internal. Do not add a second automatic routing pass, nested owner/role directories, or `unclassified` folders.
- Reuse immutable final calibration records for offline analysis and reconcile reports with actual delivered files.

**Acceptance:** all five Atlus profiles have explicit support boundaries, consistent output/report accounting, flat category output, and reusable offline metadata checks. Historical calibration counts remain reference snapshots rather than fixed universal totals.

## 5. Package and validate the usable application

- Produce a downloadable Windows release candidate with all required extraction dependencies and third-party notices.
- Check packaged GUI launch, responsive jobs, visible errors, folder opening, and master-to-deployment rebuild behavior.
- Run meaningful Rust checks and synthetic regression tests covering decoder accuracy, bounded traversal, hash/mapping correctness, collisions, and user-data preservation.
- Use recovered reports and offline replay for routine calibration. Do not impose Python fingerprint parity, repository-administration work, or repeated five-ROM calibration as user tasks.
- Check replacement behavior through narrowly scoped developer validation when suitable existing data is available; mark compatibility that has not been established for the rebuilt version.
- Publish support claims and known limitations that describe the rebuilt application rather than copying old release claims.

**Acceptance:** users can download, open, select a decrypted supported input, extract, edit masters, and generate a valid deployment without developer setup. Windows packaging and Steam Deck pack instructions are concrete and accurate.

## 6. Research the Mystery Dungeon pair independently

- Identify each game's actual filesystem, archives, texture containers, runtime layout, and material behavior.
- Add separate profiles and parsers only where evidence requires them; reuse generic ROM/PICA/export code where appropriate.
- Establish extraction and replacement evidence before changing either title from research target to supported game.

**Acceptance:** support claims for Etrian Mystery Dungeon and Etrian Mystery Dungeon 2 are backed by their own format and replacement evidence, rather than inferred from Untold or the other Atlus titles.

## Completion criteria

The rebuild is complete when the supported scope is explicit, the standalone application produces correct organized textures, user edits survive updates, emulator-ready files have justified mappings, unmapped images stay isolated, and packaged releases are usable without developer tooling. The seven-game target remains the goal; research gaps must remain visible until they are resolved.
