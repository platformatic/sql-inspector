use core::ops::ControlFlow;
use serde::{Deserialize, Serialize};
use sqlparser::ast::Visitor;
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::fmt;
use wasm_bindgen::prelude::*;

// This extracts the columns and tables from a SQL query
// It returns a ExtractResult struct with the columns and tables
// Note that in some cases is not possible to "resolve" the columns' table.
// For example:
// ```
// "select address, name from table1 join table2 on table1.id = table2.id",
// ```
//
// This query is ambiguous, because we don't know if the `address` and `name` columns are
// from table1 or table2. We can't resolve this without the actual DB schema.

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum QueryType {
    #[default]
    SELECT,
    INSERT,
    UPDATE,
    DELETE,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractResult {
    tables: Vec<String>,
    columns: Vec<String>,
    target_table: String, // This is the target table in the INSERT, UPDATE or DELETE statements case
    query_type: QueryType,
}

impl fmt::Display for ExtractResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {:?}", self.tables, self.columns)
    }
}

#[derive(Default)]
struct V {
    columns: HashSet<String>,
    tables: HashSet<String>,
    aliases: HashMap<String, String>,
    target_table: String, // This is the target table in the INSERT, UPDATE or DELETE statements case
    query_type: QueryType,
}

fn join(arr: &[Ident]) -> String {
    let mut result = String::new();
    for (index, s) in arr.iter().enumerate() {
        if index > 0 {
            result.push('.');
        }
        let q = s.to_string();
        result.push_str(q.as_str());
    }
    result
}

#[allow(clippy::assigning_clones)]
impl Visitor for V {
    type Break = ();

