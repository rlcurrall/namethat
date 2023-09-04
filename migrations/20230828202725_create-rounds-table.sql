-- -----------------------------------------------------------------------------
-- Create rounds table
-- -----------------------------------------------------------------------------

create table rounds
(
    id             uuid      default gen_random_uuid() not null
        constraint rounds_pk
            primary key,
    game_id        uuid                                not null
        constraint rounds_games_id_fk
            references games
            on delete cascade,
    round_number   integer                             not null,
    image_url      varchar                             not null,
    round_winner   uuid,
    answers_closed boolean   default false             not null,
    created        timestamp default now()             not null,
    updated        timestamp default now()             not null,
    constraint rounds_game_id_round_player_key
        unique (game_id, round_number)
);

create trigger update_rounds_updated
    before update
    on rounds
    for each row
execute procedure mod_datetime();
