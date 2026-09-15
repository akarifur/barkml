# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.9.0 (2025-09-15)

### Breaking Changes

 - **Removed:** TOML-style section headers (`[name]` / `["name"]`). Blocks are now the only
   structural grouping construct. Migrate by replacing each section header with a block:

   ```text
   # 0.8.x                      # 0.9.0
   [settings]                   settings {
   editor = "nvim"                  editor = "nvim"
                                }
   ```

   Parsing a legacy section header now fails with a migration-oriented diagnostic
   pointing to the `name { ... }` form. Array literals (`[1, 2]`), array type syntax
   (`array[string, int]`), and table values (`= { ... }`) are unaffected.

 - **Removed APIs:** `StatementType::Section`, `Statement::new_section`, and
   `Scope::get_sections` no longer exist. Use blocks (`Statement::new_block`,
   `StatementType::Block`, `Scope::get_all_blocks`) instead.

## 0.8.5 (2025-06-16)

### New Features

 - <csr-id-e8dececac6d4b98f3f0983b6ef60d832d9f28bab/> Add serde serialization module
   Add comprehensive serde serialization support for BarkML AST nodes:
   
   - Add ValueSerializer for serializing to BarkML Value types
   - Add StatementSerializer for serializing to BarkML Statement types
   - Support all Rust primitive types, collections, structs, and enums
   - Provide to_statement() and to_value() convenience functions
   - Include error handling with descriptive error messages
   - Add extensive test coverage for serialization scenarios
   
   This enables programmatic generation of BarkML AST from Rust data structures.
 - <csr-id-943c786a68e0f7a35e528fb04f8a12d5b9a42d1a/> add serde deserializer module for Statement and Value types
   Implement comprehensive serde deserialization support for BarkML AST nodes:
   
   - Add ValueDeserializer for deserializing from BarkML Value types
   - Add StatementDeserializer for deserializing from BarkML Statement types
   - Support all BarkML data types including primitives, arrays, tables, and enums
   - Provide from_statement() and from_value() convenience functions
   - Include comprehensive error handling with descriptive error messages
   - Add extensive test coverage for various deserialization scenarios
   
   This enables direct deserialization of Rust structs from parsed BarkML AST,
   making it easier to work with BarkML configuration data in Rust applications.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add serde serialization module ([`e8decec`](https://github.com/serates/barkml/commit/e8dececac6d4b98f3f0983b6ef60d832d9f28bab))
    - Add serde deserializer module for Statement and Value types ([`943c786`](https://github.com/serates/barkml/commit/943c786a68e0f7a35e528fb04f8a12d5b9a42d1a))
</details>

## 0.8.4 (2025-06-16)

<csr-id-d2122c41e9eca80b9876edeab2ff014a4529486f/>

### New Features

<csr-id-a24410701c7395b6c58b212ccb31955f477bf0f3/>
<csr-id-ea30b3d8fbe55e99811783989f582bd6d023a2c4/>

 - <csr-id-8a170cecbdb5a4c5ab6ef07d246c313d2ecc3bc5/> comprehensive improvements to load module
   - Enhanced StandardLoader with builder pattern and configuration management

### Bug Fixes

 - <csr-id-2ae8fb89cd70e08659a6d8dbcf3c4f874f0115ab/> boolean values having lower priority than identifiers

### Style

 - <csr-id-d2122c41e9eca80b9876edeab2ff014a4529486f/> Apply cargo fmt formatting

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 3 calendar days.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release barkml v0.8.4 ([`fd4eebf`](https://github.com/serates/barkml/commit/fd4eebf7ef2924f31d904ac36876ecf2f6da1200))
    - Release barkml v0.8.4 ([`1bb7e83`](https://github.com/serates/barkml/commit/1bb7e839751b08b9d39635b922c4ac22e680f8cd))
    - Apply cargo fmt formatting ([`d2122c4`](https://github.com/serates/barkml/commit/d2122c41e9eca80b9876edeab2ff014a4529486f))
    - Comprehensive improvements to load module ([`8a170ce`](https://github.com/serates/barkml/commit/8a170cecbdb5a4c5ab6ef07d246c313d2ecc3bc5))
    - Comprehensive improvements to AST module ([`a244107`](https://github.com/serates/barkml/commit/a24410701c7395b6c58b212ccb31955f477bf0f3))
    - Add recursion guard to prevent stack overflow attacks ([`ea30b3d`](https://github.com/serates/barkml/commit/ea30b3d8fbe55e99811783989f582bd6d023a2c4))
    - Boolean values having lower priority than identifiers ([`2ae8fb8`](https://github.com/serates/barkml/commit/2ae8fb89cd70e08659a6d8dbcf3c4f874f0115ab))
</details>

<csr-unknown>
Added LoaderConfig for centralized configuration optionsImplemented LoadStats for performance monitoring and statistics collectionAdded comprehensive utility functions for file handling and validationEnhanced Walk API with path navigation and dot notation supportImproved error handling with detailed context and validationAdded file caching system to avoid re-parsing identical filesEnhanced merge logic with better conflict resolutionAdded security-focused path validation and sanitizationImplemented comprehensive field inspection and navigation capabilitiesBuilder pattern for fluent loader configurationStatistics collection with processing time and memory usage trackingPath-based navigation (e.g., walker.get(‘config.database.host’))Enhanced debugging and introspection toolsValidation framework with early error detectionModule management with advanced manipulation capabilitiesIntelligent file caching systemBetter memory allocation strategiesOptimized data structures for common operationsLazy loading where possibleSome method signatures enhanced for better error handlingConfiguration now handled through LoaderConfig objectsEnhanced error types with additional contextEnhanced type system with TypeCategory enum and better compatibility checkingAdded memory tracking and performance monitoring capabilitiesImproved tree navigation with path-based lookup and recursive searchEnhanced macro resolution with recursion protection and better error handlingAdded comprehensive validation methods for statement structureImproved error messages with source code context and location informationAdded utility functions for AST analysis, memory profiling, and pretty printingEnhanced Display formatting with proper indentation across all typesAdded const methods for compile-time evaluation where possibleComprehensive test coverage for all new functionalitySome method signatures changed for better type safetyEnhanced error types with additional context fieldsReorganized internal data structures for better performanceAdd MAX_RECURSION_DEPTH constant (64 levels) to limit parser nestingAdd recursion_depth field to Parser struct for depth trackingAdd enter_recursion() and exit_recursion() guard methodsWrap recursive methods (value, value_type, statement, module) with guardsAdd RecursionLimit error type with location trackingAdd comprehensive tests for recursion limit enforcementMaintain backward compatibility for legitimate use cases<csr-unknown/>

## 0.8.3 (2025-06-10)

<csr-id-5c3f0800bc2dd43e9f750d9d11435dd83a34d31b/>
<csr-id-07393eac37f6fc542fc16a1a732a06c1b105896e/>
<csr-id-61a51e14d845fcaf689e5c95acd231c89fb0cb98/>
<csr-id-be6b9fcb574d30e6f109b83cedf8e0a1d680eaa5/>

### Chore

 - <csr-id-5c3f0800bc2dd43e9f750d9d11435dd83a34d31b/> update dependencies and Cargo.lock
   This commit updates several Rust dependencies to their latest versions, including:
   - Bump hashbrown to 0.15.4
   - Update snafu to 0.8.6
   - Upgrade uuid to 1.17.0
   - Add new dependencies like bumpalo and js-sys
   - Refresh Cargo.lock to reflect dependency changes
 - <csr-id-07393eac37f6fc542fc16a1a732a06c1b105896e/> improve error message clarity and formatting in error handling

### New Features

<csr-id-00589d25fd01c8bfe897d8a1c8aeee9b0e94d99a/>
<csr-id-14e38e02a7462f38cf14cff83a472cd5190ec4a5/>
<csr-id-e8a24d6ec53150e9c7be5a7cccabc83b6e85d2b6/>

 - <csr-id-9f4c382e88e9909a5607b21f1ca31129336a336a/> enhance BarkML loader documentation and improve code comments
   This commit improves the documentation for the BarkML loader modules by:
   - Adding comprehensive module and struct-level documentation

### Bug Fixes

 - <csr-id-b7a9d207f7b4b0d70c4029f4c4f67d35c2aa7f6a/> multiline strings were not lexing correctly

### Other

 - <csr-id-61a51e14d845fcaf689e5c95acd231c89fb0cb98/> print out line numbers and column numbers instead of character count
 - <csr-id-be6b9fcb574d30e6f109b83cedf8e0a1d680eaa5/> fix missing null and some labels

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release barkml v0.8.3 ([`72dcc56`](https://github.com/serates/barkml/commit/72dcc56ba17030eeb27453d304922e3b0b7c5276))
    - Update dependencies and Cargo.lock ([`5c3f080`](https://github.com/serates/barkml/commit/5c3f0800bc2dd43e9f750d9d11435dd83a34d31b))
    - Enhance BarkML loader documentation and improve code comments ([`9f4c382`](https://github.com/serates/barkml/commit/9f4c382e88e9909a5607b21f1ca31129336a336a))
    - Enhance AST module with improved documentation and structure ([`00589d2`](https://github.com/serates/barkml/commit/00589d25fd01c8bfe897d8a1c8aeee9b0e94d99a))
    - Improve error message clarity and formatting in error handling ([`07393ea`](https://github.com/serates/barkml/commit/07393eac37f6fc542fc16a1a732a06c1b105896e))
    - Enhance Location struct with more context and error reporting details ([`14e38e0`](https://github.com/serates/barkml/commit/14e38e02a7462f38cf14cff83a472cd5190ec4a5))
    - Enhance parser comment and token handling ([`e8a24d6`](https://github.com/serates/barkml/commit/e8a24d6ec53150e9c7be5a7cccabc83b6e85d2b6))
    - Simple update of dependencies to keep up to date ([`a4b59e7`](https://github.com/serates/barkml/commit/a4b59e73f60efb0a29733fbea1f30e0d6f8460f4))
</details>

## 0.8.2 (2025-03-07)

Fixes:

* Blocks are not treated as grouped in statement

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 22 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix statement grouped logic ([`b8140ff`](https://github.com/serates/barkml/commit/b8140ffdf7e0560e993506f69671ffc6efb49404))
</details>

## 0.8.1 (2025-02-13)

Fixes:

* Fixes implicit conversion from symbol to string

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.8.1 bugfixes ([`c1c2b78`](https://github.com/serates/barkml/commit/c1c2b780c7378fcb74f834ad4730ca7ec138accf))
</details>

## 0.8.0 (2025-02-13)

Changes:

* New get_child method in Walk for loading child statements
* Label's can no longer be a standalone value as this causes conflicts in parsing
* Introduces Symbol identifiers that start with :, this replaces using standalone label values

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.8.0 Symbols ([`a0b4972`](https://github.com/serates/barkml/commit/a0b49727519e485fe256573f29e27bf76a3dfea6))
</details>

## 0.7.0 (2024-08-19)

Changes:

* Introduces 128-bit integer types
* Refactors crate into a more maintainable structure
* Resplits Statements and Values for better data representation
* Improves error messages
* Implements a new Walker for easily reading and converting data
* Makes Statement and Value implement Serialize + Deserialize
* Consolidates comments and labels into new Metadata type

Removes:

* binary feature no longer supported, instead Value and Statement implement Serialize and Deserialize
  allowing users to use rmp-serde to serialize the AST types directly.


### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 4 calendar days.
 - 27 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump version and sync README and Changelog ([`296455c`](https://github.com/serates/barkml/commit/296455c1eecb4732108cf306b610e6f109017e00))
    - Fix testing with label change and missed float type bug ([`3c659bd`](https://github.com/serates/barkml/commit/3c659bd619ce5b4acf68cd5741e17beb792c00e6))
    - Fix injection ids and load operations ([`9c0d8e9`](https://github.com/serates/barkml/commit/9c0d8e98752ecff42710cc6085a36f3bc014a09f))
    - Consolidate error types now that we have a set ([`ddccbb3`](https://github.com/serates/barkml/commit/ddccbb388c7bcc1b0e0059b433d1d06beac40520))
    - Restructuring, track location on files through ast ([`56bd2a8`](https://github.com/serates/barkml/commit/56bd2a8f3a7f18484044db58a3e5b6fdb04ab543))
    - Migrate to new AST objects ([`6108479`](https://github.com/serates/barkml/commit/6108479d1edaca000db8841f7808aaa7375b4112))
</details>

## 0.6.8 (2024-07-22)

<csr-id-61a51e14d845fcaf689e5c95acd231c89fb0cb98/>

Changes:

* Moves the Lexer and Parser to keep track of line and column
* Updates StandardLoader to parse each module by itself for filename reporting

### Other

 - <csr-id-61a51e14d845fcaf689e5c95acd231c89fb0cb98/> print out line numbers and column numbers instead of character count

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Print out line numbers and column numbers instead of character count ([`61a51e1`](https://github.com/serates/barkml/commit/61a51e14d845fcaf689e5c95acd231c89fb0cb98))
</details>

## 0.6.7 (2024-07-22)

Fixes:

* Multiline strings were not properly lexing

### Bug Fixes

 - <csr-id-b7a9d207f7b4b0d70c4029f4c4f67d35c2aa7f6a/> multiline strings were not lexing correctly

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Multiline strings were not lexing correctly ([`b7a9d20`](https://github.com/serates/barkml/commit/b7a9d207f7b4b0d70c4029f4c4f67d35c2aa7f6a))
</details>

## 0.6.6 (2024-07-19)

<csr-id-be6b9fcb574d30e6f109b83cedf8e0a1d680eaa5/>

Fixes:

* Comments and Labels weren't propagated to a few value types
* Null was not being parsed nor reserved

### Other

 - <csr-id-be6b9fcb574d30e6f109b83cedf8e0a1d680eaa5/> fix missing null and some labels

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix missing null and some labels ([`be6b9fc`](https://github.com/serates/barkml/commit/be6b9fcb574d30e6f109b83cedf8e0a1d680eaa5))
</details>

## 0.6.5 (2024-07-19)

Version is bumping from a .0 -> .5 because there are no major changes in the language syntax but the lexer/parser has
been moved away from peg.

Changes:

* Implements a new token lexer using Logos
* New Recursive descent parser
* Improved multi-file handling as a single file in Multi mode
* Removes need for statements to be new-line delimited
* Commas are now optional in arrays and tables
* All assignment identifiers can be an identifier (no collision with a keyword) or a string
* Reserves keywords for future use

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 7 calendar days.
 - 51 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release 0.6.5 ([`13d69b6`](https://github.com/serates/barkml/commit/13d69b612677ef301409028fd57429dd4d9682a1))
    - New recursive descent parser ([`2266eb7`](https://github.com/serates/barkml/commit/2266eb75485200ebf5cbd93759592da4595366dc))
    - Migration to logos+chumsky complete ([`0893eeb`](https://github.com/serates/barkml/commit/0893eebbd5ba63b482c612f1b04f11e76f780871))
</details>

## 0.6.0 (2024-05-28)

Features:

* Reworks value structure to track id, label and even value type in a better way.
* Adds typing system and type hint to the language
* Upgrades macro resolution to a full scope system

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 25 calendar days.
 - 47 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Finish typing system ([`aa54b93`](https://github.com/serates/barkml/commit/aa54b9392fe9d2f393ec5e5f482391608c73673c))
    - Change block id to always include labels to distinguish them ([`b37dd5b`](https://github.com/serates/barkml/commit/b37dd5b6b5b0073cc134a766d2c518a9cf697b41))
    - Finish typing system ([`c715c05`](https://github.com/serates/barkml/commit/c715c05d9ed04a5cc466c45d7bfeb3db02a9cc9e))
    - Scope based macro resolution ([`a38bd86`](https://github.com/serates/barkml/commit/a38bd8691befe4275b96b1d82f67a10e796000de))
</details>

## 0.5.2 (2024-04-11)

Fixes:

* Switch StandardLoader to use BTreeMap so macro resolution doesn't indeterministically fail on module order pre-merge
  or append

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix standard loader map ordering ([`99f8161`](https://github.com/serates/barkml/commit/99f8161a12f8e605d2b98f763d5fc4157d3d5259))
</details>

## 0.5.1 (2024-04-11)

Fixes:

* read_dir causing inconsistent resolution of macros with multiple configuration files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix issue with inconsistent ordering with read_dir ([`7d5c780`](https://github.com/serates/barkml/commit/7d5c780a9a0bba202ee7da8c133f015ecf3bf4a1))
</details>

## 0.5.0 (2024-04-11)

Features:

* Added Module type

> Files in barkml are now loaded into a parent Value known as a Module. This allows better handling for multiple
> file loaders

* Moved definition of a configuration loader to a trait
* Added Semantic Version and Version Requirement types and support

> Users can now define standard semantic versions in their configuration files as well
> as version requirements

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 5 calendar days.
 - 8 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Support for versions and version req ([`9a291fb`](https://github.com/serates/barkml/commit/9a291fb16acc6cf8fc5c6b455572f3c593d2dd9e))
    - Introduce modules and standardize loader trait ([`f492f53`](https://github.com/serates/barkml/commit/f492f53dc4476a73f31277f7d3154276a4df3872))
</details>

## 0.4.1 (2024-04-03)

Fixes:

* Fix import issue in lib.rs
* Remove unimplemented feature

## 0.3.0 - 0.4.0

Features:

* Added config loader construct

> The Loader builder gives users the ability to load configuration files with more control. It also
> introduces an ability to load and merge multiple configuration files in a directory

* Added precision numeral values

> Users can now define a precision for integers using standard rust suffixes (u8, i8, u16, i16, u32, i32, u64, i64).
> Users can also define a precision for floating point numbers using suffixes (f32, f64)

Changes:

* Merged Value and Statement into one enum for easier use

## 0.2.0 - Binary encoding support

Added feature 'binary' that implements a conversion layer
to encode BarkML into a binary representation through the use
of `MessagePack`

## 0.1.0 - Initial Release

Initial release of barkml.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix import and unimplemented feature ([`296f846`](https://github.com/serates/barkml/commit/296f846699315c23e91fc989d0c9d4277bca1ea6))
</details>

## 0.4.0 (2024-04-03)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 7 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Configuration loader implementation ([`d6beb64`](https://github.com/serates/barkml/commit/d6beb642eb4bfe017845ad1e1d76d0a373ce9356))
    - Loader feature to allow multiple configuration files to be loaded at once ([`4ff4c8d`](https://github.com/serates/barkml/commit/4ff4c8d124bf2c27e33b23fed74dba804a5a1ed3))
    - Support for precision integers and floats ([`5fa8c8b`](https://github.com/serates/barkml/commit/5fa8c8b654df6e5cbf4ac1a0b4d4cca0ccd1c6e7))
    - Consolidate statement and value ([`094f82f`](https://github.com/serates/barkml/commit/094f82fa46413a168a8569f0eea5cc992623062a))
    - Create rust.yml ([`5773da7`](https://github.com/serates/barkml/commit/5773da74156861f140391eef92133fb3fe0ee6b1))
</details>

## 0.2.0 (2024-03-27)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 4 calendar days.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add binary encoding support through messagepack ([`4f341f3`](https://github.com/serates/barkml/commit/4f341f3ac7e197dffa270f848b84bd27256a3e62))
    - Update description ([`9c3db02`](https://github.com/serates/barkml/commit/9c3db026a03c5856c7d430bc4d1cc37e4acc62c0))
</details>

## 0.1.0 (2024-03-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Initial release of BarkML ([`9460e17`](https://github.com/serates/barkml/commit/9460e17c7a7dfb92f8dcafba2880009141716db5))
    - Initial commit ([`7999de1`](https://github.com/serates/barkml/commit/7999de1f5c179329ebe09e861a87993845b8413c))
</details>

