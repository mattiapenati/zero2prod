BEGIN;
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
