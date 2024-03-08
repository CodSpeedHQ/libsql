use insta::assert_json_snapshot;
use libsql::{params, Database};
use libsql_server::hrana_proto::{Batch, BatchStep, Stmt};

use crate::common::http::Client;
use crate::common::net::TurmoilConnector;

#[test]
fn sample_request() {
    let mut sim = turmoil::Builder::new().build();
    sim.host("primary", super::make_standalone_server);
    sim.client("client", async {
        let batch = Batch {
            steps: vec![BatchStep {
                condition: None,
                stmt: Stmt {
                    sql: Some("create table test (x)".to_string()),
                    ..Default::default()
                },
            }],
            replication_index: None,
        };
        let client = Client::new();

        let resp = client
            .post(
                "http://primary:8080/v1/batch",
                serde_json::json!({ "batch": batch }),
            )
            .await
            .unwrap();
        assert_json_snapshot!(resp.json_value().await.unwrap());

        Ok(())
    });

    sim.run().unwrap();
}

#[test]
fn execute_individual_statements() {
    let mut sim = turmoil::Builder::new().build();
    sim.host("primary", super::make_standalone_server);
    sim.client("client", async {
        let db = Database::open_remote_with_connector(
            "http://primary:8080",
            "dummy_token",
            TurmoilConnector,
        )?;
        let conn = db.connect()?;

        conn.execute("create table t(x text)", ()).await?;
        conn.execute("insert into t(x) values(?)", params!["hello"])
            .await?;
        let mut rows = conn
            .query("select * from t where x = ?", params!["hello"])
            .await?;

        assert_eq!(rows.column_count(), 1);
        assert_eq!(rows.column_name(0), Some("x"));
        assert_eq!(rows.next().await?.unwrap().get::<String>(0)?, "hello");
        assert!(rows.next().await?.is_none());

        Ok(())
    });

    sim.run().unwrap();
}

#[test]
fn execute_batch() {
    let mut sim = turmoil::Builder::new().build();
    sim.host("primary", super::make_standalone_server);
    sim.client("client", async {
        let db = Database::open_remote_with_connector(
            "http://primary:8080",
            "dummy_token",
            TurmoilConnector,
        )?;
        let conn = db.connect()?;

        conn.execute_batch(
            r#"
        begin;
        create table t(x text);
        insert into t(x) values('hello; world');
        end;"#,
        )
        .await?;
        let mut rows = conn
            .query("select * from t where x = ?", params!["hello; world"])
            .await?;

        assert_eq!(rows.column_count(), 1);
        assert_eq!(rows.column_name(0), Some("x"));
        assert_eq!(
            rows.next().await?.unwrap().get::<String>(0)?,
            "hello; world"
        );
        assert!(rows.next().await?.is_none());

        Ok(())
    });

    sim.run().unwrap();
}

#[test]
fn multistatement_query() {
    let mut sim = turmoil::Builder::new().build();
    sim.host("primary", super::make_standalone_server);
    sim.client("client", async {
        let db = Database::open_remote_with_connector(
            "http://primary:8080",
            "dummy_token",
            TurmoilConnector,
        )?;
        let conn = db.connect()?;
        let mut rows = conn
            .query("select 1 + ?; select 'abc';", params![1])
            .await?;

        assert_eq!(rows.column_count(), 1);
        assert_eq!(rows.next().await?.unwrap().get::<i32>(0)?, 2);
        assert!(rows.next().await?.is_none());

        Ok(())
    });

    sim.run().unwrap();
}

#[test]
fn affected_rows_and_last_rowid() {
    let mut sim = turmoil::Builder::new().build();
    sim.host("primary", super::make_standalone_server);
    sim.client("client", async {
        let db = Database::open_remote_with_connector(
            "http://primary:8080",
            "dummy_token",
            TurmoilConnector,
        )?;
        let conn = db.connect()?;

        conn.execute(
            "create table t(id integer primary key autoincrement, x text);",
            (),
        )
        .await?;

        let r = conn.execute("insert into t(x) values('a');", ()).await?;
        assert_eq!(r, 1, "1st row inserted");
        assert_eq!(conn.last_insert_rowid(), 1, "1st row id");

        let r = conn
            .execute("insert into t(x) values('b'),('c');", ())
            .await?;
        assert_eq!(r, 2, "2nd and 3rd rows inserted");
        assert_eq!(conn.last_insert_rowid(), 3, "3rd row id");

        let r = conn.execute("update t set x = 'd';", ()).await?;
        assert_eq!(r, 3, "all three rows updated");
        assert_eq!(conn.last_insert_rowid(), 3, "last row id unchanged");

        Ok(())
    });

    sim.run().unwrap();
}
