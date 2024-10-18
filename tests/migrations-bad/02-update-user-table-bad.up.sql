-- Users can now share their first name

ALTER TABLE "user"
ADD COLUMN "first_name" TEXT;

ALTER TABLE "user"
-- This column already exists
ADD COLUMN "username" TEXT;
