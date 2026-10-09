# Recovered EO-Texrip project brief

Recovered on 2026-10-09 from nine earlier conversations in the EO-Texrip project. This brief records that project's requirements and decisions. It does not establish that the lost implementation, binaries, or reported tests have been restored to this repository.

## Product goal

Build a standalone texture extraction and pack preparation application for the Nintendo 3DS Etrian Odyssey games. The user wants to experience the actual games with HD replacement textures, including a first playthrough of Untold on Steam Deck.

Offline extraction from decrypted game files is central: completing the game to accumulate emulator dumps must not be the primary extraction workflow. The application prepares textures for editing/upscaling and emulator replacement; recreating gameplay or adapting the PC EO HD releases is outside the recovered scope.

## Final decisions that govern the rebuild

| Topic | Recovered baseline |
| --- | --- |
| Application | Independent native Rust application with bundled extraction dependencies. |
| Primary interface | Packaged Windows GUI with a usable downloadable executable. |
| Play target | Azahar texture replacement on PC and Steam Deck. |
| Game coverage | Seven 3DS games, with explicit game profiles and a separate Mystery Dungeon format family. |
| Folder organization | Familiar semantic category folders. Remove the later automatic owner/role routing system. |
| Editable output | Compact, deduplicated `azahar_pack_master/`, preserved across reruns. |
| Deployment output | `azahar_pack/`, generated from masters and real hash mappings. |
| Unmapped images | `non-azahar/<category>/`, excluded from normal replacement work and `pack.json`. |
| Hash mode | CityHash64 with `use_new_hash=true`; old-hash examples must not switch the app to a legacy mode. |
| User validation | A runnable release candidate comes before requests to test it. |
| Calibration | Reuse saved metadata and offline replay. Do not repeat the five-game ROM calibration cycle for routine tuning. |
| Legacy implementation | Optional reference material; matching the old Python extractor is not a product requirement. |

These decisions take precedence over older proposals in the history, particularly the later rejection of nested categorization routing.

## Game scope and historical evidence

| Game | Profile family | Evidence recorded in the prior conversations |
| --- | --- | --- |
| Etrian Odyssey IV: Legends of the Titan | Shared Atlus extraction stack, game-specific profile | Extraction/coverage reports discussed; user reported Azahar replacement passed. |
| Etrian Odyssey Untold: The Millennium Girl | Shared Atlus extraction stack, game-specific profile | Original target; coverage work discussed; user reported replacement visible. |
| Etrian Odyssey 2 Untold: The Fafnir Knight | Shared Atlus extraction stack, game-specific profile | Native extraction/coverage work discussed; user reported replacement visible. |
| Etrian Odyssey V: Beyond the Myth | Shared Atlus extraction stack, game-specific profile | Extraction/coverage reports discussed; user reported Azahar replacement passed. |
| Etrian Odyssey Nexus | Shared Atlus extraction stack, game-specific profile | Extraction/coverage reports discussed; user reported Azahar replacement passed. |
| Etrian Mystery Dungeon | Separate Mystery Dungeon family | Included in the target scope; completed extraction/replacement support was not established in the recovered history. |
| Etrian Mystery Dungeon 2 | Separate Mystery Dungeon family | Included in the target scope; completed extraction/replacement support was not established in the recovered history. |

Historical evidence is useful for prioritization and regression design. It is not a current support claim for this repository. Texture counts and title identifiers can vary by region/build and must come from the input, not guesses or hard-coded global expectations.

## Desktop experience and distribution

The intended user flow is ROM selection, output-folder selection, **Extract Textures**, progress/status, completion counts and warnings, and **Open Output Folder**. The history also records ROM drag-and-drop and a dark desktop interface.

Extraction and expensive workspace operations should run in the background so the GUI stays responsive. Launch failures and extraction errors must be visible rather than causing the executable to silently close.

End-user packages must include their extraction dependencies. No user Python/Pillow/NumPy installation, Texture Forge bootstrap, Cargo build, Git checkout, or command-line setup should be necessary for normal Windows use. Developer toolchains remain development requirements.

Linux/Steam Deck CLI use appeared in the earlier workflow. Steam Deck-compatible pack output is required; the history does not establish a completed native Linux GUI release.

Game assets, ROMs, firmware, and keys are input data rather than bundled application dependencies.

## Input and extraction architecture

The historical input scope includes decrypted `.3ds`/CCI/NCSD, `.cia`, `.cxi`/NCCH, and `.app` containers. Encrypted payloads should produce an actionable unsupported-input message instead of being interpreted as texture bytes.

Preserve the separation between ROM reading, game identification, bounded archive traversal, validated resource parsing, texture decoding, material analysis, asset cataloging, folder selection, and pack generation. Profiles select shared components and game-specific rules; one game's container assumptions must not be applied to every title.

