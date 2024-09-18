-- Create the initial user table for authentication and profile management

CREATE TABLE "user" (
    id SERIAL PRIMARY KEY
    ,username TEXT NOT NULL UNIQUE
    ,email TEXT NOT NULL UNIQUE
    ,password_hash TEXT NOT NULL
    ,created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
