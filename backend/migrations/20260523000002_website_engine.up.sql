ALTER TABLE websites ADD COLUMN engine TEXT NOT NULL DEFAULT 'nginx';

UPDATE websites SET engine = 'nginx' WHERE engine IS NULL;
