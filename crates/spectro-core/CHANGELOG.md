# Changelog

## [0.3.8](https://github.com/Tinnci/spectro-rs/compare/spectro-rs-v0.3.7...spectro-rs-v0.3.8) (2026-01-24)


### Features

* Add CIE 2015 Observer and Native Display Calibration ([d6c901e](https://github.com/Tinnci/spectro-rs/commit/d6c901e1128db54502ba5c26effb7682bd39e0f0))
* Add VCGT control and Target Generator ([ff95a7f](https://github.com/Tinnci/spectro-rs/commit/ff95a7f7d83f835218902c431d960b18497a0cf4))
* **display:** Update NativeDisplay to use centered patch window ([365b4cb](https://github.com/Tinnci/spectro-rs/commit/365b4cbb2eb6af2e62ec177d50fb43a1df852b71))
* Implement Advanced Target Generation and Integrate VCGT ([7028068](https://github.com/Tinnci/spectro-rs/commit/7028068291d365ede2d6cd2db0449bcec142e49b))


### Bug Fixes

* **core:** Adjust photometric constant Km for mW input ([d3ea34c](https://github.com/Tinnci/spectro-rs/commit/d3ea34c25a4037807683218467047af5d15eb144))
* **core:** Fix physics model bias application in DSP ([4649f5f](https://github.com/Tinnci/spectro-rs/commit/4649f5f6d3104ca095354a068b5fee4b900d14b0))
* **core:** Use correct CoreGraphics symbol CGSetDisplayTransferByTable ([373d941](https://github.com/Tinnci/spectro-rs/commit/373d941900db6a398e4722ae72bcd7adfb95deb5))

## [0.3.7](https://github.com/Tinnci/spectro-rs/compare/spectro-rs-v0.3.6...spectro-rs-v0.3.7) (2026-01-23)


### Features

* **gui:** add sensor diagnostics view and linearity testing ([25734ac](https://github.com/Tinnci/spectro-rs/commit/25734ac130ae87e08d2d37abe3fbb7748249b0fc))
* implement luminance measurement and session-based calibration workflow ([2e3e7cb](https://github.com/Tinnci/spectro-rs/commit/2e3e7cb895ad9ccc79d3f7446c0cebdef0c53bb9))
* implement modular view system and display calibration workflow ([d0b26a3](https://github.com/Tinnci/spectro-rs/commit/d0b26a3801fa073ef7995a9ba9dda69bfc3e3536))
* **munki:** align driver with professional standards (ArgyllCMS) ([e88e864](https://github.com/Tinnci/spectro-rs/commit/e88e864c59872dcf9f8d30e0ca38bf5dd4fa24f7))
* **munki:** complete physics-based closed-loop calibration ([314a340](https://github.com/Tinnci/spectro-rs/commit/314a340cc41c15cd767ed16e473610a551394bcf))
* **munki:** implement advanced spectral calculation fixes ([adfd497](https://github.com/Tinnci/spectro-rs/commit/adfd497b06fe3451670a6066b1203e26b21be125))
* **munki:** implement physics-based sensor characterization ([e1fc364](https://github.com/Tinnci/spectro-rs/commit/e1fc364d76ca4f61f02d3be9f6c34f5224a1a919))
* **munki:** implement robust auto-exposure with SRP architecture ([1be729d](https://github.com/Tinnci/spectro-rs/commit/1be729dbb1af15b499b641a69b4b501361a4f985))
* **munki:** integrate advanced DSP diagnostics into GUI ([41f6aba](https://github.com/Tinnci/spectro-rs/commit/41f6aba7a35549053906767410a3255c4c680bb6))
* **munki:** integrate physics-based linearizer into DSP pipeline ([450f387](https://github.com/Tinnci/spectro-rs/commit/450f387a31ca53b839329e4dc73faa387e9e8e60))
* upgrade Expert View and History Panel ([a095b1c](https://github.com/Tinnci/spectro-rs/commit/a095b1cf70d4db7c037fb6b0f6e4f79f8fd06762))
