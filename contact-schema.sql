
CREATE TABLE contacts (
    id          INT             NOT NULL AUTO_INCREMENT,
    name        VARCHAR(14)     NOT NULL,
    email       VARCHAR(16)     NOT NULL UNIQUE,
    PRIMARY KEY (id)
);

INSERT INTO contacts
        (name, email)
        VALUES
      ('John', 'g0@gmail.com'), 
      ('Jane', 'g1@gmail.com'), 
      ('Billy', 'g2@gmail.com'),
      ('Miranda', 'g3@gmail.com');