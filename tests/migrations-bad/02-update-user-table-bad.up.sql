-- Users can now share their first name

ALTER TABLE "user"
-- This column already exists
ADD COLUMN "username" TEXT;
