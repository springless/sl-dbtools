SELECT pg_catalog.set_config('search_path', '', false);

CREATE VIEW public._schema_version AS
 SELECT '02-update-user-table'::text AS version;

CREATE TABLE public."user" (
    id integer NOT NULL,
    username text NOT NULL,
    email text NOT NULL,
    password_hash text,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    first_name text
);

CREATE SEQUENCE public.user_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.user_id_seq OWNED BY public."user".id;

ALTER TABLE ONLY public."user" ALTER COLUMN id SET DEFAULT nextval('public.user_id_seq'::regclass);

INSERT INTO public."user" (id, username, email, password_hash, created_at, first_name) VALUES
	(1, 'user1', 'user1@test.com', 'abcdefg', '2024-10-17 01:00:55.260444', 'User1'),
	(2, 'user2', 'user2@test.com', 'abcdefg', '2024-10-17 01:00:55.260444', 'User2'),
	(3, 'user3', 'user3@test.com', 'abcdefg', '2024-10-17 01:00:55.260444', 'User3');

SELECT pg_catalog.setval('public.user_id_seq', 3, true);

ALTER TABLE ONLY public."user"
    ADD CONSTRAINT user_email_key UNIQUE (email);

ALTER TABLE ONLY public."user"
    ADD CONSTRAINT user_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public."user"
    ADD CONSTRAINT user_username_key UNIQUE (username);

