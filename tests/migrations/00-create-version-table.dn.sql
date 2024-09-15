-- Create the table that will track migrations
CREATE TABLE sl_migration (
    version TEXT NOT NULL PRIMARY KEY
);

COMMENT ON TABLE sl_migration IS 'Tracks the current version of the database schema'
