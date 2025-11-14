-- Align the holidays.id column type with the rest of the schema.
-- Earlier migration (010) created the column as UUID, but the application code
-- stores identifiers as text values. This converts existing data and updates
-- the column type to TEXT so inserts stop failing.
ALTER TABLE holidays
    ALTER COLUMN id TYPE TEXT USING id::text;

