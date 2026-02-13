ALTER TABLE notes
ADD COLUMN IF NOT EXISTS owner_subject TEXT;

UPDATE notes
SET owner_subject = 'legacy'
WHERE owner_subject IS NULL;

ALTER TABLE notes
ALTER COLUMN owner_subject SET NOT NULL;

CREATE INDEX IF NOT EXISTS notes_owner_subject_id_idx ON notes(owner_subject, id DESC);
