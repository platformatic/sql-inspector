# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

- Install dependencies: `cargo build`
- Run all tests: `cargo make test` (runs lint, Rust tests, WASM build, and JS tests)
- Run single Rust test: `cargo test tests::test_name`
- Run JavaScript tests only: `node --test ./test/index.test.js`
- Run linter: `cargo clippy`
- Fix linting errors: `cargo clippy --fix`
- Format code: `cargo fmt`

## High-level Code Architecture

### Overview
sql-inspector is a Rust library that parses SQL queries and extracts information about the tables and columns referenced. It's compiled to WebAssembly for use in Node.js applications.

### Core Components

1. **SQL Parsing**
   - Uses `sqlparser` crate (version 0.58.0) with the `visitor` feature
   - Supports `SELECT`, `INSERT`, `UPDATE`, `DELETE` statements
   - Does not support DDL statements like `CREATE TABLE`

2. **Visitor Pattern Implementation**
   - `V` struct implements the `Visitor` trait from sqlparser
   - Traverses the AST to extract table and column information
   - Handles table aliases and fully-qualified column names

3. **WASM Interface**
   - `inspect` function is the main entry point exposed to JavaScript
   - Uses `wasm-bindgen` for JS interop
   - Returns `ExtractResult` serialized as JSON

4. **Data Structures**
   - `ExtractResult`: Contains extracted columns, tables, and query type
   - `QueryType`: Enum for SELECT, INSERT, UPDATE, DELETE
   - `V`: Visitor implementation with collections for tables, columns, aliases

### Key Design Patterns
- Uses AST visitor pattern for SQL parsing
- Handles ambiguous column references (doesn't resolve without schema)
- Preserves wildcard selects as "*" without expansion
- Tracks table aliases for column resolution

### Limitations
- Cannot resolve ambiguous column references without database schema
- Wildcard selects (*) are not expanded to actual column names
- Only supports DML statements (SELECT, INSERT, UPDATE, DELETE)

## Code Style Guidelines

- Use standard Rust formatting with `cargo fmt`
- Follow Rust naming conventions (snake_case for functions/variables)
- Use explicit error handling with `Result` types where appropriate
- Prefer pattern matching over conditional chains
- Keep functions focused and single-purpose

## Type System

- Strong typing for SQL AST elements from sqlparser
- Serde serialization for WASM interface
- Uses `HashSet` for deduplication of tables/columns

## Runtime Requirements

- Rust toolchain (latest stable)
- wasm-pack for WebAssembly compilation
- cargo-make for build automation
- Node.js for JavaScript tests

## Build Notes

**Important:** The WASM build uses `--no-opt` flag to skip wasm-opt optimization due to compatibility issues with bulk memory operations in the current wasm-opt version bundled with wasm-pack. This is a known issue when using modern Rust compilers (1.79+) with wasm-pack 0.13.x.

## Testing

- Rust unit tests in `src/lib.rs` test core parsing functionality
- JavaScript integration tests in `test/index.test.js` verify WASM interface
- Tests cover all supported SQL statement types

## Dependencies

- `sqlparser`: SQL parsing with visitor pattern support
- `serde`: Serialization for WASM interface  
- `wasm-bindgen`: WebAssembly bindings for JavaScript
- `serde-wasm-bindgen`: Serde integration for WASM

## License

Apache License 2.0 - See LICENSE file for details.