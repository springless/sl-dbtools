// File migration utilities

use std::{fs::File, io::Write, path::{Path, PathBuf}};

use log::info;

use crate::{
    db::pg::{
        managed::PgManagedDb, migrate::PgMigrationManager, temp::{
            Initial,
            PgTempDbBuilder
        }, util::dump::{
            DumpType, dump_db
        }
    }, error::DbToolsError, managed::{ManagedDb, Seed}, migrate::{
        manager::MigrationManager,
        version::TargetVersion,
    }, namer::{DbNamingTemplate, ToDbId}, url::DbUrl
};

pub struct FileMigrator {
    base_url: DbUrl,
    admin_url: DbUrl,
    file: SqlFile,
    db: Option<PgManagedDb>,
    manager: Option<PgMigrationManager>,
}

#[derive(Clone)]
pub enum SqlFile {
    /// A schema-only file that contains no data
    Schema(PathBuf),
    /// A file that includes both schema and data
    SchemaWithData(PathBuf),
    /// A file that contains data, with an auxiliary schema file
    Data { schema: PathBuf, data: PathBuf },
}

impl FileMigrator {
    fn new(
        file: SqlFile,
        base_url: DbUrl,
        admin_url: Option<DbUrl>,
    ) -> Self {
        FileMigrator {
            admin_url: if let Some(admin_url) = admin_url { admin_url } else { base_url.guess_pg_maintenance_url() },
            base_url,
            file,
            db: None,
            manager: None,
        }
    }

    /// Create the database to house this file. Generates a random database name based
    /// on the file
    async fn create_db(mut self, pattern: &DbNamingTemplate) -> Result<Self, DbToolsError> {
        let db_builder = PgTempDbBuilder::new(
            &self.base_url,
            &Some(self.admin_url.clone()),
            Initial::Empty,
            pattern.clone(),
        )?.set_name(self.file.to_db_id());

        let db_builder = match &self.file {
            SqlFile::SchemaWithData(fname) => {
                db_builder.add_seed(Seed::File(fname.to_owned()))
            }
            SqlFile::Schema(fname) => {
                db_builder.add_seed(Seed::File(fname.to_owned()))
            }
            SqlFile::Data { schema, data } => {
                db_builder
                    .add_seed(Seed::File(schema.to_owned()))
                    .add_seed(Seed::File(data.to_owned()))
            }
        };

        let created_db = db_builder.build().await?;
        self.db = Some(created_db);
        Ok(self)
    }

    /// Create the migration manager for the file's database.
    async fn create_migrator(mut self, folder: PathBuf, view_name: &str) -> Result<Self, DbToolsError> {
        let db = self.db.as_ref().unwrap();
        let manager = PgMigrationManager::new(
            folder,
            db.url().clone(),
            view_name,
        ).await;

        if let Err(mgr_err) = manager {
            self.cleanup().await?;
            return Err(mgr_err);
        }
        self.manager = Some(manager.unwrap());
        Ok(self)
    }

    async fn migrate(mut self, target: &TargetVersion) -> Result<Self, DbToolsError> {
        let mgr = self.manager.as_mut().unwrap();
        mgr.set_target(target.clone())?;

        while mgr.get_next_step().is_some() {
            mgr.do_next_migration().await?;
        }
        Ok(self)
    }

    async fn dump<W: Write>(self, writer: &mut W) -> Result<Self, DbToolsError> {
        let db = self.db.as_ref().unwrap();

        match &self.file {
            SqlFile::Schema(_) => {
                dump_db(
                    db.url(),
                    writer,
                    &DumpType::SchemaOnly,
                    &None,
                )?;
            }
            SqlFile::SchemaWithData(_) => {
                dump_db(
                    db.url(),
                    writer,
                    &DumpType::All,
                    &None,
                )?;
            }
            SqlFile::Data { schema: _, data: _ } => {
                dump_db(
                    db.url(),
                    writer,
                    &DumpType::DataOnly,
                    &None,
                )?;
            }
        }
        Ok(self)
    }

    /// Cleans up this file migration, deleting the database being used if it
    /// was created.
    async fn cleanup(self) -> Result<(), DbToolsError> {
        if let Some(db) = self.db {
            db.drop().await?;
        }
        Ok(())
    }

    /// Migrates a file on top of itself
    async fn migrate_file(
        base_url: DbUrl,
        admin_url: Option<DbUrl>,
        file: SqlFile,
        migration_folder: &Path,
        view_name: &str,
        target: &TargetVersion,
        pattern: &DbNamingTemplate,
    ) -> Result<(), DbToolsError> {
        let file_migrator = Self::new(
            file.clone(),
            base_url,
            admin_url,
        );
        let dump_path = match &file {
            SqlFile::Data { schema: _, data } => data,
            SqlFile::Schema(schema) => schema,
            SqlFile::SchemaWithData(schema) => schema,
        };
        file_migrator
            .create_db(pattern).await?
            .create_migrator(migration_folder.to_owned(), view_name).await?
            .migrate(target).await?
            .dump(&mut File::create(dump_path)?).await?
            .cleanup().await?;
        Ok(())
    }


