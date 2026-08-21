use rusqlite::{params, Connection};

fn setup() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(
        r#"
        CREATE TABLE accounts(
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            code TEXT NOT NULL,
            name TEXT NOT NULL,
            nature TEXT NOT NULL,
            is_active INTEGER NOT NULL
        );
        CREATE TABLE journal_entries(
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            fiscal_year_id TEXT NOT NULL,
            status TEXT NOT NULL,
            entry_date TEXT NOT NULL
        );
        CREATE TABLE journal_lines(
            id TEXT PRIMARY KEY,
            journal_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            debit INTEGER NOT NULL,
            credit INTEGER NOT NULL
        );
        CREATE TABLE products(id TEXT PRIMARY KEY, name TEXT NOT NULL);
        CREATE TABLE sales_invoices(
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            fiscal_year_id TEXT NOT NULL,
            status TEXT NOT NULL
        );
        CREATE TABLE sales_invoice_lines(
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL,
            product_id TEXT NOT NULL,
            quantity REAL NOT NULL,
            line_total INTEGER NOT NULL
        );
        CREATE TABLE invoice_settlements(
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            fiscal_year_id TEXT NOT NULL,
            invoice_id TEXT NOT NULL,
            invoice_type TEXT NOT NULL,
            amount INTEGER NOT NULL
        );
        "#,
    )
    .unwrap();
    c
}

#[test]
fn top_products_orders_by_the_selected_aggregate() {
    let c = setup();
    c.execute_batch(
        r#"
        INSERT INTO products VALUES ('p1','Product A'),('p2','Product B');
        INSERT INTO sales_invoices VALUES
            ('i1','c1','fy1','posted'),
            ('i2','c1','fy1','posted'),
            ('i3','c2','fy2','posted');
        INSERT INTO sales_invoice_lines VALUES
            ('l1','i1','p1',2,100),
            ('l2','i2','p1',1,70),
            ('l3','i1','p2',1,50),
            ('l4','i3','p1',99,9999);
        "#,
    )
    .unwrap();

    let rows: Vec<(String, i64)> = c
        .prepare(
            "SELECT p.id, COALESCE(SUM(l.line_total),0) AS revenue
             FROM sales_invoice_lines l
             JOIN sales_invoices i ON i.id=l.invoice_id
             JOIN products p ON p.id=l.product_id
             WHERE i.company_id=?1 AND i.fiscal_year_id=?2 AND i.status='posted'
             GROUP BY p.id,p.name
             ORDER BY revenue DESC
             LIMIT 10",
        )
        .unwrap()
        .query_map(params!["c1", "fy1"], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(rows, vec![("p1".to_string(), 170), ("p2".to_string(), 50)]);
}

#[test]
fn trial_balance_does_not_include_other_company_or_draft_lines() {
    let c = setup();
    c.execute_batch(
        r#"
        INSERT INTO accounts VALUES ('a1','c1','1101','Cash','debit',1);
        INSERT INTO journal_entries VALUES
            ('j1','c1','fy1','posted','1405-01-01'),
            ('j2','c2','fy2','posted','1405-01-01'),
            ('j3','c1','fy1','draft','1405-01-02');
        INSERT INTO journal_lines VALUES
            ('l1','j1','a1',100,0),
            ('l2','j2','a1',900,0),
            ('l3','j3','a1',700,0);
        "#,
    )
    .unwrap();

    let balance: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN j.id IS NOT NULL THEN l.debit-l.credit ELSE 0 END),0)
             FROM accounts a
             LEFT JOIN journal_lines l ON l.account_id=a.id
             LEFT JOIN journal_entries j
               ON j.id=l.journal_id
              AND j.status='posted'
              AND j.company_id=?1
              AND j.fiscal_year_id=?2
             WHERE a.company_id=?1 AND a.is_active=1 AND a.id=?3",
            params!["c1", "fy1", "a1"],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(balance, 100);
}

#[test]
fn settlement_reports_are_scoped_to_company_and_fiscal_year() {
    let c = setup();
    c.execute_batch(
        r#"
        INSERT INTO invoice_settlements VALUES
            ('s1','c1','fy1','i1','sales',40),
            ('s2','c2','fy2','i1','sales',900),
            ('s3','c1','fy2','i1','sales',800),
            ('s4','c1','fy1','i1','purchase',700);
        "#,
    )
    .unwrap();

    let settled: i64 = c
        .query_row(
            "SELECT COALESCE(SUM(amount),0)
             FROM invoice_settlements
             WHERE company_id=?1 AND fiscal_year_id=?2
               AND invoice_id=?3 AND invoice_type='sales'",
            params!["c1", "fy1", "i1"],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(settled, 40);
}
