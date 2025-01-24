# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.13.0...htsget-config-v0.13.1) - 2025-01-24

### Other

- update Cargo.lock dependencies

## [0.13.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.12.0...htsget-config-v0.13.0) - 2025-01-24

### Added

- [**breaking**] implement encryption scheme logic in storage
- *(config)* remove a few usages of cfg and add defaults
- add encryption scheme to http and config crates
- *(config)* add cargo package filled service info fields
- [**breaking**] add pre-filled package info, description and repository url to the service info endpoint
- *(config)* implement path-based locations

### Fixed

- *(config)* parser should ignore `HTSGET_CONFIG` because that gets picked up by the clap Args parser
- *(config)* locations prefix should not be appended to the id so that there is not a double prefix
- service info group, artifact and version, and add flexibility in configuration

### Other

- add test for encryption scheme flag
- [**breaking**] remove deploy directory in favour of https://github.com/umccr/htsget-deploy, keep docker inside docker directory
- *(config)* add details to service_info config specifying custom values and overriding defaults
- *(config)* add test for service info
- use `--all-features` in config examples
- rename s3-storage to aws and url-storage to url
- *(config)* ensure all examples and example files are working
- update docs for path-based and regex config
- move cors into advanced and add support for mirrored headers and methods
- add location concept and move advanced config to its own module
- grammar and typos
- fix new clippy warnings
- re-word and simplify, add quick starts where applicable

## [0.12.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.11.0...htsget-config-v0.12.0) - 2024-10-16

### Added

- *(config)* allow retrieving secret from binary or string
- allow retrieving c4gh keys from secrets manager

### Fixed

- *(storage)* ensure c4gh data is read after determining the header size

### Other

- *(deploy)* add options to deploy C4GH keys to secrets manager
- *(config)* add section on using Secrets Manager to fetch keys

## [0.11.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.10.1...htsget-config-v0.11.0) - 2024-09-30

### Added

- *(config)* add configuration values for C4GH S3 and Url storage
- Crypt4GH support using LocalStorage

### Fixed

- explicitly choose aws_lc_rs as the crypto provider

### Other

- *(deps)* bump noodles and tower
- remove `ObjectType` in favour of a more flattened config hierarchy
- rename type to backend and clarify experimental feature flag
- *(config)* slight adjustment to reduce conditionally compiled code branches
- [**breaking**] make storage config more explicit
- *(config)* [**breaking**] remove object type and specify keys directly
- bump noodles and other dependencies
- [**breaking**] rename c4gh-experimental to experimental
- add more server tests and fix Dockerfile.dockerignore

## [0.10.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.10.0...htsget-config-v0.10.1) - 2024-08-04

### Fixed
- *(axum)* enable http2 support, re-word docs to include htsget-axum

### Other
- update rust msrv
- bump Lambda and noodles dependencies

## [0.10.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.9.0...htsget-config-v0.10.0) - 2024-05-22

### Other
- major dep updates, rustls 0.21 -> 0.23, http 0.2 -> 1, reqwest 0.11 -> 0.12, noodles 0.65 -> 0.74 + minor bumps for other crates depending on these.
- Merge branch 'main' of https://github.com/umccr/htsget-rs into mio_h2_bump

## [0.9.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.8.1...htsget-config-v0.9.0) - 2024-05-19

### Added
- UrlStorage forwarded header blacklist

### Fixed
- *(config)* clippy warning

### Other
- document UrlStorage header_blacklist option
- update MSRV
- *(config)* update examples and add openssl certificate example
- [**breaking**] swap out hyper for reqwest to support redirects
- simplify default resolvers and local storage data
- add links
- add full minio example, rearrange location of compose.yml examples
- remove S3 default on s3-storage feature flag, reword docs for compose deployment
- *(deps)* update noodles to 0.65

## [0.8.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.8.0...htsget-config-v0.8.1) - 2024-01-02

### Other
- *(deps)* further minor dependency changes
- *(deps)* update noodles to 0.60, new clippy warnings

## [0.8.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.7.1...htsget-config-v0.8.0) - 2023-11-02

### Added
- *(search)* remove response scheme because response url can specify it
- *(config)* add response url to config

### Fixed
- *(s3)* properly handle first capture group bucket and add warnings for s3 errors

## [0.7.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.7.0...htsget-config-v0.7.1) - 2023-09-06

### Other
- *(deps)* properly update noodles and aws dependencies
- *(deps)* update msrv and attempt using htsget-test as a dev dependency
- bump deps including rustls 0.21
- Merge branch 'main' of https://github.com/umccr/htsget-rs into feat/htsget-elsa

## [0.7.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.6.2...htsget-config-v0.7.0) - 2023-07-11

### Added
- move hyper client construction to config and copy it to url storage
- *(config)* [**breaking**] unflatten tls config making it common across all structs that use it
- [**breaking**] implement client tls config
- [**breaking**] add server config to certificate key pair
- [**breaking**] add stronger types for certificate key pairs
- introduce cert and key parsing into config

### Fixed
- add hyper-rustls for all features

