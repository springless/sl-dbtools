// File migration utilities

use std::{fs::File, io::Write, path::Path};

use log::info;

use crate::{
    url::DbUrl,
    managed::{ManagedDb, Seed},
    migrate::{
        manager::MigrationManager,
        version::TargetVersion,
    },
    db::pg::{
        migrate::PgMigrationManager,
        managed::PgManagedDb,
        temp::{
            Initial,
            PgTempDbBuilder
        },
        util::dump::{
            dump_db,
            DumpType,
        },
    },
    error::DbToolsError,
};

pub struct FileMigrator {
    pub base_url: DbUrl,
    pub admin_url: DbUrl,
    pub migration_dir: String,
    pub view_name: String,
}

impl FileMigrator {
    async fn create_db_for_file(
        &self,
        file: &Path,
        schema: Option<&Path>,
    ) -> Result<PgManagedDb, DbToolsError> {
        let db_builder = PgTempDbBuilder::new(
            &self.base_url,
            &Some(self.admin_url.clone()),
            Initial::Empty,
        )?;
        let db_builder = if let Some(schema_file) = schema {
            db_builder.add_seed(Seed::File(schema_file.to_path_buf()))
        } else { db_builder };
        let db_builder = db_builder.add_seed(Seed::File(file.to_path_buf()));

        let created_db = db_builder.build().await?;
        Ok(created_db)
    }

    async fn migrate_file<W: Write>(
        &self,
        target: &TargetVersion,
        file: &Path,
        writer: &mut W,
        schema: Option<&Path>,
        dump_type: &DumpType,
    ) -> Result<(), DbToolsError> {
        let temp_db = self.create_db_for_file(&file, schema).await?;

        let manager = PgMigrationManager::new(
            self.migration_dir.clone(),
            temp_db.url().clone(),
            &self.view_name,
        ).await;

        if let Err(mgr_err) = manager {
            temp_db.drop().await?;
            return Err(mgr_err);
        }
        let mut manager = manager?;

        manager.set_target(target.clone())?;

        if None == manager.get_next_step() {
            temp_db.drop().await?;
            return Ok(());
        }

        while let Some(_) = manager.get_next_step() {
            manager.do_next_migration().await?;
        }

        dump_db(
            &temp_db.url(),
            writer,
            dump_type,
            &None,
        )?;

        temp_db.drop().await?;
        Ok(())
    }

    /// Migrates files that are assumed to be full schema + data files. This will load
    /// them into a database, run the migrations, and then dump them back out on top
    /// of their original file location.
    pub async fn migrate_files<P: AsRef<Path>>(
        &self,
        target: &TargetVersion,
        files: Vec<P>,
    ) -> Result<(), DbToolsError> {
        for f in files {
            let fname = f.as_ref();
            let mut fwriter = File::create("newfile.sql")?;
            info!("Migrating Schema+Data File: {:?}", fname);
            let _ = self.migrate_file(
                target,
                fname,
                &mut fwriter,
                None,
                &DumpType::All,
            ).await?;
        }
        Ok(())
    }

    /// Migrates files that are broken up between a schema file and a list of data files.
    /// This will create a temporary database for each file, load the schema, load the data
    /// file, then create a data-only dump on top of the old data file location. Once all
    /// of the data files are migrated, it will migrate the schema file and do a schema-only
    /// dump on top of the old schema file location.
    pub async fn migrate_files_with_schema<P: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        target: &TargetVersion,
        schema: P,
        files: Vec<P2>,
    ) -> Result<(), DbToolsError> {
        let schema_path = schema.as_ref();

        // First migrate all of the data files, since we need to load the schema file
        // for each
        for f in files {
            let fname = f.as_ref();
            let mut fwriter = File::create(fname)?;
            info!("Migrating Data File: {:?}", fname);
            let _ = self.migrate_file(
                target,
                fname,
                &mut fwriter,
                Some(schema_path),
                &DumpType::DataOnly,
            ).await?;
        }

        // Now we can finish by migrating the schema file
        let _ = self.migrate_schema(
            target,
            schema_path,
        ).await?;

        Ok(())
    }

    pub async fn migrate_schema<P: AsRef<Path>>(
        &self,
        target: &TargetVersion,
        schema_file: P,
    ) -> Result<(), DbToolsError> {
        let fname = schema_file.as_ref();
        let mut fwriter = File::create(fname)?;
        info!("Migrating Schema File: {:?}", fname);
        let _ = self.migrate_file(
            target,
            fname,
            &mut fwriter,
            None,
            &DumpType::SchemaOnly,
        ).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use std::io::Cursor;

    use crate::test::TEST_ENV;

    #[tokio::test]
    async fn test_migrate_file() {
        let test_file = TEST_ENV.seed_path("pg/01-mid-migrate.sql");
        let mut buf = Cursor::new(Vec::new());

        let migrator = FileMigrator {
            base_url: TEST_ENV.get_postgres_url().clone(),
            admin_url: TEST_ENV.get_postgres_admin_url(),
            view_name: "_schema_version".to_owned(),
            migration_dir: "../../tests/migrations".to_owned(),
        };
        let res = migrator.migrate_file(
            &TargetVersion::Current(2),
            &test_file,
            &mut buf,
            None,
            &DumpType::All
        ).await;
        assert!(res.is_ok(), "Received an error during file migration: {:?}", res.err());
        let migrated_str = String::from_utf8(buf.into_inner()).unwrap();
        assert_debug_snapshot!(
            migrated_str,
            @r#""SELECT pg_catalog.set_config('search_path', '', false);\n\nCREATE VIEW public._schema_version AS\n SELECT '04-remove-password'::text AS version;\n\nCREATE TABLE public.\"user\" (\n    id integer NOT NULL,\n    username text NOT NULL,\n    email text NOT NULL,\n    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,\n    first_name text\n);\n\nCREATE SEQUENCE public.user_id_seq\n    AS integer\n    START WITH 1\n    INCREMENT BY 1\n    NO MINVALUE\n    NO MAXVALUE\n    CACHE 1;\n\nALTER SEQUENCE public.user_id_seq OWNED BY public.\"user\".id;\n\nALTER TABLE ONLY public.\"user\" ALTER COLUMN id SET DEFAULT nextval('public.user_id_seq'::regclass);\n\nINSERT INTO public.\"user\" (id, username, email, created_at, first_name) VALUES\n\t(1, 'user1', 'user1@test.com', '2024-10-17 01:00:55.260444', 'User1'),\n\t(2, 'user2', 'user2@test.com', '2024-10-17 01:00:55.260444', 'User2'),\n\t(3, 'user3', 'user3@test.com', '2024-10-17 01:00:55.260444', 'User3');\n\nSELECT pg_catalog.setval('public.user_id_seq', 3, true);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_email_key UNIQUE (email);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_pkey PRIMARY KEY (id);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_username_key UNIQUE (username);\n\n""#,
        );
    }
}