    fn pre_visit_statement(&mut self, _stmt: &Statement) -> ControlFlow<Self::Break> {
        match _stmt {
            Statement::Query(q) => {
                self.query_type = QueryType::SELECT;
                if let SetExpr::Select(select) = (q.body).as_ref() {
                    for select_item in &select.projection {
                        if let SelectItem::UnnamedExpr(expr) = select_item {
                            if let Expr::Identifier(ident) = expr {
                                self.columns.insert(ident.value.clone());
                            } else if let Expr::CompoundIdentifier(ident) = expr {
                                // This is a compound identifier, like table.column
                                let full_name = join(ident);
                                self.columns.insert(full_name);
                            }
                        } else if let SelectItem::ExprWithAlias { expr, alias: _ } = select_item {
                            if let Expr::Identifier(ident) = expr {
                                self.columns.insert(ident.value.clone());
                            } else if let Expr::CompoundIdentifier(ident) = expr {
                                // This is a compound identifier, like table.column
                                let full_name = join(ident);
                                self.columns.insert(full_name);
                            }
                        } else if let SelectItem::Wildcard(_expr) = select_item {
                            self.columns.insert("*".to_string());
                        }
                    }
                }
            }
            Statement::Insert(i) => {
                self.query_type = QueryType::INSERT;
                // The "insert" statement has a table as a target
                let table_name = i.table_name.to_string();
                self.tables.insert(table_name.clone());
                self.target_table = table_name.clone();
                for i in &i.columns {
                    let full_name = format!("{table_name}.{i}");
                    self.columns.insert(full_name);
                }
            }
            Statement::Update {
                table,
                assignments,
                from: _,
                selection: _,
                returning: _,
            } => {
                self.query_type = QueryType::UPDATE;
                // The "insert" statement has a table as a target
                let table_name = table.to_string();
                self.target_table = table_name.clone();
                for assignment in assignments {
                    let value = assignment.value.clone();
                    let target = assignment.target.clone();
                    match value {
                        Expr::CompoundIdentifier(ident) => {
                            // This is a compound identifier, like table.column
                            let first = ident.first().unwrap();
                            let second = ident.last().unwrap();
                            let full_name = format!("{first}.{second}");
                            self.columns.insert(full_name);
                        }
                        Expr::Identifier(ident) => {
                            let full_name = format!("{table_name}.{ident}");
                            self.columns.insert(full_name);
                        }
                        _ => {}
                    }
                    if let AssignmentTarget::ColumnName(ident) = target {
                        // It's a tuple with one vector of idents
                        if (ident.0).len() == 1 {
                            let column = ident.0.first().unwrap();
                            let full_name = format!("{table_name}.{column}");
                            self.columns.insert(full_name);
                        } else {
                            let full_name = join(&ident.0);
                            self.columns.insert(full_name);
                        }
                    }
                }
                self.tables.insert(table.to_string());
            }
            Statement::Delete(delete) => {
                self.query_type = QueryType::DELETE;
                if let FromTable::WithFromKeyword(tables) = &delete.from {
                    self.target_table = tables[0].to_string();
                    // In mysql, the FROM clause can have multiple tables
                    for i in tables {
                        self.tables.insert(i.to_string());
                    }
                }
            }

            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, _table_factor: &TableFactor) -> ControlFlow<Self::Break> {
        // Here we extract aliases for table names
        if let TableFactor::Table { name, alias, .. } = _table_factor {
            let table_name = name.to_string();
            self.tables.insert(table_name.clone());
            if let Some(alias) = alias {
                let alias = alias.to_string();
                self.aliases.insert(alias, table_name);
            }
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        // Relation === table name
        let table_name = relation.0[0].value.clone();
        self.tables.insert(table_name.clone());
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if let Expr::Wildcard = expr {
            self.columns.insert("*".to_string());
        }
        if let Expr::Identifier(ident) = expr {
            self.columns.insert(ident.value.clone());
        }

        if let Expr::CompoundIdentifier(idents) = expr {
            let mut full_column = String::new();
            for ident in idents {
                full_column.push_str(&ident.value);
                full_column.push('.');
            }
            full_column.pop();
            self.columns.insert(full_column);
        }

        ControlFlow::Continue(())
    }
}

fn inspect(sql: &str) -> ExtractResult {
    let statements = Parser::parse_sql(&GenericDialect {}, sql).unwrap();
    let mut visitor = V::default();
    statements.visit(&mut visitor);
    let mut columns: Vec<String> = Vec::from_iter(visitor.columns.iter().map(|c| c.to_string()));
    // We replace the aliases with the real table name for
    // the fully-qualified columns
    for c in columns.iter_mut() {
        if !c.contains('.') {
            continue;
        }
        let prefix = c.split('.').next().unwrap();
        let col = c.split('.').last().unwrap();
        if let Some(alias) = visitor.aliases.get(prefix) {
            *c = format!("{}.{}", alias, col);
        }
    }

    let mut tables: Vec<String> = Vec::from_iter(visitor.tables.iter().map(|c| c.to_string()));
    columns.sort();
    tables.sort();
    let target_table = visitor.target_table.clone();
    let query_type = visitor.query_type;
    ExtractResult {
        columns,
        tables,
        target_table,
        query_type,
    }
}

// This is the entry point for the WASM module, return a JSON with the result
#[wasm_bindgen]
pub fn sqlinspector(sql: &str) -> JsValue {
    let res = inspect(sql);
    serde_wasm_bindgen::to_value(&res).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_extract(sql: &str, columns: Vec<&str>, tables: Vec<&str>, query_type: QueryType) {
        let res = inspect(sql);
        assert_eq!(res.columns, columns);
        assert_eq!(res.tables, tables);
        assert_eq!(res.query_type, query_type);
    }

    #[test]
    fn select() {
        let tests = vec![
            (
                // simple
                "SELECT id FROM users WHERE age > 30",
                vec!["age", "id"],
                vec!["users"],
            ),(
                // alias (must be ignored)
                "SELECT id, name as name2 FROM users WHERE age > 30",
                vec!["age", "id", "name"],
                vec!["users"],
            ),(
                // wildcard
                "SELECT * FROM users WHERE age > 30",
                vec!["*", "age"],
                vec!["users"],
            ),(
                // compound identifier
                "SELECT users.id, users.name FROM users WHERE age > 30",
                vec!["age", "users.id", "users.name"],
                vec!["users"],
            ),(
                // join with no fully qualified columns
                "select address, name from table1 join table2 on table1.id = table2.id",
                vec!["address", "name", "table1.id", "table2.id"],
                vec!["table1", "table2"]
            ),(
                // Join
                "SELECT users.id, users.name, orders.id FROM users JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "orders.id", "orders.user_id", "users.id", "users.name"],
                vec!["orders", "users"]
            ),(
                // join with some not compund identifiers
                "SELECT users.id, name, orders.id, title FROM users JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "name", "orders.id", "orders.user_id", "title", "users.id"],
                vec!["orders", "users"]
            ), (
                // join with some alias
                "SELECT id, users2.name as name2, orders.id, title FROM users as users2 JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "id", "orders.id", "orders.user_id", "title", "users.id", "users.name"],
                vec!["orders", "users"]
            ), (
                // more aliases
                "Select t1.test_date, t3.testno, t1.examno, t1.serialno, type, (t2.started - t2.ended) as hours
                 From Table1 t1, Table2 t2, Table3 t3
                     Where t1.testno = t2.testno
                     And t3.testno = 1
                     and type = 'xxxxx'; ",
                vec![
                    "Table1.examno", 
                    "Table1.serialno", 
                    "Table1.test_date", 
                    "Table1.testno",
                    "Table2.ended", 
                    "Table2.started", 
                    "Table2.testno", 
                    "Table3.testno", 
                    "type",
                ],
                vec!["Table1", "Table2", "Table3"]
            ), (
                // multiple joins
                "SELECT customerName, customercity, customermail, ordertotal, salestotal
                FROM table1 AS t1
                INNER JOIN table2 AS t2 ON t1.customerid = t2.customerid
                LEFT JOIN
                    table3 AS t3
                    ON t2.orderId = t3.orderId
                    WHERE t3.salesId IS NULL",
                vec![
                    "customerName", 
                    "customercity", 
                    "customermail", 
                    "ordertotal", 
                    "salestotal",
                    "table1.customerid", 
                    "table2.customerid",
                    "table2.orderId",
                    "table3.orderId", 
                    "table3.salesId"
                ],
                vec!["table1", "table2", "table3"]
            ),(
                // complex query with join, counts and group by 
                "SELECT
                    t1.id, 
                    t1.label_real_address, 
                    t1.ext, 
                    COUNT(t2.contact_id), 
                    COUNT(t4.release_id) 
                FROM
                    table1 t1
                    LEFT JOIN table2 t2  ON t2.contact_type='lx' AND t2.contact_id=t1.id 
                    LEFT JOIN table3 t3 ON t3.id=t1.id 
                    LEFT JOIN table4 t4 ON t3.release_id=t4.release_id 
                GROUP BY t1.label_real_address 
                ORDER BY COUNT(t2.contact_id) DESC", 
                vec![
                    "table1.ext", 
                    "table1.id", 
                    "table1.label_real_address",
                    "table2.contact_id", 
                    "table2.contact_type",
                    "table3.id", 
                    "table3.release_id",
                    "table4.release_id", 
                ],
                vec!["table1", "table2", "table3", "table4"]
            ),(
                "SELECT id, name from (SELECT * FROM users UNION SELECT * FROM customers)",
                vec!["id", "name"],
                vec!["customers", "users"]

        )];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables, QueryType::SELECT);
        }
    }