| Layer | Formats and findings recorded in the history |
| --- | --- |
| ROM/filesystem | NCSD, NCCH, CIA/TMD, ExeFS, IVFC/RomFS. |
| Archives | HPI/HPB and ACMP compression; FARC/SIR0; recursive discovery and EPL-contained resources. |
| Standalone textures | STEX; structurally valid CTPK; TMX decoded as PNG-only where no runtime mapping is established. |
| Model textures | Untold ATBC/BAM resources containing CGFX; 2 Untold BAM2 resources containing BCH/H3D. The earlier assumption that both Untold games use the same model path was corrected. |
| Fonts | BCFNT font-sheet extraction, including sheets previously missed by a parser limit. |

The recorded PICA format scope is all fourteen formats: `RGBA8`, `RGB8`, `RGBA5551`, `RGB565`, `RGBA4`, `LA8`, `HILO8`, `L8`, `A8`, `LA4`, `L4`, `A4`, `ETC1`, and `ETC1A4`.

Correctness requires channel ordering, 8x8 Morton swizzling, visible versus padded storage dimensions, mip boundaries, and strict payload/range validation. Broad binary scans that reinterpret arbitrary bytes as textures previously produced garbled images. Recovery must validate structure before decoding, while still finding valid embedded resources and archive members without useful extensions.

Historical coverage fixes included embedded CGFX discovery, unnamed TMX members, STEX hidden by a rejected EPL archive probe, and BCFNT sheet traversal. Unsupported outer containers must not mask independently valid inner texture resources.

Some EO IV/V/Nexus payloads named `.ctpk` were found not to be standard CTPK. They remained unresolved; filename extension alone is insufficient evidence. TTD/TGD texture handling was also not established in the recovered history.

Archive traversal must have depth, file, and byte limits and contain paths within its workspace. Malformed input must not damage an existing good workspace.

## Alpha and material handling

Preserve embedded transparency in exported PNGs. Export separate alpha/mask textures only when real texture/material evidence establishes a distinct resource. Do not manufacture duplicate alpha PNGs for every image.

A friendly pair such as `monkey.png` and `monkey-alpha.png` is appropriate only when the source actually contains a separate bound mask. A combined material preview is diagnostic output, not automatically an independent emulator replacement. Different UVs, filtering, transforms, or sampling can prevent a preview from reproducing the in-game material exactly.

## Texture identity and emulator export

The history corrected an early XXH64 implementation to **CityHash64** and consistently retained **`use_new_hash=true`** afterward. Old-hash sample packs were provided as references, not authorization to revert that decision.

Hashing must distinguish the encoded base-level bytes used by the runtime from decoded RGBA pixels, padding, and additional mips. Record the evidence behind mappings: candidate, structural, runtime-verified, or user-verified. Do not silently turn a visually similar image into a verified hash match.

The deployment pack uses `pack.json` at the game's title root to map meaningful PNG filenames to their hashes. A deduplicated image may serve multiple runtime hashes. Filenames must remain globally unambiguous for the mapping; collisions can use a short stable suffix. Preserve useful source names without inventing unsupported English monster names.

TMX decoding from a PS2 GS source layout did not establish an Azahar runtime hash. These textures must remain PNG-only unless separate evidence supplies a valid mapping. More generally, successful decoding alone is insufficient to claim replacement readiness.

## Compact workspace and preservation

| Location | Purpose |
| --- | --- |
| `azahar_pack_master/` | Deduplicated editable PNG masters, organized by the familiar categories. |
| `azahar_pack/` | Deployment copies and per-title `pack.json`, generated from the masters. |
| `non-azahar/<category>/` | Isolation pattern for decoded images lacking established runtime mappings; outside the eligible deployment set. |
| `.eouhd/` | Lightweight catalog, extraction reports, and diagnostics. |

The two practical image outputs are the master and deployment packs. Keep both compact; temporary ROM/archive copies and intermediary extraction trees should be cleaned up after a successful run.

Asset identity must survive user renames and category changes. Rerunning extraction or rebuilding the deployment pack must preserve edited/upscaled pixels, manually renamed masters, category overrides, and meaningful edits to the active `pack.json`.

Inventory existing files independently of the manifest, stage and validate replacements, and publish the new workspace transactionally with rollback. A corrupt manifest or failed extraction must not overwrite the user's masters or remove unrelated files. Retained material/report references must resolve to persistent files or clearly report that the artifact is unavailable.

## Folder baseline: the final routing rollback

The familiar categories are:

| Folder | Intended assets |
| --- | --- |
| `characters` | Character artwork and textures. |
| `monsters` | Enemy, FOE, and boss textures. |
| `ui` | Interface elements. |
| `icons` | Small symbolic assets. |
| `maps` | Map assets. |
| `dungeon` | Dungeon/battlefield geometry and scenery. |
| `backgrounds` | Background artwork. |
| `effects` | Effect textures. |
| `fonts` | Font sheets. |
| `misc` | Remaining assets without a justified category. |

The later M1-M27 experiments separated asset owner from texture role and proposed nested paths. Version 0.72 applied trusted routes in the GUI. The user then explicitly rejected the extra nesting and `unclassified` folders and asked to remove categorization routing altogether.

