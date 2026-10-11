# EO-Texrip 0.1.0-alpha.3

Alpha.2 exempted only save-data placeholders. That was too narrow: Untold 1
also reported `SND/MORI1R/STREAM/NBGM_BATTLE2.DSPADPCM.BCSTM` outside the HPB.
Alpha.3 handles unavailable entries individually, regardless of namespace.
An invalid byte range is never read; its path, offset, stored size, and HPB
size are recorded in Extraction issues and the saved report. Valid members
before and after it continue to extract.

Known audio, video, and save files are excluded from texture decoding. Members
over the decode budget or with invalid ACMP data are reported individually.
Unavailable texture members remain explicit coverage gaps, and previously
edited masters survive partial reruns. Malformed index tables/names, missing
HPB files, cancellation, job-wide limits, and zero-texture runs remain fatal.

Extracted PNGs now use upright artwork row order in all 14 PICA formats,
including ETC1 and ETC1A4. Runtime hashes still cover the original base-mip bytes.
Azahar's `flip_png_files=true` remains necessary for its GPU loading convention.

Load an existing alpha.1 workspace and rebuild its emulator pack to upgrade it.
Unedited originals are repaired once. Already-flipped originals are recognized.
Edited/upscaled masters retain their bytes, with an upright deployment copy and
an orientation review item. After fixing an editable master yourself, confirm
**PNG orientation: Upright**. See `USER_GUIDE.md` for the complete upgrade steps.

Regression fixtures cover both reported save/sound entries, unavailable unknown
and texture entries, decode limits, corrupt members, partial rerun preservation,
strict index structure,
asymmetric rows in every format across tile boundaries, repeated workspace
upgrades, edited 16-bit upscales, and indexed PNG transparency. Windows packaging
also exercises the CLI and opens/closes the native GUI.

Parser behavior and orientation were checked against the original
[UntoldUnpack archive reader](https://github.com/xdanieldzd/UntoldUnpack/blob/2c5aebcc17a69262f5529f36c682d50f968a33ac/UntoldUnpack/Archive.cs)
and Azahar's pinned
[texture codec](https://github.com/azahar-emu/azahar/blob/5f3306d13cab40bbd5cf38f2cf3ae8b6e6eedca6/src/video_core/rasterizer_cache/texture_codec.h),
[dumping](https://github.com/azahar-emu/azahar/blob/5f3306d13cab40bbd5cf38f2cf3ae8b6e6eedca6/src/video_core/custom_textures/custom_tex_manager.cpp),
and [PNG loading](https://github.com/azahar-emu/azahar/blob/5f3306d13cab40bbd5cf38f2cf3ae8b6e6eedca6/src/video_core/custom_textures/material.cpp).
Full retail-ROM and in-emulator validation remains pending; this release does
not claim complete categorization or extraction coverage. Support gaps remain
documented in `SUPPORT.md`.
