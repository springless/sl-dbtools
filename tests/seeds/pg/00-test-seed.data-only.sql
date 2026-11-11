-- SCHEMA VERSION: 04-remove-password

-- SECTION: DATA

SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;

INSERT INTO public."user" (id, username, email, created_at, first_name) VALUES
	(1, 'user1', 'user1@test.com', '2024-10-17 01:00:55.260444', 'User1'),
	(2, 'user2', 'user2@test.com', '2024-10-17 01:00:55.260444', 'User2'),
	(3, 'user3', 'user3@test.com', '2024-10-17 01:00:55.260444', 'User3');

SELECT pg_catalog.setval('public.user_id_seq', 3, true);