    /// Migrates files that are assumed to be full schema + data files. This will load
    /// them into a database, run the migrations, and then dump them back out on top
    /// of their original file location.
    pub async fn migrate_files<
        P: AsRef<Path>,
        F: AsRef<Path>,
        I: IntoIterator<Item=P>
    >(
        base_url: DbUrl,
        admin_url: Option<DbUrl>,
        migration_folder: F,
        view_name: &str,
        target: &TargetVersion,
        files: I,
        pattern: &DbNamingTemplate,
    ) -> Result<(), DbToolsError> {
        for f in files {
            let fname = f.as_ref();
            info!("Migrating Schema+Data File: {:?}", fname);
            Self::migrate_file(
                base_url.clone(),
                admin_url.clone(),
                SqlFile::SchemaWithData(fname.to_owned()),
                migration_folder.as_ref(),
                view_name,
                target,
                pattern,
            ).await?;
        }
        Ok(())
    }

    /// Migrates files that are broken up between a schema file and a list of data files.
    /// This will create a temporary database for each file, load the schema, load the data
    /// file, then create a data-only dump on top of the old data file location. Once all
    /// of the data files are migrated, it will migrate the schema file and do a schema-only
    /// dump on top of the old schema file location.
    pub async fn migrate_files_with_schema<
        P: AsRef<Path>,
        P2: AsRef<Path>,
        F: AsRef<Path>,
        I: IntoIterator<Item=P2>
    >(
        base_url: DbUrl,
        admin_url: Option<DbUrl>,
        migration_folder: &F,
        view_name: &str,
        target: &TargetVersion,
        schema: P,
        files: I,
        pattern: &DbNamingTemplate,
    ) -> Result<(), DbToolsError> {
        let schema_path = schema.as_ref();

        // First migrate all of the data files, since we need to load the schema file
        // for each
        for f in files {
            let fname = f.as_ref();
            info!("Migrating Data File: {:?}", fname);
            Self::migrate_file(
                base_url.clone(),
                admin_url.clone(),
                SqlFile::Data{ schema: schema_path.to_owned(), data: fname.to_owned() },
                migration_folder.as_ref(),
                view_name,
                target,
                pattern,
            ).await?;
        }

        // Now we can finish by migrating the schema file
        info!("Migrating Schema File: {:?}", schema_path);
        Self::migrate_file(
            base_url.clone(),
            admin_url.clone(),
            SqlFile::Schema(schema_path.to_owned()),
            migration_folder.as_ref(),
            view_name,
            target,
            pattern,
        ).await?;

        Ok(())
    }
}

impl ToDbId for SqlFile {
    fn to_db_id(&self) -> String {
        match self {
            SqlFile::Data { schema: _, data } => { data.to_db_id() },
            SqlFile::Schema(f) => { f.to_db_id() },
            SqlFile::SchemaWithData(f) => { f.to_db_id() },
        }
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

        let migrator = FileMigrator::new(
            SqlFile::SchemaWithData(test_file),
            TEST_ENV.get_postgres_url().clone(),
            Some(TEST_ENV.get_postgres_admin_url()),
        );

        let res = migrator
            .create_db(&TEST_ENV.temp_db_pattern).await.unwrap()
            .create_migrator("../../tests/migrations".into(), "_schema_version")
            .await.unwrap()
            .migrate(&TargetVersion::Current(2)).await.unwrap()
            .dump(&mut buf).await.unwrap()
            .cleanup().await;

        assert!(res.is_ok(), "Received an error during file migration: {:?}", res.err());
        let migrated_str = String::from_utf8(buf.into_inner()).unwrap();
        assert_debug_snapshot!(
            migrated_str,
            @r#""SET client_encoding = 'UTF8';\nSET standard_conforming_strings = on;\nSELECT pg_catalog.set_config('search_path', '', false);\nSET check_function_bodies = false;\nSET xmloption = content;\n\nCREATE VIEW public._schema_version AS\n SELECT '04-remove-password'::text AS version;\n\nCREATE TABLE public.\"user\" (\n    id integer NOT NULL,\n    username text NOT NULL,\n    email text NOT NULL,\n    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,\n    first_name text\n);\n\nCREATE SEQUENCE public.user_id_seq\n    AS integer\n    START WITH 1\n    INCREMENT BY 1\n    NO MINVALUE\n    NO MAXVALUE\n    CACHE 1;\n\nALTER SEQUENCE public.user_id_seq OWNED BY public.\"user\".id;\n\nALTER TABLE ONLY public.\"user\" ALTER COLUMN id SET DEFAULT nextval('public.user_id_seq'::regclass);\n\nINSERT INTO public.\"user\" (id, username, email, created_at, first_name) VALUES\n\t(1, 'user1', 'user1@test.com', '2024-10-17 01:00:55.260444', 'User1'),\n\t(2, 'user2', 'user2@test.com', '2024-10-17 01:00:55.260444', 'User2'),\n\t(3, 'user3', 'user3@test.com', '2024-10-17 01:00:55.260444', 'User3');\n\nSELECT pg_catalog.setval('public.user_id_seq', 3, true);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_email_key UNIQUE (email);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_pkey PRIMARY KEY (id);\n\nALTER TABLE ONLY public.\"user\"\n    ADD CONSTRAINT user_username_key UNIQUE (username);\n\n""#,
        );
    }
}
