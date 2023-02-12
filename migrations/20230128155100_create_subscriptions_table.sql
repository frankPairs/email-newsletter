CREATE TABLE subscriptions(
  id uuid NOT NULL PRIMARY KEY,
  email VARCHAR(255) NOT NULL UNIQUE,
  name  VARCHAR(255) NOT NULL,
  subscribed_at timestamptz NOT NULL
)