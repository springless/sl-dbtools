-- An unused table in the migrations but which exists in the database due to existing
-- in `ROOT.sql`
CREATE TABLE unused_table (
    id SERIAL PRIMARY KEY
    ,name TEXT NOT NULL
    ,description TEXT NOT NULL
);
