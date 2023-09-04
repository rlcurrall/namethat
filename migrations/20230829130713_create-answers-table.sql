-- -----------------------------------------------------------------------------
-- Create answers table
-- -----------------------------------------------------------------------------

create table answers
(
    id        uuid      default gen_random_uuid() not null
        constraint answers_pk
            primary key,
    round_id  uuid                                not null
        constraint answers_rounds_id_fk
            references rounds
            on delete cascade,
    player_id uuid                                not null
        constraint answers_players_id_fk
            references players
            on delete cascade,
    value     varchar                             not null,
    likes     int       default 0                 not null,
    shown     boolean   default false             not null,
    created   timestamp default now()             not null,
    updated   timestamp default now()             not null,
    constraint answers_round_id_player_id_key
        unique (round_id, player_id)
);

create trigger update_answers_updated
    before update
    on answers
    for each row
execute procedure mod_datetime();
