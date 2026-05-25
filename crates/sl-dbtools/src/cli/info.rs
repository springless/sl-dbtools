use clap::{Args, Subcommand};
use sqlx::{ConnectOptions, Row};

use super::SlArgs;

/// Display information about the database schema
#[derive(Args, Debug, Clone)]
pub struct InfoArgs {
    #[command(subcommand)]
    command: InfoSubcommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum InfoSubcommand {
    /// Generate a human-readable schema reference and print it to standard output
    Human(InfoHumanArgs),
}

/// Generate a consolidated, human-readable schema reference for a given schema
#[derive(Args, Debug, Clone)]
pub struct InfoHumanArgs {
    /// Database schema to inspect
    #[arg(short, long, default_value = "public")]
    pub schema: String,
}

impl InfoArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        match &self.command {
            InfoSubcommand::Human(sub_args) => sub_args.run(args).await,
        }
    }
}

fn rule80() {
    println!("{}", "=".repeat(80));
}

fn rule80s() {
    println!("{}", "-".repeat(80));
}

fn print_indented_comment(prefix: &str, text: &str) {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .for_each(|l| println!("{}{}", prefix, l));
}

impl InfoHumanArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        let mut conn = args.get_db_conn_opts()?.connect().await?;
        let schema = &self.schema;

        // DOMAINS
        rule80();
        println!("DOMAINS");
        rule80();

        let rows = sqlx::query(r"
            SELECT format(E'%s\n  Base type : %s\n  Not null  : %s\n  Default   : %s\n  Check     : %s\n  Comment   : %s\n',
                t.typname,
                pg_catalog.format_type(t.typbasetype, t.typtypmod),
                CASE WHEN t.typnotnull THEN 'yes' ELSE 'no' END,
                COALESCE(t.typdefault, '(none)'),
                COALESCE(
                    (SELECT string_agg(pg_get_constraintdef(c.oid, true), ', ')
                     FROM pg_constraint c WHERE c.contypid = t.oid),
                    '(none)'
                ),
                COALESCE(regexp_replace(trim(obj_description(t.oid, 'pg_type')), '\s+$', '', 'gm'), '(none)')
            )
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace AND n.nspname = $1
            WHERE t.typtype = 'd'
            ORDER BY t.typname
        ")
        .bind(schema)
        .fetch_all(&mut conn)
        .await?;

        for row in &rows {
            let text: String = row.try_get(0)?;
            println!("{}", text);
        }

        // ENUMS
        rule80();
        println!("ENUMS");
        rule80();

        let rows = sqlx::query(r"
            SELECT format(E'%s\n  Values  : %s\n  Comment : %s\n',
                t.typname,
                string_agg(e.enumlabel, ', ' ORDER BY e.enumsortorder),
                COALESCE(trim(obj_description(t.oid, 'pg_type')), '(none)')
            )
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace AND n.nspname = $1
            JOIN pg_enum e ON e.enumtypid = t.oid
            GROUP BY t.oid, t.typname
            ORDER BY t.typname
        ")
        .bind(schema)
        .fetch_all(&mut conn)
        .await?;

        for row in &rows {
            let text: String = row.try_get(0)?;
            println!("{}", text);
        }

        // TABLES
        rule80();
        println!("TABLES");
        rule80();

        let table_rows = sqlx::query(
            "SELECT c.relname
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = $1
            WHERE c.relkind = 'r'
            ORDER BY c.relname",
        )
        .bind(schema)
        .fetch_all(&mut conn)
        .await?;

        for table_row in &table_rows {
            let table: String = table_row.try_get(0)?;
            let fq = format!("{}.{}", schema, table);

            println!();
            rule80s();
            println!("TABLE: {}", fq);
            rule80s();

            let comment_row = sqlx::query(
                "SELECT COALESCE(trim(obj_description($1::regclass, 'pg_class')), '')",
            )
            .bind(&fq)
            .fetch_one(&mut conn)
            .await?;
            let comment: String = comment_row.try_get(0)?;
            if !comment.is_empty() {
                print_indented_comment("  ", &comment);
            }

            println!();
            println!("  Columns:");
            let col_rows = sqlx::query(r"
                SELECT format('    %-28s  %-38s  %s%s',
                    a.attname,
                    pg_catalog.format_type(a.atttypid, a.atttypmod)
                        || CASE WHEN a.attidentity != '' THEN ' [identity]' ELSE '' END
                        || CASE WHEN a.attgenerated != '' THEN ' [generated]' ELSE '' END,
                    CASE WHEN a.attnotnull THEN 'NOT NULL' ELSE 'NULL    ' END,
                    CASE WHEN d.adbin IS NOT NULL AND a.attidentity = '' AND a.attgenerated = ''
                         THEN '  DEFAULT ' || pg_get_expr(d.adbin, d.adrelid)
                         ELSE ''
                    END
                )
                FROM pg_attribute a
                LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                WHERE a.attrelid = $1::regclass
                  AND a.attnum > 0
                  AND NOT a.attisdropped
                ORDER BY a.attnum
            ")
            .bind(&fq)
            .fetch_all(&mut conn)
            .await?;

            for row in &col_rows {
                let text: String = row.try_get(0)?;
                println!("{}", text);
            }

            let cc_rows = sqlx::query(r"
                SELECT format('    %-28s  %s',
                    a.attname,
                    replace(
                        regexp_replace(trim(col_description(a.attrelid, a.attnum)), '\s+$', '', 'gm'),
                        E'\n',
                        E'\n' || repeat(' ', 32)
                    )
                )
                FROM pg_attribute a
                WHERE a.attrelid = $1::regclass
                  AND a.attnum > 0
                  AND NOT a.attisdropped
                  AND col_description(a.attrelid, a.attnum) IS NOT NULL
                ORDER BY a.attnum
            ")
            .bind(&fq)
            .fetch_all(&mut conn)
            .await?;

            if !cc_rows.is_empty() {
                println!();
                println!("  Column Comments:");
                for row in &cc_rows {
                    let text: String = row.try_get(0)?;
                    println!("{}", text);
                }
            }

            let con_rows = sqlx::query(
                "SELECT format('    [%s] %-52s  %s',
                    CASE c.contype
                        WHEN 'p' THEN 'PK'
                        WHEN 'f' THEN 'FK'
                        WHEN 'u' THEN 'UQ'
                        WHEN 'c' THEN 'CK'
                        ELSE c.contype::text
                    END,
                    c.conname,
                    pg_get_constraintdef(c.oid, true)
                )
                FROM pg_constraint c
                WHERE c.conrelid = $1::regclass
                ORDER BY
                    CASE c.contype WHEN 'p' THEN 0 WHEN 'u' THEN 1 WHEN 'f' THEN 2 WHEN 'c' THEN 3 ELSE 4 END,
                    c.conname",
            )
            .bind(&fq)
            .fetch_all(&mut conn)
            .await?;

            if !con_rows.is_empty() {
                println!();
                println!("  Constraints:");
                for row in &con_rows {
                    let text: String = row.try_get(0)?;
                    println!("{}", text);
                }
            }

            let idx_rows = sqlx::query(
                "SELECT format('    %s', pg_get_indexdef(x.indexrelid))
                FROM pg_index x
                JOIN pg_class ic ON ic.oid = x.indexrelid
                WHERE x.indrelid = $1::regclass
                  AND NOT EXISTS (
                      SELECT 1 FROM pg_constraint c WHERE c.conindid = x.indexrelid
                  )
                ORDER BY ic.relname",
            )
            .bind(&fq)
            .fetch_all(&mut conn)
            .await?;

            if !idx_rows.is_empty() {
                println!();
                println!("  Indexes:");
                for row in &idx_rows {
                    let text: String = row.try_get(0)?;
                    println!("{}", text);
                }
            }

            let trig_rows = sqlx::query(
                "SELECT format('    %-40s  %s',
                    t.tgname,
                    pg_get_triggerdef(t.oid, true)
                )
                FROM pg_trigger t
                WHERE t.tgrelid = $1::regclass
                  AND NOT t.tgisinternal
                ORDER BY t.tgname",
            )
            .bind(&fq)
            .fetch_all(&mut conn)
            .await?;

            if !trig_rows.is_empty() {
                println!();
                println!("  Triggers:");
                for row in &trig_rows {
                    let text: String = row.try_get(0)?;
                    println!("{}", text);
                }
            }
        }

        // VIEWS
        println!();
        rule80();
        println!("VIEWS");
        rule80();

        let view_rows = sqlx::query(
            "SELECT c.relname
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = $1
            WHERE c.relkind = 'v'
            ORDER BY c.relname",
        )
        .bind(schema)
        .fetch_all(&mut conn)
        .await?;

        for view_row in &view_rows {
            let view: String = view_row.try_get(0)?;
            let fq = format!("{}.{}", schema, view);

            println!();
            rule80s();
            println!("VIEW: {}", fq);
            rule80s();

            let comment_row = sqlx::query(
                "SELECT COALESCE(trim(obj_description($1::regclass, 'pg_class')), '')",
            )
            .bind(&fq)
            .fetch_one(&mut conn)
            .await?;
            let comment: String = comment_row.try_get(0)?;
            if !comment.is_empty() {
                print_indented_comment("  ", &comment);
                println!();
            }

            let def_row = sqlx::query("SELECT pg_get_viewdef($1::regclass, true)")
                .bind(&fq)
                .fetch_one(&mut conn)
                .await?;
            let view_def: String = def_row.try_get(0)?;
            for line in view_def.lines() {
                println!("  {}", line);
            }
        }

        // FUNCTIONS
        println!();
        rule80();
        println!("FUNCTIONS");
        rule80();
        println!();

        let func_rows = sqlx::query(r"
            SELECT format('  %s(%s) -> %s%s',
                p.proname,
                pg_get_function_arguments(p.oid),
                pg_get_function_result(p.oid),
                CASE WHEN obj_description(p.oid, 'pg_proc') IS NOT NULL
                     THEN E'\n    ' || replace(trim(obj_description(p.oid, 'pg_proc')), E'\n', E'\n    ')
                     ELSE ''
                END
            )
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace AND n.nspname = $1
            WHERE p.prokind IN ('f', 'p')
            ORDER BY p.proname, pg_get_function_arguments(p.oid)
        ")
        .bind(schema)
        .fetch_all(&mut conn)
        .await?;

        for row in &func_rows {
            let text: String = row.try_get(0)?;
            println!("{}", text);
        }

        Ok(())
    }
}
