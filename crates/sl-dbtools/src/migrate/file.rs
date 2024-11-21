// File migration utilities

use std::path::Path;

use sqlx::{postgres::PgConnectOptions, ConnectOptions};

use crate::{db::transient::{pg::{PgTransientDb, PgTransientDbBuilder}, Initial, Seed, TransientDb, TransientDbBuilder}, dump::pgdump::{dump_db, DumpType}, error::DbToolsError};

use super::{manager::{MigrationManager, PgMigrationManager}, version::TargetVersion};

pub struct FileMigrator {
    pub base_url: PgConnectOptions,
    pub admin_url: PgConnectOptions,
    pub migration_dir: String,
    pub view_name: String,
}

impl FileMigrator {
    async fn create_db_for_file(
        &self,
        file: &Path,
        schema: Option<&Path>,
    ) -> Result<PgTransientDb, DbToolsError> {
        let db_builder = PgTransientDbBuilder::new_from_conn_opts(
            self.base_url.clone(),
            Some(self.admin_url.clone()),
            Initial::Empty,
        );
        let db_builder = if let Some(schema_file) = schema {
            db_builder.add_seed(Seed::File(schema_file.to_path_buf()))
        } else { db_builder };
        let db_builder = db_builder.add_seed(Seed::File(file.to_path_buf()));

        let created_db = db_builder.build().await?;
        Ok(created_db)
    }

    async fn migrate_file(
        &self,
        target: &TargetVersion,
        file: &Path,
        schema: Option<&Path>,
        dump_type: &DumpType,
    ) -> Result<(), DbToolsError> {
        let temp_db = self.create_db_for_file(&file, schema).await?;

        let manager = PgMigrationManager::new(
            self.migration_dir.clone(),
            &temp_db.url.to_url_lossy().to_string(),
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

        let mut db_url = temp_db.url.to_url_lossy();
        db_url.set_query(None);
        dump_db(
            &db_url.to_string(),
            file,
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
            println!("Migrating File: {:?}", fname);
            let _ = self.migrate_file(
                target,
                fname,
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
            let _ = self.migrate_file(
                target,
                fname,
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
        println!("Migrating Schema: {:?}", fname);
        let _ = self.migrate_file(
            target,
            fname,
            None,
            &DumpType::SchemaOnly,
        ).await?;
        Ok(())
    }

}