### Other
- *(config)* update docs for combined TLS config
- *(config)* add tls client config test
- *(config)* fix tests with new server config structs
- *(config)* use parsed certificates and keys directly in tests
- update for http client config

## [0.6.2](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.6.1...htsget-config-v0.6.2) - 2023-06-20

### Other
- bump deps

## [0.6.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.6.0...htsget-config-v0.6.1) - 2023-06-19

### Fixed
- avoid overwriting forwarded headers in url when formatting response

### Other
- add tests for extending urls and headers
- *(config)* make example url storage config runnable with default config

## [0.6.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.5.0...htsget-config-v0.6.0) - 2023-06-08

### Other
- nightly clippy suggestions
- *(config)* avoid double new type struct by using named inner field and transparent container attribute
- use http::Uri directly in config
- update remaining dependencies, hold back tokio-rustls due to conflicting versions with actix-web
- update noodles, remove repeated code in search

## [0.5.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.4.0...htsget-config-v0.5.0) - 2023-06-02

### Fixed
- *(config)* add default values to url storage

### Other
- add example config files

## [0.3.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.2.0...htsget-config-v0.3.0) - 2023-05-29

### Added
- implement url formatting for url storage
- *(config)* use proper url parsing in config
- add option to format logs in different styles
- add error type to config
- [**breaking**] add tls config to ticket server, rearrange some fields
- use serve_at in data server rather than a constant
- *(config)* set `Local` resolvers from data server config after parsing the `Config`
- *(config)* add automatic config inference for local and s3 storage, and rearrange modules
- [**breaking**] simplify storage config by allowing untagged enum representation

### Fixed
- inserting keys with the same name multiple times into headers serializes correctly
- local storage on windows no longer returns urls with incorrect paths
- *(config)* use set to avoid duplicate key-value pairs in headers
- [**breaking**] headers should allow multiple values for the same key
- use correct help context for a crate using htsget-config
- *(release)* Bump all crates to 0.1.2 as explored in https://github.com/MarcoIeni/release-plz/issues/507#issuecomment-1422187766

