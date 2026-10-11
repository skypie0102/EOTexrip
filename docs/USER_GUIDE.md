# Using EO-Texrip

## Start and extract

Extract the Windows ZIP and open `EO-Texrip.exe`. Choose your game, input, and
workspace. Inputs can be a decrypted `.3ds`/`.cci`, `.cia`, `.cxi`/`.app`, RomFS,
or extracted resource directory. A directory containing HPI indexes must also
contain the matching HPB files. Keep the workspace outside the input directory.

Title IDs are detected from ROM containers. Loose resources need the title ID
shown in Azahar's game properties to produce a deployment pack. Without it,
extraction still creates editable masters and reports the missing deployment.

To try the bundled synthetic demo, load `Demo/Workspace` using **Load workspace**,
or extract `Demo/Input` into a new folder with the Untold profile and the demo
ID `00040000000EC700`. The demo includes no game assets.

## Review and correct

The review list includes uncertain category assignments and unresolved names.
Use the category filter and search by original source or name. Select a texture
to see its PNG preview, original identities, runtime hashes, and evidence.

Enter a readable filename, choose its category, then **Queue confirmed correction**.
You may apply the category to every texture in that exact source model. Names
remain individual. **Save corrections and rebuild pack** publishes the changes
and saves the choices. Re-extraction uses the same confirmed identities.

**Analyze saved metadata** replays a historical calibration bundle without a
ROM. It produces decisions for inspection; it does not decode pixels. **Save
correction file** exports confirmed choices, and **Import correction file**
loads them into a workspace for publication. Save a metadata plan through the CLI
when a complete proposed catalog is needed.

Evidence grades describe the rule basis, not statistical accuracy. A strong
folder assignment does not automatically prove an English NPC/monster name.
`misc` contains unresolved/conflicting categories. Source codes are preserved
when a verified identity is unavailable.

## Edit and rebuild

Work on `azahar_pack_master/<category>/*.png`. You can edit or upscale the images;
the app preserves their bytes while improving a name or category. Keep the
`.eouhd` metadata folder together with the masters. Use **Rebuild emulator pack**
after editing to refresh `azahar_pack/<TITLE_ID>/`.

Use the GUI to rename or move a catalogued texture. If renaming it manually,
update its entry in the master's `pack.json` to its unique PNG basename before
rebuilding. That live mapping becomes a saved correction. Missing or ambiguous
files stop the rebuild; the app does not silently replace them with originals.
Additional valid custom hash mappings remain in the pack.

A damaged catalog is reported and existing images remain intact. Interrupted
publication is recovered when the workspace next opens. Back up a workspace
before making external bulk edits.

## Use in Azahar

Use Azahar's **Open Custom Texture Location** for the selected game. Copy the
contents of `azahar_pack/<TITLE_ID>/` into that game's texture folder, including
`pack.json` and the category directories. Enable custom textures in Azahar's
graphics settings. The app emits `use_new_hash=true`, `flip_png_files=true`,
and `skip_mipmap=false` with CityHash64 mappings.

On Steam Deck, transfer the generated title folder to the game's custom texture
location in your installed Azahar build. EO-Texrip performs extraction offline;
a playthrough is not required to gather texture dumps. This alpha's in-emulator
replacement behavior still needs validation on real game data.

`non-azahar/<category>/` is reserved for images without established runtime
mappings. They are excluded from deployment and should be excluded from the
normal emulator upscale batch.

## Upgrade an alpha.1 workspace

Use alpha.2 to **Load workspace**, then **Rebuild emulator pack**. You do not
need to extract the ROM again. Untouched alpha.1 PNGs are flipped upright once;
names, categories, and runtime hash mappings remain attached to the same assets.
An otherwise unchanged original that you already flipped is detected and kept.

If a legacy master has edited pixels or was upscaled, its bytes are preserved.
The deployment copy is flipped upright, and the asset appears in the review
list as **Legacy flipped**. To make its editable master upright too, flip it
vertically in your image editor, choose **PNG orientation: Upright** in EO-Texrip,
then queue and save the correction. If you already made an edited master
upright, confirm **Upright** without flipping it again. Orientation choices apply
to the selected image, even when applying a category to the whole source model.

Copy the rebuilt deployment folder into Azahar again. Keep
`flip_png_files=true`: Azahar applies its GPU row reversal when loading the
upright PNG. Changing this setting would undo the correction in the emulator.

For Untold 1, retry extraction with alpha.2. Out-of-payload `SAVEDATA/*.SAV`
entries are reported and skipped; texture members still require valid HPB ranges.

## Command line

```text
eotexrip.exe extract game.3ds --output workspace --game eou
eotexrip.exe extract resources --output workspace --game eov --title-id 0004000000000000
eotexrip.exe export workspace
eotexrip.exe apply workspace confirmed-names.json
eotexrip.exe inspect workspace
eotexrip.exe replay EOU-categorization-calibration-bundle.json --output proposed-catalog.json
eotexrip.exe demo --output demo-input
```

Profiles are `eou`, `eou2`, `eoiv`, `eov`, and `eon`. Mystery Dungeon profiles
remain research targets. The CLI and GUI use the same extraction/classification
logic. A workspace reports resources, decoded textures, unique images, reviews,
and issues in `.eouhd/<GAME>-extraction-report.json`.
