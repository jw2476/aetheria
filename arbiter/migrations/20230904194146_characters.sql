CREATE TABLE IF NOT EXISTS characters
(
  id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  name TEXT NOT NULL,
  position_x REAL NOT NULL,
  position_y REAL NOT NULL,
  position_z REAL NOT NULL,
  owner INTEGER NOT NULL,
  FOREIGN KEY (owner) REFERENCES users (id)
);
