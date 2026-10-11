# EO-Texrip 0.1.0-alpha.2

Untold 1 extraction no longer stops on an HPI save-data placeholder such as
`SAVEDATA/MO1R04_GAME.SAV` outside the HPB payload. These save entries are
reported and skipped. Out-of-bounds texture/resource entries remain errors.

Extracted PNGs now use upright artwork row order in all 14 PICA formats,
including ETC1 and ETC1A4. Runtime hashes still cover the original base-mip bytes.
Azahar's `flip_png_files=true` remains necessary for its GPU loading convention.

Load an existing alpha.1 workspace and rebuild its emulator pack to upgrade it.
Unedited originals are repaired once. Already-flipped originals are recognized.
Edited/upscaled masters retain their bytes, with an upright deployment copy and
an orientation review item. After fixing an editable master yourself, confirm
**PNG orientation: Upright**. See `USER_GUIDE.md` for the complete upgrade steps.

Regression fixtures cover the reported save entry, strict resource bounds,
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
