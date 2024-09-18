-- User passwords were getting leaked and so we are forcing all users to reset their
-- credentials

UPDATE "user"
SET "password_hash" = '';
