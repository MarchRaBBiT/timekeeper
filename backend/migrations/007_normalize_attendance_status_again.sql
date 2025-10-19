-- Re-normalize attendance status to snake_case for any records inserted by admin upsert before code fix
UPDATE attendance SET status = lower(status);

