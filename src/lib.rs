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
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ExtractResult {
    tables: Vec<String>,
    columns: Vec<String>,
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
}

impl Visitor for V {
    type Break = ();

    fn pre_visit_statement(&mut self, _stmt: &Statement) -> ControlFlow<Self::Break> {
        match _stmt {
            Statement::Query(q) => {
                if let SetExpr::Select(select) = (q.body).as_ref() {
                    for select_item in &select.projection {
                        if let SelectItem::UnnamedExpr(expr) = select_item {
                            if let Expr::Identifier(ident) = expr {
                                self.columns.insert(ident.value.clone());
                            } else if let Expr::CompoundIdentifier(ident) = expr {
                                // This is a compound identifier, like table.column
                                let first = ident.first().unwrap();
                                let second = ident.last().unwrap();
                                let full_name = format!("{first}.{second}");
                                self.columns.insert(full_name);
                            }
                        } else if let SelectItem::ExprWithAlias { expr, alias: _ } = select_item {
                            if let Expr::Identifier(ident) = expr {
                                self.columns.insert(ident.value.clone());
                            } else if let Expr::CompoundIdentifier(ident) = expr {
                                // This is a compound identifier, like table.column
                                let first = ident.first().unwrap();
                                let second = ident.last().unwrap();
                                let full_name = format!("{first}.{second}");
                                self.columns.insert(full_name);
                            }
                        } else if let SelectItem::Wildcard(_expr) = select_item {
                            self.columns.insert("*".to_string());
                        }
                    }
                }
            }
            Statement::Insert(i) => {
                for i in &i.columns {
                    self.columns.insert(i.to_string());
                }
            }
            Statement::Update {
                table,
                assignments,
                from: _,
                selection: _,
                returning: _,
            } => {
                for assignment in assignments {
                    let ident = assignment.target.to_string();
                    self.columns.insert(ident);
                }
                self.tables.insert(table.to_string());
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
    ExtractResult { columns, tables }
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

    fn test_extract(sql: &str, columns: Vec<&str>, tables: Vec<&str>) {
        let res = inspect(sql);
        assert_eq!(res.columns, columns);
        assert_eq!(res.tables, tables);
    }

    #[test]
    fn select() {
        let tests = vec![
            (
                // simple
                "SELECT id FROM users WHERE age > 30",
                vec!["age", "id"],
                vec!["users"],
            ),
            (
                // alias (must be ignored)
                "SELECT id, name as name2 FROM users WHERE age > 30",
                vec!["age", "id", "name"],
                vec!["users"],
            ),
            (
                // wildcard
                "SELECT * FROM users WHERE age > 30",
                vec!["*", "age"],
                vec!["users"],
            ),
            (
                // compound identifier
                "SELECT users.id, users.name FROM users WHERE age > 30",
                vec!["age", "users.id", "users.name"],
                vec!["users"],
            ),
            (
                // Join
                "SELECT users.id, users.name, orders.id FROM users JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "orders.id", "orders.user_id", "users.id", "users.name"],
                vec!["orders", "users"]
            ),
            (
                // join with some not compund identifiers
                "SELECT users.id, name, orders.id, title FROM users JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "name", "orders.id", "orders.user_id", "title", "users.id"],
                vec!["orders", "users"]
            ),
            (
                // join with some alias
                "SELECT id, users2.name as name2, orders.id, title FROM users as users2 JOIN orders ON users.id = orders.user_id WHERE age > 30",
                vec!["age", "id", "orders.id", "orders.user_id", "title", "users.id", "users.name"],
                vec!["orders", "users"]
            ), (
                // more aliases
            " Select S.Test_Date, E.Testno, S.Examno, S.Serialno, Type, (F.STARTED- F.ENDED) as hours
                 From Semester S, TIME F, TESTPAPERS E
                     Where S.Testno = F.Testno
                     And E.Testno = 1
                     and TYPE = 'Non-FLight'; ",
            vec!["Semester.Examno", "Semester.Serialno", "Semester.Test_Date", "Semester.Testno", "TESTPAPERS.Testno", "TIME.ENDED", "TIME.STARTED", "TIME.Testno", "TYPE", "Type"],
            vec!["Semester", "TESTPAPERS", "TIME"],
            ), (
                // multiple join
             "SELECT customerName, customercity, customermail, ordertotal,salestotal
                FROM onlinecustomers AS c
                INNER JOIN orders AS o ON c.customerid = o.customerid
                LEFT JOIN
                    sales AS s
                    ON o.orderId = s.orderId
                    WHERE s.salesId IS NULL",
            vec!["customerName", "customercity", "customermail", "onlinecustomers.customerid", "orders.customerid", "orders.orderId", "ordertotal", "sales.orderId", "sales.salesId", "salestotal"],
            vec!["onlinecustomers", "orders", "sales"]
        ),(
            "select address, name from table1 join table2 on table1.id = table2.id",
            vec!["address", "name", "table1.id", "table2.id"],
            vec!["table1", "table2"]
        )];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables);
        }
    }

    #[test]
    fn insert() {
        let tests = vec![(
            // simple
            "INSERT INTO users (id, name) VALUES (1, 'John')",
            vec!["id", "name"],
            vec!["users"],
        ),
        (
            // multiple
            "INSERT INTO Customers (CustomerName, ContactName, Address, City, PostalCode, Country)
                VALUES
                ('Cardinal', 'Tom B. Erichsen', 'Skagen 21', 'Stavanger', '4006', 'Norway'),
                ('Greasy Burger', 'Per Olsen', 'Gateveien 15', 'Sandnes', '4306', 'Norway'),
                ('Tasty Tee', 'Finn Egan', 'Streetroad 19B', 'Liverpool', 'L1 0AA', 'UK');",
            vec!["Address", "City", "ContactName", "Country", "CustomerName", "PostalCode"],
            vec!["Customers"] 
        ), (
            // without columns
            "INSERT INTO Customers VALUES (5,'Harry', 'Potter', 31, 'USA');",
            vec![],
            vec!["Customers"]
        ),
            (
            "Insert Into Test (Test_Date, Testno, Examno, Serialno, Type, Hours)
                Select S.Test_Date, E.Testno, S.Examno, S.Serialno, Type, (F.STARTED- F.ENDED) as hours
                From Semester S, TIME F, TESTPAPERS E
                    Where S.Testno = F.Testno
                    And E.Testno = 1
                    and TYPE = 'Non-FLight'; ",
            vec!["Examno", 
                "Hours", 
                "Semester.Examno", 
                "Semester.Serialno", 
                "Semester.Test_Date", 
                "Semester.Testno", 
                "Serialno", 
                "TESTPAPERS.Testno",
                "TIME.ENDED", "TIME.STARTED", "TIME.Testno", "TYPE", "Test_Date", "Testno", "Type"],
            vec!["Semester", "TESTPAPERS", "TIME", "Test"],
        )
        ];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables);
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
                    AND TYPE = 'Non-FLight';",
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
            test_extract(sql, columns, tables);
        }
    }

    #[test]
    fn update() {
        let tests = vec![
            (
                // simple
                "UPDATE users SET age = 30 WHERE age > 30",
                vec!["age"],
                vec!["users"],
            ),
            (
                // With AND condition
                "UPDATE Test
                    SET Test_Date = '2021-01-01'
                    WHERE Testno = 1
                    AND TYPE = 'Non-FLight';",
                vec!["TYPE", "Test_Date", "Testno"],
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
                "UPDATE Component SET Name = p.Number
                    FROM Part p JOIN 
                        ComponentPart cp ON p.ID = cp.PartID  JOIN 
                        Component c      ON cp.ComponentID = c.ID  
                    WHERE p.BrandID = 1003
                    AND Component.Name='Door'",
                vec![
                    "Component.ID",
                    "Component.Name",
                    "ComponentPart.ComponentID",
                    "ComponentPart.PartID",
                    "Name",
                    "Part.BrandID",
                    "Part.ID",
                    "Part.Number",
                ],
                vec!["Component", "ComponentPart", "Part"],
            ),
        ];

        for (sql, columns, tables) in tests {
            test_extract(sql, columns, tables);
        }
    }
}
