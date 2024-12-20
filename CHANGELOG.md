# Changelog

## [0.4.0](https://github.com/varfish-org/viguno/compare/v0.3.2...v0.4.0) (2024-11-20)


### Features

* proper OpenAPI-documented and versioned endpoints ([#227](https://github.com/varfish-org/viguno/issues/227)) ([#228](https://github.com/varfish-org/viguno/issues/228)) ([900858e](https://github.com/varfish-org/viguno/commit/900858e16ef9b5d3c0a266c9a62425f53f0aac98))

## [0.3.2](https://github.com/varfish-org/viguno/compare/v0.3.1...v0.3.2) (2024-07-14)


### Bug Fixes

* allow "OMIM:" and "MIM:" prefixes in omim_id param ([#178](https://github.com/varfish-org/viguno/issues/178)) ([#179](https://github.com/varfish-org/viguno/issues/179)) ([5cbd43d](https://github.com/varfish-org/viguno/commit/5cbd43d369867e612b0663cb2bd02fa7a40742f0))

## [0.3.1](https://github.com/varfish-org/viguno/compare/v0.3.0...v0.3.1) (2024-07-11)


### Bug Fixes

* remove path to hgnc_xlink.tsv from Docker entrypoint ([#176](https://github.com/varfish-org/viguno/issues/176)) ([f061c9d](https://github.com/varfish-org/viguno/commit/f061c9df36a17c08d0d02d2c9ede169c9836cc84))

## [0.3.0](https://github.com/varfish-org/viguno/compare/v0.2.1...v0.3.0) (2024-07-11)


### Features

* hgnc_xlink.tsv is expected now in hpo folder ([#170](https://github.com/varfish-org/viguno/issues/170)) ([#171](https://github.com/varfish-org/viguno/issues/171)) ([5c19d95](https://github.com/varfish-org/viguno/commit/5c19d95511df5b60af7c6134d5a93b54b07290a8))
* implement OpenAPI documentation ([#162](https://github.com/varfish-org/viguno/issues/162)) ([#165](https://github.com/varfish-org/viguno/issues/165)) ([b89fc26](https://github.com/varfish-org/viguno/commit/b89fc26d0f9c5555730b576aa5bac0a8ed9f2cf0))
* remove Phenix-like P-value precomputation ([#148](https://github.com/varfish-org/viguno/issues/148)) ([#163](https://github.com/varfish-org/viguno/issues/163)) ([3fb8be1](https://github.com/varfish-org/viguno/commit/3fb8be1c01d829768336c959b4b8bab8f3d975ab))


### Bug Fixes

* allow case insensitive search for OMIM terms ([#160](https://github.com/varfish-org/viguno/issues/160)) ([#172](https://github.com/varfish-org/viguno/issues/172)) ([0d0e5af](https://github.com/varfish-org/viguno/commit/0d0e5af1ce2ee553c5f28324d6ae0e96f126ca95))
* escape colons in term query ([#159](https://github.com/varfish-org/viguno/issues/159)) ([#173](https://github.com/varfish-org/viguno/issues/173)) ([5c40e30](https://github.com/varfish-org/viguno/commit/5c40e30ae5a9ac44b6ae1bddd14d5abcd4f61e30))
* proper error handling in /hpo/terms ([#156](https://github.com/varfish-org/viguno/issues/156)) ([#175](https://github.com/varfish-org/viguno/issues/175)) ([c0436de](https://github.com/varfish-org/viguno/commit/c0436de1a85eecd279876a4b6ae3e48442caae22))

## [0.2.1](https://github.com/varfish-org/viguno/compare/v0.2.0...v0.2.1) (2024-02-01)


### Miscellaneous Chores

* changed org (bihealth =&gt; varfish-org) ([#132](https://github.com/varfish-org/viguno/issues/132)) ([dacd570](https://github.com/varfish-org/viguno/commit/dacd57039962bf689fea2128a605d602dc236817))

## [0.2.0](https://github.com/varfish-org/viguno/compare/v0.1.6...v0.2.0) (2023-12-27)


### Features

* improve term search with full-text index ([#109](https://github.com/varfish-org/viguno/issues/109)) ([#112](https://github.com/varfish-org/viguno/issues/112)) ([9751e83](https://github.com/varfish-org/viguno/commit/9751e8329f6407b73e1309fe66367ebc8e539952))


### Bug Fixes

* make clippy happy about PartialOrd impls ([#96](https://github.com/varfish-org/viguno/issues/96)) ([da19a5f](https://github.com/varfish-org/viguno/commit/da19a5f122f51da83f344e3cf9857df3c5ebe5da))

## [0.1.6](https://github.com/varfish-org/viguno/compare/v0.1.5...v0.1.6) (2023-06-19)


### Bug Fixes

* docker build version in CI ([a3bdecf](https://github.com/varfish-org/viguno/commit/a3bdecf9c0d7d37ebd08c08c62b565cd576833b4))


### Build System

* some small fixes to CI ([#27](https://github.com/varfish-org/viguno/issues/27)) ([06177e7](https://github.com/varfish-org/viguno/commit/06177e7dc0fea7029adcd213b9b6a081184e9c3c))

## [0.1.5](https://github.com/varfish-org/viguno/compare/v0.1.4...v0.1.5) (2023-06-19)


### Build System

* fix docker builds ([#25](https://github.com/varfish-org/viguno/issues/25)) ([2bede82](https://github.com/varfish-org/viguno/commit/2bede822986380a3971c0cdf02fa85752565606b))

### [0.1.4](https://www.github.com/varfish-org/viguno/compare/v0.1.3...v0.1.4) (2023-06-17)


### Build System

* adjust Docker builds for PRs and branches ([#23](https://www.github.com/varfish-org/viguno/issues/23)) ([f4de844](https://www.github.com/varfish-org/viguno/commit/f4de844a14865573de303319d43e4e095fb7bdaf))

### [0.1.3](https://www.github.com/varfish-org/viguno/compare/v0.1.2...v0.1.3) (2023-06-16)


### Miscellaneous Chores

* re-release as v0.1.3 ([81257d8](https://www.github.com/varfish-org/viguno/commit/81257d8633686eac1d67d7d100ec12acd3d2636a))

### [0.1.2](https://www.github.com/varfish-org/viguno/compare/v0.1.1...v0.1.2) (2023-06-13)


### Bug Fixes

* make /hpo/genes endpoint return HGNC ID ([#16](https://www.github.com/varfish-org/viguno/issues/16)) ([#17](https://www.github.com/varfish-org/viguno/issues/17)) ([77bd9af](https://www.github.com/varfish-org/viguno/commit/77bd9af0cc498183d6c1172eebc7a3a08fcf3ebb))

### [0.1.1](https://www.github.com/varfish-org/viguno/compare/v0.1.0...v0.1.1) (2023-06-11)


### Build System

* provide Docker images from GitHub releases ([#6](https://www.github.com/varfish-org/viguno/issues/6)) ([#13](https://www.github.com/varfish-org/viguno/issues/13)) ([5df9de5](https://www.github.com/varfish-org/viguno/commit/5df9de59ff3734896279003be7ff0a10fa86ae5a))

## 0.1.0 (2023-06-11)


### Features

* expose similarity function of hpo crate ([#7](https://www.github.com/varfish-org/viguno/issues/7)) ([#9](https://www.github.com/varfish-org/viguno/issues/9)) ([e573957](https://www.github.com/varfish-org/viguno/commit/e57395713cdb8652b4ff945e40ca9a9142349a85))
* port over code from worker ([#1](https://www.github.com/varfish-org/viguno/issues/1)) ([#2](https://www.github.com/varfish-org/viguno/issues/2)) ([feaefc1](https://www.github.com/varfish-org/viguno/commit/feaefc11f0aa9c8732f593c6a85460c5267b7d61))
* use/provide HGNC gene IDs ([#10](https://www.github.com/varfish-org/viguno/issues/10)) ([#11](https://www.github.com/varfish-org/viguno/issues/11)) ([0a1da92](https://www.github.com/varfish-org/viguno/commit/0a1da923d3601d25d36dc000fe39af24eb1960c3))
