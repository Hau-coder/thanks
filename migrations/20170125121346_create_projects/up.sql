CREATE TABLE projects (
    id SERIAL NOT NULL,
    name VARCHAR NOT NULL,
    url_path VARCHAR NOT NULL,
    github_link VARCHAR NOT NULL
);

ALTER TABLE ONLY projects
    ADD CONSTRAINT projects_pkey PRIMARY KEY (id);

CREATE UNIQUE INDEX projects_id_idx ON projects USING btree (id);
CREATE UNIQUE INDEX projects_name_idx ON projects USING btree (name);
CREATE UNIQUE INDEX projects_url_path_idx ON projects USING btree (url_path);
