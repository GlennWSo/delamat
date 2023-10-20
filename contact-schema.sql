
CREATE TABLE contacts (
    id          INT             NOT NULL,
    name        VARCHAR(14)     NOT NULL,
    email       VARCHAR(16)     NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO contacts
        (id, name, email)
        VALUES
      (0, 'John', 'g0@gmail.com'), 
      (1, 'Jane', 'g1@gmail.com'), 
      (2, 'Billy', 'g2@gmail.com'),
      (3, 'Miranda', 'g3@gmail.com');