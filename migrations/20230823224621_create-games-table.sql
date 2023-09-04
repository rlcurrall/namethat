-- -----------------------------------------------------------------------------
-- Create games table
-- -----------------------------------------------------------------------------

create type game_status as enum ('pending', 'started', 'finished');

create table games
(
    id             uuid        default gen_random_uuid()      not null
        constraint games_pk
            primary key,
    user_id        uuid                                       not null
        constraint games_users_id_fk
            references users
            on delete cascade,
    name           varchar                                    not null,
    image_urls     varchar[]   default '{}'::varchar[]        not null,
    status         game_status default 'pending'::game_status not null,
    winner         uuid,
    created        timestamp   default now()                  not null,
    updated        timestamp   default now()                  not null
);

comment on column games.image_urls is 'Array of image URLs for the game';

create trigger update_games_updated
    before update
    on games
    for each row
execute procedure mod_datetime();
