-- -----------------------------------------------------------------------------
-- Create users table
-- -----------------------------------------------------------------------------

create table users
(
    id        uuid      default gen_random_uuid() not null
        constraint users_pk
            primary key,
    email     varchar(100)                        not null,
    password  varchar(255)                        not null,
    full_name varchar(255),
    created   timestamp default now()             not null,
    updated   timestamp default now()             not null
);

create trigger update_users_updated
    before update
    on users
    for each row
execute procedure mod_datetime();
