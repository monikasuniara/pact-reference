# Reference implementation in Rust for the Pact Specification

[![Build Status](https://travis-ci.org/pact-foundation/pact-reference.svg?branch=master)](https://travis-ci.org/pact-foundation/pact-reference) [![Windows Build status](https://ci.appveyor.com/api/projects/status/bqlb7ny924lsu6yi?svg=true)](https://ci.appveyor.com/project/MichelBoudreau/pact-reference)

This is the project for a reference implementation of Pact in Rust. It implements the [V1 Pact specification](https://github.com/pact-foundation/pact-specification/tree/version-1).

There are 3 main modules to this implementation:

## [libpact_matching](libpact_matching)

This is a library that provides the Pact models and functions for matching requests and responses, as well as reading
and writing pact files.

## [libpact_mock_server](libpact_mock_server)

This is a library that provides an in-process mock server for Pact client tests. It uses the [libpact_matching](libpact_matching)
library.

## [pact_mock_server_cli](pact_mock_server_cli)

This module provides a command line executable that provides a standalone pact mock server and commands for controlling
the mock servers. It uses the [libpact_mock_server](libpact_mock_server) and [libpact_matching](libpact_matching)
libraries.