### Other
- *(config)* clarify which HTTP requests `UrlStorage` will make
- *(config)* update docs for more clarity
- update for UrlStorage
- Merge branch 'main' of https://github.com/umccr/htsget-rs into feat/url_storage
- [**breaking**] rename AwsS3Storage to S3Storage in search
- [**breaking**] http refactor, pass request with query
- remove s3-storage as default
- *(config)* rename ResolveResponse functions
- *(config)* add logic for url storage in resolvers
- Merge branch 'main' of https://github.com/umccr/htsget-rs into feat/url_storage
- *(config)* add documentation for endpoint when using S3 storage
- Merge branch 'main' of https://github.com/umccr/htsget-rs into feat/actix-tls
- *(config)* fix typo
- *(config)* rename test
- *(config)* add tests for resolving responses
- *(config)* fix incorrectly using resolved id when searching for regex capture groups
- a few style changes, changed default resolver
- *(config)* add tests for default tagged storage type
- *(config)* document new features and how to use them
- [**breaking**] rename `HttpTicketFormatter` and remove `UrlFormatter` implementation for it
- [**breaking**] move htsget structs to config, and resolve storage type in config
- *(config)* leverage tagged enum types to allow selecting storage type without manually specifying config values
- manually bump crate versions to 0.1.4
- bump crate versions to 0.1.3 manually
- [**breaking**] move CertificateKeyPair to config to simplify data server logic
- release
- Downgrade release after fixing cargo publish circular dep issues as discussed in https://github.com/MarcoIeni/release-plz/issues/507#issuecomment-1420254400
- Update cargo files ([#152](https://github.com/umccr/htsget-rs/pull/152))
- release ([#148](https://github.com/umccr/htsget-rs/pull/148))
- Set MSRV on all sub-crates ([#146](https://github.com/umccr/htsget-rs/pull/146))
- Better CI ([#98](https://github.com/umccr/htsget-rs/pull/98))
- add missing environment variable options
- remove unnecessary default column for environment variables, surround environment variables in backticks.
- reword resolvers description
- clarify how the resolvers work
- reword usage string
- remove duplicate config module
- Merge branch 'main' of https://github.com/umccr/htsget-rs into more-flexible-config-rename
- fix feature flag compile errors
- add test for long resolvers from environment variable config
- fix broken data server optional by introducing boolean flag to enable data server
- add documentation for reworked config
- flatten data server config
- clippy and fmt
- update config file with default values, add option to print a default config
- allow specifying tags, reference names, fields with an 'All' value
- remove setters, add constructors, add documentation.
- reduce some options for cors, remove repeated code when configuring cors
- fix tests affected by config, change some default values and move around config options
- remove custom deserializer for None option and instead use custom enum
- fix logic involving allowed attributes
- apply changes to other crates from reworked config
- update getter return types
- deserialize empty string as None value
- add cors tests and environment variable tests
- remove public fields, add public getters
- allow configuring multiple data servers
- add expose headers cors option
- move cors config to separate file
- add case insensitive aliases to enum variants
- add allow origins, and separate out tagged and untagged enum variants
- add generic allow type configuration option for allow headers and allow methods
- add cors max age option
- add cors allow header types for cors config
- add CorsConfig shared struct
- add UrlResolver, separate data server config from resolver
- fix errors relating to new config
- use figment instead of config because it is simpler to set defaults
- move config into separate module
- add tests for checking for contained value in interval
- implement query matcher logic
- move fields, tags, no tags, query, and interval to config
- add separate config for local server and s3 storage
- add config file from command line or env option
- swap out envy for config dependency
- Add documentation for cors
- Implement configurable origin for cors
- Add cors allow credentials option to data server config
- Move data server config to separate struct
- Move server config into separate struct
- Implement cors for htsget-http-actix.
- Add more spans and tracing calls.
- Move tracing setup to config.
- Bump dependencies, fix clippy warnings.
- Improve errors so that they are more informative.
- Update README instructions.
- Add ticker server addr test.
- Clean up code, format, update dependencies.
- Implement non-tls ticket server alongside tls ticket server.
- Fix localstorage path ([#86](https://github.com/umccr/htsget-rs/pull/86))
- Fix tests and errors ([#83](https://github.com/umccr/htsget-rs/pull/83))
- Deploy htsget-http-lambda. ([#81](https://github.com/umccr/htsget-rs/pull/81))
- Enable choosing between storage types. ([#80](https://github.com/umccr/htsget-rs/pull/80))
- Remove file from localstorage ([#79](https://github.com/umccr/htsget-rs/pull/79))
- Htsget http lambda ([#76](https://github.com/umccr/htsget-rs/pull/76))

## [0.2.0](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.1.4...htsget-config-v0.2.0) - 2023-04-28

### Added
- use serve_at in data server rather than a constant
- *(config)* set `Local` resolvers from data server config after parsing the `Config`
- *(config)* add automatic config inference for local and s3 storage, and rearrange modules
- [**breaking**] simplify storage config by allowing untagged enum representation

### Other
- *(config)* fix typo
- *(config)* rename test
- *(config)* add tests for resolving responses
- *(config)* fix incorrectly using resolved id when searching for regex capture groups
- a few style changes, changed default resolver
- *(config)* add tests for default tagged storage type
- *(config)* document new features and how to use them
- [**breaking**] rename `HttpTicketFormatter` and remove `UrlFormatter` implementation for it
- [**breaking**] move htsget structs to config, and resolve storage type in config
- *(config)* leverage tagged enum types to allow selecting storage type without manually specifying config values

## [0.1.1](https://github.com/umccr/htsget-rs/compare/htsget-config-v0.1.0...htsget-config-v0.1.1) - 2023-01-27

### Other
- Set MSRV on all sub-crates (#146)
- Better CI (#98)
- add missing environment variable options
- remove unnecessary default column for environment variables, surround environment variables in backticks.
- reword resolvers description
- clarify how the resolvers work
- reword usage string
- remove duplicate config module
- Merge branch 'main' of https://github.com/umccr/htsget-rs into more-flexible-config-rename
- fix feature flag compile errors
- add test for long resolvers from environment variable config
- fix broken data server optional by introducing boolean flag to enable data server
- add documentation for reworked config
- flatten data server config
- clippy and fmt
- update config file with default values, add option to print a default config
- allow specifying tags, reference names, fields with an 'All' value
- remove setters, add constructors, add documentation.
- reduce some options for cors, remove repeated code when configuring cors
- fix tests affected by config, change some default values and move around config options
- remove custom deserializer for None option and instead use custom enum
- fix logic involving allowed attributes
- apply changes to other crates from reworked config
- update getter return types
- deserialize empty string as None value
- add cors tests and environment variable tests
- remove public fields, add public getters
- allow configuring multiple data servers
- add expose headers cors option
- move cors config to separate file
- add case insensitive aliases to enum variants
- add allow origins, and separate out tagged and untagged enum variants
- add generic allow type configuration option for allow headers and allow methods
- add cors max age option
- add cors allow header types for cors config
- add CorsConfig shared struct
- add UrlResolver, separate data server config from resolver
- fix errors relating to new config
- use figment instead of config because it is simpler to set defaults
- move config into separate module
- add tests for checking for contained value in interval
- implement query matcher logic
- move fields, tags, no tags, query, and interval to config
- add separate config for local server and s3 storage
- add config file from command line or env option
- swap out envy for config dependency
- Add documentation for cors
- Implement configurable origin for cors
- Add cors allow credentials option to data server config
- Move data server config to separate struct
- Move server config into separate struct
- Implement cors for htsget-http-actix.
- Add more spans and tracing calls.
- Move tracing setup to config.
- Bump dependencies, fix clippy warnings.
- Improve errors so that they are more informative.
- Update README instructions.
- Add ticker server addr test.
- Clean up code, format, update dependencies.
- Implement non-tls ticket server alongside tls ticket server.
- Fix localstorage path (#86)
- Fix tests and errors (#83)
- Deploy htsget-http-lambda. (#81)
- Enable choosing between storage types. (#80)
- Remove file from localstorage (#79)
- Htsget http lambda (#76)
