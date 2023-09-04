-- -----------------------------------------------------------------------------
-- Create players table
-- -----------------------------------------------------------------------------

create table players
(
    id          uuid      default gen_random_uuid() not null
        constraint players_pk
            primary key,
    game_id     uuid                                not null
        constraint players_game_id_fk
            references games
            on delete cascade,
    username    varchar                             not null,
    active      boolean   default false             not null,
    is_observer boolean   default true              not null,
    score       int       default 0                 not null,
    created     timestamp default now()             not null,
    updated     timestamp default now()             not null,
    constraint players_round_id_username_key
        unique (game_id, username)
);

create trigger update_players_updated
    before update
    on players
    for each row
execute procedure mod_datetime();

alter table rounds
    add constraint rounds_players_id_fk
        foreign key (round_winner) references players on delete set null;

alter table games
    add constraint games_players_id_fk
        foreign key (winner) references players on delete set null;