The recorded **0.73.0** rollback removed automatic GUI routing and restored the established extraction layout. Internal categorization diagnostics may remain, but must not perform a second reorganization pass. Do not rebuild `environment/unclassified`, owner/role hierarchies, or a policy-routing GUI as default behavior.

Useful classification fixes should fit the existing categories at initial extraction. Source structure and provenance are stronger evidence than generic substrings. Recorded regression examples are `bf01_forest1`, `bf03_water`, `bf05_ceiling01`, and `bf05_floor01`: these dungeon assets had been placed in monsters. Strip exported size/format/hash suffixes before applying logical-name rules, and preserve explicit manual choices.

Image-analysis classification was discussed as a possible future feature. The history explicitly distinguished that proposal from the implemented metadata/provenance classifier. A bundled AI image classifier or AI upscaler is not an established rebuild requirement.

## Reports and reusable calibration

Produce reports automatically and prefix filenames with the game so uploads are easy to distinguish. The latest five-game prefixes in the history are `EOU`, `EOU2`, `EOIV`, `EOV`, and `EON`, for example `EOU-extraction-report.json` and `EOU-categorization-debug.json`.

Extraction reports should account for exported textures, duplicates, eligible versus PNG-only images, unsupported resources, parser/decoding failures, dimensions/formats, material associations, and hash evidence. Diagnostics must describe the final delivered files after post-processing, not an obsolete intermediate layout.

The user asked to stop repeated retail-ROM test runs. M24 therefore captured one terminal metadata bundle per game; M27 added a Rust-native offline replay path for classifier tuning without reopening ROMs or reading pixel data.

The historical M24 bundle schema is `eo-texrip-categorization-calibration-bundle-v1`. Bundles contain the final extraction report, synchronized categorization debug, extraction validation, names/provenance, hashes, dimensions/formats, material/evidence information, and issues. They deliberately exclude ROM/archive payloads and texture pixels. Metadata may still contain private paths or provenance and must not be copied wholesale into a public repository.

| Bundle prefix | Historical final exported records |
| --- | ---: |
| EOU | 2,280 |
| EOU2 | 2,884 |
| EOIV | 2,158 |
| EOV | 3,146 |
| EON | 3,988 |
| Total | 14,456 |

These are recorded calibration snapshots, not universal acceptance thresholds. Five attachments were located in the prior chat, and the EOU JSON was downloaded and checked during this recovery. Recovery of the other four local copies is still pending.

Use saved metadata and offline replay for future categorization analysis. Routine tuning must not require another five-ROM extraction/upload cycle or restore the rejected folder routing.

## Validation and release expectations

Success means supported decrypted inputs produce correct, usable textures and a working emulator pack, while protecting user edits. Legacy Python fingerprint parity is not a required release gate, and branch-protection administration is not an unfinished product milestone.

Developer checks should focus on real failure modes: malformed ranges/archives, non-8-aligned PICA dimensions, channel/alpha decoding, base-mip hash spans, missing embedded assets, false-positive detection, pack mappings, filename collisions, and transactional preservation of edited masters.

Before asking the user to test, provide an actual downloadable packaged Windows release candidate. Check GUI launch, extraction responsiveness, error display, output opening, and deployment generation. Prefer existing metadata, synthetic fixtures, and narrow regression cases over repeated broad retail-ROM calibration requests.

## Historical milestones and recovery limits

| Recorded stage | What the history describes |
| --- | --- |
| Python 0.1-0.12 | Untold-focused offline extractor; progressively corrected discovery, formats, hashing, alpha handling, deduplication, friendly names, and workspace preservation. |
| 0.13 | Legacy hardening after a static audit. |
| 0.20-0.50 | Native Rust foundation, ROM parsing, PICA decoding, and container work. |
| 0.60 | Native Untold extraction and Windows GUI release candidates. |
| 0.70 | Universal extraction/coverage work and reported visible replacements for the five Atlus titles. |
| 0.71 | Categorization diagnostics and M1-M27 work, including offline calibration replay. |
| 0.72 | GUI automatic trusted routing and battlefield-name fixes. |
| **0.73** | **Routing removed; familiar folder layout restored. Final recovered stable baseline.** |

The last recorded release commit is `807e094572f9aea8896b8192db41de6e359d3b1f`; the routing rollback merge is `cc0ea53f8d40c09953dfc3b9b238b6265f87d8c5`. These identify the historical lost repository, not commits present in this repository.

The recovered conversations are: **EO Untold Modding Feasibility**, **Repository audit findings**, **Status update**, **Progress update**, **0.60 Release Status**, **Compare Universal Gap Audits**, **Merge 070**, **Improve Texture Categorization**, and **Continue Review Update**. Only EO-Texrip project material was used. Private conversation links and raw transcripts are excluded from this public brief.

No current Rust source or historical Windows binary has been recovered into this repository. The chat history also shows an earlier downloadable Python 0.12 ZIP; that attachment has not been recovered in this pass. It may be reference material if still available, but the rebuild must follow the final native-app requirements rather than treating Python parity as the objective.
