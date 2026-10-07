CREATE TABLE tools (id varchar(64) PRIMARY KEY, contract text NOT NULL, report text NOT NULL, risk integer NOT NULL CHECK (risk BETWEEN 0 AND 100));
CREATE TABLE intents (id uuid PRIMARY KEY, subject varchar(200) NOT NULL, idem varchar(128) NOT NULL, tool varchar(64) NOT NULL REFERENCES tools(id), decision varchar(16) NOT NULL, UNIQUE(subject, idem));
CREATE TABLE audit (ordinal bigserial PRIMARY KEY, intent uuid NOT NULL REFERENCES intents(id), seq integer NOT NULL, kind varchar(32) NOT NULL, detail text NOT NULL, UNIQUE(intent, seq));
CREATE FUNCTION reject_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'audit is append only'; END $$;
CREATE TRIGGER immutable_audit BEFORE UPDATE OR DELETE OR TRUNCATE ON audit FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_mutation();
CREATE TABLE outbox (id uuid PRIMARY KEY, intent uuid NOT NULL, seq integer NOT NULL, body text NOT NULL, sent boolean NOT NULL DEFAULT false, UNIQUE(intent,seq));
CREATE TABLE inbox (id uuid PRIMARY KEY, intent uuid NOT NULL, seq integer NOT NULL, body text NOT NULL, state varchar(16) NOT NULL CHECK(state IN ('applied','dead')), reason varchar(32) NOT NULL);
CREATE TABLE projections (intent uuid PRIMARY KEY, seq integer NOT NULL);
