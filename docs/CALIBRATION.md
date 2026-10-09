# Recovered metadata replay

All five original calibration bundles were recovered and replayed offline by
the production Rust classifier. Old output categories were excluded from its
inputs. Only aggregates are committed; the original captures remain private.

| Game | Records | Strong / confirmed assignments | Category reviews | Name reviews | Misc |
| --- | ---: | ---: | ---: | ---: | ---: |
| EOIV | 2158 | 2064 | 94 | 379 | 70 |
| EON | 3988 | 2892 | 1096 | 70 | 1052 |
| EOU | 2280 | 2157 | 123 | 420 | 90 |
| EOU2 | 2884 | 2753 | 131 | 562 | 98 |
| EOV | 3146 | 2358 | 788 | 18 | 770 |

These counts are **coverage, not accuracy**. They count metadata records, before
pixel deduplication. A strong evidence grade is not a verified correctness label.
Name reviews flag generic or coded identities; a readable internal label does
not establish a complete English entity dictionary.

Concrete rule regressions cover keyboard glyphs, facility backgrounds, enemy
effects, battlefield geometry, map icons, model-owner conflicts, and V/Nexus
embedded portrait paths. Flat archive names and embedded parent paths are
handled explicitly. Conflicting owners remain reviewable.

The remaining reviews, especially standalone artwork families in V/Nexus, need
verified labels or exact-asset corrections. The app preserves those source
identities and exposes previews after extraction. There is no request for
another five-ROM calibration run.

See [machine-readable aggregates](calibration-summary.json) and
[support boundaries](SUPPORT.md).
