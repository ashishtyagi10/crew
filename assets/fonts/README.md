# Lilex — crew's built-in typeface

crew embeds Lilex so it **never depends on what is installed on the machine**.
Without it, cosmic-text resolves `Family::Monospace` to `"Noto Sans Mono"`
(hardcoded in `cosmic-text/src/font/system.rs`) and, on a miss, falls through to
the platform's common fallback list — `Segoe UI` on Windows, a *proportional*
face. A proportional face in a cell grid with `set_monospace_width` rounds every
narrow advance (`i`, `l`, `.`, `|`) to zero, which is what made a fresh Windows
install look mangled through v0.17.8.

- Upstream: <https://github.com/mishamyrt/Lilex>
- Version: **2.700**
- License: SIL Open Font License 1.1 — see `OFL.txt` (copied verbatim from
  upstream `master`).

## Which files, and why these

OTF (CFF) statics, not the variable font: `fontdb` has no `fvar` support, so a
variable font registers as a single face at its default weight and every
`/weight` step would render identically. Statics give real weights.

`crew`'s base weight is configurable over 300–900 (`/weight`, default SemiBold
600) and bold cells always request 700, so four upright weights plus their
italics cover the range — CSS weight matching snaps 300 to Regular and 800/900
to Bold.

| File | Weight |
|---|---|
| `Lilex-Regular.otf`, `Lilex-Italic.otf` | 400 |
| `Lilex-Medium.otf`, `Lilex-MediumItalic.otf` | 500 |
| `Lilex-SemiBold.otf`, `Lilex-SemiBoldItalic.otf` | 600 |
| `Lilex-Bold.otf`, `Lilex-BoldItalic.otf` | 700 |

~840 KB total. To update, download the `Lilex.zip` release asset, copy those
eight files out of `otf/`, and refresh `OFL.txt` and the version above.
