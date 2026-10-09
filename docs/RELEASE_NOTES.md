This is the first working native rebuild, version 0.1.0-alpha.1. The lost
0.73.0 binaries and source were not recovered; this release uses a new version
series and does not inherit historical compatibility claims.

Extract the ZIP and open `EO-Texrip.exe`. No Python, Cargo, or Git installation
is required. A small synthetic `Demo` workspace is included.

The app provides decrypted ROM/RomFS/resource-folder extraction, all 14 PICA
formats, flat categories, readable source-based names, PNG previews, evidence
review, persistent confirmed corrections, protected editable masters, and
Azahar pack rebuilding. CI exercises Linux and Windows tests and the packaged
Windows CLI with synthetic data, then opens and closes the native desktop.

Classification was replayed against 14,456 recovered metadata records from the
five mainline 3DS games. The aggregate report records assignments and unresolved
reviews; these are coverage measurements, not verified accuracy percentages.

Known limits: retail-ROM pixel and in-emulator validation of this rebuild is
pending. TMX, nonstandard CTPK payloads, some archive variants, material-linked
separate alpha masks, and Mystery Dungeon profiles are not claimed as complete.
Uncertain identities remain reviewable; filenames do not invent English enemy
or NPC names. See `docs/SUPPORT.md` and `docs/USER_GUIDE.md`.
