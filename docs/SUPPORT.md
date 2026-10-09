# Support boundaries for 0.1.0-alpha.1

The rebuild is a usable native alpha. Historical tests of the lost application
are not tests of this implementation.

| Area | Implemented | Validation / remaining work |
| --- | --- | --- |
| Decrypted NCSD, NCCH, CIA, RomFS | Bounded file traversal and title detection | Synthetic end-to-end fixtures; no current retail input available |
| HPI/HPB + ACMP | Native index/member reads, bounded backward decompression | Index extraction and overlapping backward runs with raw prefix tested; additional real compressed variants needed |
| FARC/SIR0 | Structural outer/FAT tables | Generic synthetic fixture; game-specific variants need validation |
| STEX | Combined GL type/format, named base-mip payload | Synthetic channels, orientation, bounds and hash-span tests |
| CGFX | Typed image TXOB descriptor/pointers | Synthetic models; one bad entry preserves valid entries and reports an issue |
| BCH | Texture table and masked/consecutive GPU command binding | Synthetic model; raw-extension relocation and cube textures remain gaps |
| BCFNT | Typed font sheets | Structural implementation; font-file variants need additional coverage |
| CTPK | Standard 2D entries only | Unsupported cube/disguised entries are reported |
| PICA | All fourteen formats, padded 8×8 storage, Morton layout, alpha | Synthetic format vectors and output orientation; retail pixel validation pending |
| Materials / separate alpha | Embedded alpha is preserved | No guessed combination of standalone masks; material-binding recovery remains work |
| TMX, TTD, TGD | No complete native implementation | Reported as unsupported when encountered; no speculative runtime hashes |
| Five mainline 3DS games | Profile-scoped rules and shared structural parsers | All five saved metadata captures replayed; rebuilt game extraction/replacement not yet certified |
| Mystery Dungeon 1 / 2 | Visible research targets | Extraction disabled until independently validated |
| Windows desktop | Dark GUI, file/folder selection, drag/drop, background jobs, previews, corrections; WGPU renderer on Windows | Headless egui render test; native launch/exit is a packaging gate. Full manual interaction still needs validation |
| Masters and deployment | Persistent identities, dedup hashes, saved corrections, staged publication | Edited/upscaled preservation, manual rename, failed-input and recovery tests |

Unsupported entries appear in the issue list. Previously extracted masters
that a partial rerun does not rediscover are retained with preservation issues.
Do not interpret a nonzero texture count as proof that every asset was extracted.

Classification coverage is reported independently from accuracy. No complete
human-verified gold dataset or exhaustive English entity-name database has been
recovered. Confirmed corrections provide a durable exact-asset name map.
