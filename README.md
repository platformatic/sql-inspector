# SQL Inspector

This extracts the columns and tables from a SQL query
It returns a ExtractResult struct with the columns and tables
Note that in some cases is not possible to "resolve" the columns' table.
For example:

```
"select address, name from table1 join table2 on table1.id = table2.id",
```

This query is ambiguous, because we don't know if the `address` and `name` columns are
from table1 or table2. We can't resolve this without the actual DB schema.

Examples:

```javascript
const res = sqlinspector("select name, id from users where age > 30;");
const expected = {
  columns: ["age", "id", "name"],
  tables: ["users"],
  query_type: "SELECT",
};
deepEqual(res, expected);
```

Not knowing the DB schema, we cannot solve wildcards either:

```javascript
const res = sqlinspector("select * from users u");
const expected = {
  columns: ["*"],
  tables: ["users"],
  query_type: "SELECT",
};
deepEqual(res, expected);
```

This library supports `SELECT`, `INSERT`, `UPDATE`, `DELETE` (so no DDL like `CREATE TABLE` ).

## Development

Prerequisites:

- Rust toolchain: https://www.rust-lang.org/tools/install
- wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
- cargo make: https://github.com/sagiegurari/cargo-make

### Run test

The actual tests are in rust, but there are also some (simple) JS test to test that the JS call works correctly.

To run all of them:

```
cargo make test
```

### Build

```
cargo make build
```

### Bump version

Use `cargo-bump`, e.g.:

```
cargo bump patch --git-tag
```

Then this updates the Cargo.lock

```
cargo update
```

Let's push it:

```
git add Cargo.lock
```

And:

```
git commit -m "version bumb 0.0.3"
```

Finally:

```
git push --tags
```

### Publish

You need to be logged on `npm`:

```
npm login
```

Then:

```
cargo make publish
```

The package is published under: https://www.npmjs.com/package/@platformatic/sql-inspector
