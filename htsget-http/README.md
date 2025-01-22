# htsget-http

[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/umccr/htsget-rs/blob/main/LICENSE
[actions-badge]: https://github.com/umccr/htsget-rs/actions/workflows/action.yml/badge.svg
[actions-url]: https://github.com/umccr/htsget-rs/actions?query=workflow%3Atests+branch%3Amain

Framework independent code for handling HTTP in [htsget-rs].

[htsget-rs]: https://github.com/umccr/htsget-rs

## Overview

This crate handles all the framework independent code for htsget-rs, it:

* Produces htsget-specific HTTP responses.
* Converts query results to JSON HTTP responses.
* Handles htsget client error reporting.
* Uses [htsget-search] to calculate URL tickets and byte ranges.

## Usage

There is no need to interact with this crate for running htsget-rs.

### As a library

This crate is useful for implementing additional framework dependent versions of the htsget-rs server.
For example, htsget-rs could be written using another framework such as [warp]. This crate provides functions 
like `get`, `post` and `get_service_info_json` for this purpose. These functions take query and endpoint information,
and process it using [htsget-search] to return JSON HTTP responses.

#### Feature flags

This crate has the following features:
* `aws`: used to enable `S3` location functionality and any other AWS features.
* `url`: used to enable `Url` location functionality.
* `experimental`: used to enable experimental features that aren't necessarily part of the htsget spec, such as Crypt4GH support through `C4GHStorage`.

[warp]: https://github.com/seanmonstar/warp
[htsget-search]: ../htsget-search

## License

This project is licensed under the [MIT license][license].

[license]: LICENSE