-- -----------------------------------------------------------------------------
-- Create sessions table
-- -----------------------------------------------------------------------------

create table sessions
(
    id      varchar not null
        constraint sessions_pk
            primary key,
    expires timestamp,
    session text    not null
);
