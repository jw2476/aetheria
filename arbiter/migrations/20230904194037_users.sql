CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  username TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL
);
