# SQL Inspector

A Node.js module that parses SQL queries and extracts information about referenced tables and columns.
This library is written in Rust and compiled to WebAssembly for use in Node.js applications.

## Features

- ✅ Extracts table and column names from SQL queries
- ✅ Supports `SELECT`, `INSERT`, `UPDATE`, `DELETE` statements
- ✅ Handles table aliases and fully-qualified column names
- ✅ WebAssembly interface for JavaScript/Node.js
- ❌ Does not support DDL statements (`CREATE TABLE`, `ALTER TABLE`, etc.)
- ❌ Cannot resolve ambiguous column references without database schema

## Limitations

In some cases, it's not possible to "resolve" which table a column belongs to. For example:

```sql
SELECT address, name FROM table1 JOIN table2 ON table1.id = table2.id
```

This query is ambiguous because we don't know if the `address` and `name` columns are from `table1` or `table2`. We can't resolve this without access to the actual database schema.

## Installation

```bash
npm install @platformatic/sql-inspector
```

## Usage

```javascript
const { sqlinspector } = require("@platformatic/sql-inspector");

// Basic SELECT query
const result = sqlinspector("SELECT name, id FROM users WHERE age > 30");
console.log(result);
// Output: {
//   columns: ["age", "id", "name"],
//   tables: ["users"],
//   query_type: "SELECT",
//   target_table: ""
// }

// Wildcard queries (columns are not expanded)
const wildcardResult = sqlinspector("SELECT * FROM users u");
console.log(wildcardResult);
// Output: {
//   columns: ["*"],
//   tables: ["users"],
//   query_type: "SELECT",
//   target_table: ""
// }

// INSERT statement
const insertResult = sqlinspector(
  "INSERT INTO users (id, name) VALUES (1, 'John')",
);
console.log(insertResult);
// Output: {
//   columns: ["users.id", "users.name"],
//   tables: ["users"],
//   query_type: "INSERT",
//   target_table: "users"
// }

// UPDATE statement
const updateResult = sqlinspector("UPDATE users SET age = 30");
console.log(updateResult);
// Output: {
//   columns: ["users.age"],
//   tables: ["users"],
//   query_type: "UPDATE",
//   target_table: "users"
// }

// DELETE statement
const deleteResult = sqlinspector("DELETE users WHERE age > 30");
console.log(deleteResult);
// Output: {
//   columns: ["age"],
//   tables: ["users"],
//   query_type: "DELETE",
//   target_table: ""
// }
```

## API Reference

### `sqlinspector(sql: string): ExtractResult`

Parses a SQL query string and returns information about referenced tables and columns.

#### Parameters

- `sql` (string): The SQL query to analyze

#### Returns

`ExtractResult` object with the following properties:

- `columns` (string[]): Array of column names found in the query. May include table prefixes (e.g., `"users.name"`) for INSERT/UPDATE operations
- `tables` (string[]): Array of table names referenced in the query
- `query_type` (string): Type of SQL operation - one of `"SELECT"`, `"INSERT"`, `"UPDATE"`, or `"DELETE"`
- `target_table` (string): The primary table being modified (for INSERT/UPDATE operations). Empty string for SELECT/DELETE operations

#### Examples

```javascript
// SELECT query
sqlinspector("SELECT name FROM users WHERE age > 18");
// Returns: {
//   columns: ["age", "name"],
//   tables: ["users"],
//   query_type: "SELECT",
//   target_table: ""
// }

// INSERT query
sqlinspector("INSERT INTO products (name, price) VALUES ('item', 10)");
// Returns: {
//   columns: ["products.name", "products.price"],
//   tables: ["products"],
//   query_type: "INSERT",
//   target_table: "products"
// }
```

## Development

Prerequisites:

- Rust toolchain: https://www.rust-lang.org/tools/install
- wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
- cargo make: https://github.com/sagiegurari/cargo-make

### Testing

The project includes both Rust unit tests and JavaScript integration tests to verify the WASM interface works correctly.

To run all tests:

```bash
cargo make test
```

To run only Rust tests:

```bash
cargo test
```

To run only JavaScript tests:

```bash
node --test ./test/index.test.js
```

**Note:** The build uses `--no-opt` flag to skip wasm-opt optimization due to compatibility issues with bulk memory operations in the current wasm-opt version bundled with wasm-pack. This is a known issue when using modern Rust compilers (1.79+) with wasm-pack 0.13.x.

### Building

To build the WebAssembly module:

```bash
cargo make build
```

To build just the Rust library:

```bash
cargo build
```

### Publishing

1. Build the WASM package: `cargo make build`
2. Navigate to the generated package: `cd pkg`
3. Login to NPM: `npm login`
4. Publish: `npm publish --access public` (by default, scoped packages are published with private visibility).

The package is published under: https://www.npmjs.com/package/@platformatic/sql-inspector

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Ensure tests pass: `cargo make test`
5. Commit your changes (`git commit -am 'Add some amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs) for SQL parsing
- Uses [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) for WebAssembly bindings
