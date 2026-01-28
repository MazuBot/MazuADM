--
-- PostgreSQL database dump
--

\restrict kR0EaQ6jrcoucQTCMCfyjrkecUq8HCgyNzFIrLylr1DMk9EA6GQsjgI9qhaFIkI

-- Dumped from database version 16.11 (Ubuntu 16.11-0ubuntu0.24.04.1)
-- Dumped by pg_dump version 16.11 (Ubuntu 16.11-0ubuntu0.24.04.1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: challenges; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.challenges (
    id integer NOT NULL,
    name character varying(255) NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    default_port integer,
    priority integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    flag_regex character varying(512)
);


ALTER TABLE public.challenges OWNER TO postgres;

--
-- Name: challenges_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.challenges_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.challenges_id_seq OWNER TO postgres;

--
-- Name: challenges_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.challenges_id_seq OWNED BY public.challenges.id;


--
-- Name: exploits; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.exploits (
    id integer NOT NULL,
    name character varying(255) NOT NULL,
    challenge_id integer NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    priority integer DEFAULT 0 NOT NULL,
    max_per_container integer DEFAULT 1 NOT NULL,
    docker_image character varying(512) NOT NULL,
    entrypoint character varying(512),
    timeout_secs integer DEFAULT 30 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.exploits OWNER TO postgres;

--
-- Name: exploits_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.exploits_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.exploits_id_seq OWNER TO postgres;

--
-- Name: exploits_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.exploits_id_seq OWNED BY public.exploits.id;


--
-- Name: teams; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.teams (
    id integer NOT NULL,
    team_id character varying(100) NOT NULL,
    team_name character varying(255) NOT NULL,
    default_ip character varying(255),
    priority integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    enabled boolean DEFAULT true NOT NULL
);


ALTER TABLE public.teams OWNER TO postgres;

--
-- Name: teams_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.teams_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.teams_id_seq OWNER TO postgres;

--
-- Name: teams_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.teams_id_seq OWNED BY public.teams.id;


--
-- Name: challenges id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.challenges ALTER COLUMN id SET DEFAULT nextval('public.challenges_id_seq'::regclass);


--
-- Name: exploits id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exploits ALTER COLUMN id SET DEFAULT nextval('public.exploits_id_seq'::regclass);


--
-- Name: teams id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.teams ALTER COLUMN id SET DEFAULT nextval('public.teams_id_seq'::regclass);


--
-- Data for Name: challenges; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.challenges (id, name, enabled, default_port, priority, created_at, flag_regex) FROM stdin;
\.


--
-- Data for Name: exploits; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.exploits (id, name, challenge_id, enabled, priority, max_per_container, docker_image, entrypoint, timeout_secs, created_at) FROM stdin;
\.


--
-- Data for Name: teams; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.teams (id, team_id, team_name, default_ip, priority, created_at, enabled) FROM stdin;
1	test	Test	1.1.1.1	0	2026-01-28 02:37:54.924013+08	t
\.


--
-- Name: challenges_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.challenges_id_seq', 1, false);


--
-- Name: exploits_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.exploits_id_seq', 1, false);


--
-- Name: teams_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.teams_id_seq', 1, true);


--
-- Name: challenges challenges_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.challenges
    ADD CONSTRAINT challenges_pkey PRIMARY KEY (id);


--
-- Name: exploits exploits_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exploits
    ADD CONSTRAINT exploits_pkey PRIMARY KEY (id);


--
-- Name: teams teams_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_pkey PRIMARY KEY (id);


--
-- Name: teams teams_team_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_team_id_key UNIQUE (team_id);


--
-- Name: exploits exploits_challenge_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exploits
    ADD CONSTRAINT exploits_challenge_id_fkey FOREIGN KEY (challenge_id) REFERENCES public.challenges(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict kR0EaQ6jrcoucQTCMCfyjrkecUq8HCgyNzFIrLylr1DMk9EA6GQsjgI9qhaFIkI

