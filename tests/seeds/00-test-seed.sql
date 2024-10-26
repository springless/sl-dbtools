
SELECT pg_catalog.set_config('search_path', '', false);

COMMENT ON SCHEMA "public" IS 'standard public schema';

CREATE VIEW "public"."_schema_version" AS
 SELECT '04-remove-password'::"text" AS "version";

ALTER VIEW "public"."_schema_version" OWNER TO "postgres";

CREATE TABLE "public"."user" (
    "id" integer NOT NULL,
    "username" "text" NOT NULL,
    "email" "text" NOT NULL,
    "created_at" timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "first_name" "text"
);

ALTER TABLE "public"."user" OWNER TO "postgres";

CREATE SEQUENCE "public"."user_id_seq"
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE "public"."user_id_seq" OWNER TO "postgres";

ALTER SEQUENCE "public"."user_id_seq" OWNED BY "public"."user"."id";

ALTER TABLE ONLY "public"."user" ALTER COLUMN "id" SET DEFAULT "nextval"('"public"."user_id_seq"'::"regclass");

INSERT INTO "public"."user" ("id", "username", "email", "created_at", "first_name") VALUES
	(1, 'user1', 'user1@test.com', '2024-10-17 01:00:55.260444', 'User1'),
	(2, 'user2', 'user2@test.com', '2024-10-17 01:00:55.260444', 'User2'),
	(3, 'user3', 'user3@test.com', '2024-10-17 01:00:55.260444', 'User3');

SELECT pg_catalog.setval('"public"."user_id_seq"', 3, true);

ALTER TABLE ONLY "public"."user"
    ADD CONSTRAINT "user_email_key" UNIQUE ("email");

ALTER TABLE ONLY "public"."user"
    ADD CONSTRAINT "user_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."user"
    ADD CONSTRAINT "user_username_key" UNIQUE ("username");