    #[test]
    fn insert() {
        let tests = vec![
            (
                // simple
                "INSERT INTO users (id, name) VALUES (1, 'Marco')",
                vec!["users.id", "users.name"],
                vec!["users"],
            ), (
                // multiple
                "INSERT INTO Customers (CustomerName, ContactName, Address, City, PostalCode, Country)
                    VALUES
                    ('Platformatic', 'Luca', 'xxx 21', 'Vancouver', '4006', 'Canada'),
                    ('Platformatic eu', 'Marco', 'yyy 23', 'Bologna', '40137', 'Italy');",
                vec!["Customers.Address", "Customers.City", "Customers.ContactName", "Customers.Country", "Customers.CustomerName", "Customers.PostalCode"],
                vec!["Customers"]
            ), (
                // without columns
                "INSERT INTO Customers VALUES (5,'Harry', 'Potter', 31, 'Hogwarts');",
                vec![],
                vec!["Customers"]
            ),
                (
                "INSERT INTO Table1 (test_date, testno, examno, serialno, type, hours)
                    SELECT T2.test_date, T4.testno, T2.examno, T2.serialno, type, (T3.started- T3.ended) as hours
                    FROM Table2 T2, Table3 T3, Table4 T4
                        Where T2.testno = T3.testno
                        And T4.testno = 1
                        and type = 'xxxxx'; ",
                vec![
                    "Table1.examno",
                    "Table1.hours",
                    "Table1.serialno",
                    "Table1.test_date",
                    "Table1.testno",
                    "Table1.type",
                    "Table2.examno",
                    "Table2.serialno",
                    "Table2.test_date",
                    "Table2.testno",
                    "Table3.ended",
                    "Table3.started",
                    "Table3.testno",
                    "Table4.testno", 
                    "type"
                ],
                vec!["Table1", "Table2", "Table3", "Table4"],
            )
        ];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables, QueryType::INSERT);
        }
    }

    #[test]
    fn delete() {
        let tests = vec![
            (
                // simple
                "DELETE FROM users WHERE age > 30",
                vec!["age"],
                vec!["users"],
            ),
            (
                // With AND condition
                "DELETE FROM Test
                    WHERE Testno = 1
                    AND TYPE = 'xxxxxx';",
                vec!["TYPE", "Testno"],
                vec!["Test"],
            ),
            (
                // With EXISTS
                "DELETE FROM t1
                    WHERE t1.V1 > t1.V2
                    AND EXISTS (SELECT * FROM t2 WHERE t2.V1 = t1.V1);",
                vec!["t1.V1", "t1.V2", "t2.V1"],
                vec!["t1", "t2"],
            ),
        ];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables, QueryType::DELETE);
        }
    }

    #[test]
    fn update() {
        let tests = vec![
            (
                // simple
                "UPDATE users SET age = 30",
                vec!["users.age"],
                vec!["users"],
            ),
            (
                // With AND condition
                "UPDATE Test
                    SET Test_Date = '2021-01-01'
                    WHERE Testno = 1
                    AND TYPE = 'xxxxxxx';",
                vec!["TYPE", "Test.Test_Date", "Testno"],
                vec!["Test"],
            ),
            (
                // With EXISTS
                "UPDATE t1
                    SET t1.V1 = t2.V1
                    WHERE t1.V1 > t1.V2
                    AND EXISTS (SELECT * FROM t2 WHERE t2.V1 = t1.V1);",
                vec!["t1.V1", "t1.V2", "t2.V1"],
                vec!["t1", "t2"],
            ),
            (
                // Complex update
                "UPDATE component SET name = p.number
                       FROM part p 
                       JOIN
                           component_part cp ON p.id = cp.partId  JOIN
                           component c ON cp.componentId = c.id
                       WHERE p.brandId = 1003
                       AND component.name='xxx'",
                vec![
                    "component.id",
                    "component.name",
                    "component_part.componentId",
                    "component_part.partId",
                    "part.brandId",
                    "part.id",
                    "part.number",
                ],
                vec!["component", "component_part", "part"],
            ),
        ];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables, QueryType::UPDATE);
        }
    }
}
