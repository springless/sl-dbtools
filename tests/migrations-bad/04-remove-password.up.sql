-- We've decided to remove the password column entirely

ALTER TABLE "user"
DROP COLUMN "password_hash";
