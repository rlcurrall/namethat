use std::{collections::HashMap, vec};

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::games::{
        Answer, Game, GameFilter, GameState, GameStatus, NewAnswer, NewGame, NewRound, Player,
        Round, UpdateGame,
    },
};

#[derive(Clone, Debug)]
pub struct GameRepo {
    client: PgPool,
}

impl GameRepo {
    pub fn new(client: PgPool) -> Self {
        Self { client }
    }

    // -------------------------------------------------------------------------
    // Basic CRUD operations
    // -------------------------------------------------------------------------

    pub async fn insert(&self, new_game: NewGame) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            INSERT INTO games (user_id, name, image_urls)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            new_game.user_id,
            new_game.name,
            &new_game.image_urls
        )
        .fetch_one(&self.client)
        .await?
        .id;

        Ok(self.get(&game_id).await?)
    }

    pub async fn get(&self, id: &Uuid) -> AppResult<Game> {
        let mut game: Game = sqlx::query_as!(
            GameRow,
            r#"
            SELECT id, user_id, name, image_urls, status as "status: GameStatus", winner
            FROM games
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.client)
        .await?
        .into();

        game.rounds = self.get_rounds_for_game(&id).await?;
        game.players = self.get_players_for_game(&id).await?;

        self.get_answers_for_game(&id)
            .await?
            .into_iter()
            .for_each(|a| {
                if let Some(round) = game.rounds.iter_mut().find(|r| r.id == a.round_id) {
                    round.answers.push(a);
                }
            });

        game.players
            .iter()
            .filter(|p| !p.is_observer)
            .for_each(|p| {
                game.scores.insert(p.username.clone(), p.score);
            });

        Ok(game)
    }

    pub async fn list(&self, filter: GameFilter) -> AppResult<Vec<Game>> {
        let results = sqlx::query_as!(
            GameRow,
            r#"
            SELECT id, user_id, name, image_urls, status as "status: GameStatus", winner
            FROM games
            WHERE
                ($1::uuid IS NULL OR user_id = $1::uuid) AND
                ($2::game_status IS NULL OR status = $2::game_status)
            "#,
            filter.user_id,
            filter.status as Option<GameStatus>
        )
        .fetch_all(&self.client)
        .await?;

        let mut games = vec![];
        results.into_iter().try_for_each(|r| match r.try_into() {
            Ok(game) => {
                games.push(game);
                Ok(())
            }
            Err(_) => Err(AppError::InternalError(
                "Could not deserialize game data".into(),
            )),
        })?;

        let game_ids = games.iter().map(|g: &Game| g.id).collect::<Vec<Uuid>>();
        let answers = self.get_answers_for_games(&game_ids).await?;
        let players = self.get_players_for_games(&game_ids).await?;
        let rounds: Vec<Round> = self
            .get_rounds_for_games(&game_ids)
            .await?
            .iter_mut()
            .map(|r| {
                r.answers = answers
                    .iter()
                    .filter(|a| a.round_id == r.id)
                    .cloned()
                    .collect();
                r.to_owned()
            })
            .collect();

        games.iter_mut().for_each(|g| {
            g.players = players
                .iter()
                .filter(|p| p.game_id == g.id)
                .cloned()
                .collect();
            g.rounds = rounds
                .iter()
                .filter(|r| r.game_id == g.id)
                .cloned()
                .collect();
        });

        Ok(games)
    }

    pub async fn update(&self, id: Uuid, update_game: UpdateGame) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE games
            SET name = $2, image_urls = $3
            WHERE id = $1
            "#,
            id,
            update_game.name,
            update_game.image_urls.as_slice()
        )
        .execute(&self.client)
        .await?;

        Ok(self.get(&id).await?)
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM games
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.client)
        .await?;
        Ok(())
    }

    pub async fn get_state(&self, id: &Uuid) -> AppResult<GameState> {
        let game = self.get(&id).await?;
        let round = game.rounds.last();
        let round_id = round.and_then(|r| Some(r.id));
        let round_number = round.and_then(|r| Some(r.round_number));
        let last_round = match game.rounds.last() {
            None => true,
            Some(round) => round.round_number == game.image_urls.len() as i32,
        };
        let image_url = round.and_then(|r| Some(r.image_url.clone()));
        let answers = round
            .and_then(|r| Some(r.answers.clone()))
            .unwrap_or(vec![]);
        let answers_closed = round.and_then(|r| Some(r.answers_closed)).unwrap_or(false);
        let round_winner = match round.and_then(|r| r.round_winner) {
            None => None,
            Some(winner) => game
                .players
                .iter()
                .find(|p| p.id == winner)
                .and_then(|p| Some(p.clone())),
        };
        let game_winner = match game.winner {
            None => None,
            Some(winner) => game
                .players
                .iter()
                .find(|p| p.id == winner)
                .and_then(|p| Some(p.clone())),
        };

        Ok(GameState {
            game_id: game.id,
            name: game.name,
            round_id,
            round_number,
            last_round,
            answers_closed,
            image_url,
            answers,
            status: game.status,
            players: game.players,
            round_winner,
            scores: game.scores,
            game_winner,
        })
    }

    // -------------------------------------------------------------------------
    // Game logic
    // -------------------------------------------------------------------------

    pub async fn get_player(&self, id: &Uuid) -> AppResult<Player> {
        let player = sqlx::query_as!(
            Player,
            r#"
            SELECT id, game_id, username, active, is_observer, score
            FROM players
            WHERE id = $1
            "#,
            id,
        )
        .fetch_one(&self.client)
        .await?;

        Ok(player)
    }

    pub async fn add_player(
        &self,
        game_id: &Uuid,
        username: &str,
        is_observer: &bool,
    ) -> AppResult<Game> {
        sqlx::query!(
            r#"
            INSERT INTO players (game_id, username, active, is_observer)
            VALUES ($1, $2, true, $3)
            ON CONFLICT DO NOTHING
            "#,
            game_id,
            username,
            is_observer
        )
        .execute(&self.client)
        .await?;

        Ok(self.get(&game_id).await?)
    }

    pub async fn remove_player(&self, id: &Uuid) -> AppResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM players
            WHERE game_id = $1
            "#,
            id,
        )
        .execute(&self.client)
        .await?;

        Ok(())
    }

    pub async fn mark_player_active(&self, id: &Uuid) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            UPDATE players
            SET active = true
            WHERE id = $1
            RETURNING game_id
            "#,
            id
        )
        .fetch_one(&self.client)
        .await?
        .game_id;

        Ok(self.get(&game_id).await?)
    }

    pub async fn mark_player_inactive(&self, id: &Uuid) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            UPDATE players
            SET active = false
            WHERE id = $1
            RETURNING game_id
            "#,
            id
        )
        .fetch_one(&self.client)
        .await?
        .game_id;

        Ok(self.get(&game_id).await?)
    }

    pub async fn start(&self, game_id: &Uuid) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE games
            SET status = 'started'::game_status
            WHERE id = $1
            "#,
            game_id
        )
        .execute(&self.client)
        .await?;

        Ok(self.get(&game_id).await?)
    }

    pub async fn add_round(&self, round: NewRound) -> AppResult<Game> {
        sqlx::query!(
            r#"
            INSERT INTO rounds (game_id, round_number, image_url)
            VALUES ($1, $2, $3)
            "#,
            round.game_id,
            round.round_number,
            round.image_url
        )
        .execute(&self.client)
        .await?;

        Ok(self.get(&round.game_id).await?)
    }

    pub async fn add_answer(&self, answer: NewAnswer) -> AppResult<Game> {
        let game = self.get_by_round_id(&answer.round_id).await?;
        if !game.players.iter().any(|p| p.id == answer.player_id) {
            return Err(AppError::ValidationError(
                "Player is not part of this game".into(),
            ));
        }

        sqlx::query!(
            r#"
            INSERT INTO answers (round_id, player_id, value)
            VALUES ($1, $2, $3)
            "#,
            answer.round_id,
            answer.player_id,
            answer.value
        )
        .execute(&self.client)
        .await?;

        Ok(self.get_by_round_id(&answer.round_id).await?)
    }

    pub async fn increment_like(&self, answer_id: &Uuid) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE answers
            SET likes = COALESCE(likes, 0) + 1
            WHERE id = $1
            "#,
            answer_id
        )
        .execute(&self.client)
        .await?;

        Ok(self.get_by_answer_id(&answer_id).await?)
    }

    pub async fn close_answers(&self, round_id: &Uuid) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE rounds
            SET answers_closed = true
            WHERE id = $1
            "#,
            round_id
        )
        .execute(&self.client)
        .await?;

        Ok(self.get_by_round_id(&round_id).await?)
    }

    pub async fn show_answer(&self, answer_id: &Uuid) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE answers
            SET shown = true
            WHERE id = $1
            "#,
            answer_id
        )
        .execute(&self.client)
        .await?;

        Ok(self.get_by_answer_id(&answer_id).await?)
    }

    pub async fn end_round(&self, round_id: &Uuid, winner: &Uuid) -> AppResult<Game> {
        sqlx::query!(
            r#"
            UPDATE rounds
            SET round_winner = $2
            WHERE id = $1
            "#,
            round_id,
            winner
        )
        .execute(&self.client)
        .await?;

        Ok(self.get_by_round_id(&round_id).await?)
    }

    pub async fn increment_score(&self, player_id: &Uuid) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            UPDATE players
            SET score = COALESCE(score, 0) + 1
            WHERE id = $1
            RETURNING game_id
            "#,
            player_id
        )
        .fetch_one(&self.client)
        .await?
        .game_id;

        Ok(self.get(&game_id).await?)
    }

    pub async fn end(&self, game_id: &Uuid) -> AppResult<Game> {
        let game = self.get(&game_id).await?;

        // Get user with highest score
        let mut max_score = 0;
        let mut game_winner = None;
        for player in game.players.iter() {
            if player.score > max_score {
                max_score = player.score;
                game_winner = Some(player.id);
            }
        }

        sqlx::query!(
            r#"
            UPDATE games
            SET status = 'finished'::game_status, winner = $2
            WHERE id = $1
            "#,
            game_id,
            game_winner
        )
        .execute(&self.client)
        .await?;

        Ok(self.get(&game_id).await?)
    }

    // -------------------------------------------------------------------------
    // Helper methods
    // -------------------------------------------------------------------------

    async fn get_rounds_for_game(&self, game_id: &Uuid) -> AppResult<Vec<Round>> {
        Ok(sqlx::query_as!(
            RoundRow,
            r#"
            SELECT id, game_id, round_number, image_url, answers_closed, round_winner
            FROM rounds
            WHERE game_id = $1
            "#,
            game_id
        )
        .fetch_all(&self.client)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect())
    }

    async fn get_rounds_for_games(&self, game_ids: &Vec<Uuid>) -> AppResult<Vec<Round>> {
        Ok(sqlx::query_as!(
            RoundRow,
            r#"
            SELECT id, game_id, round_number, image_url, answers_closed, round_winner
            FROM rounds
            WHERE game_id = ANY($1)
            "#,
            game_ids
        )
        .fetch_all(&self.client)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect())
    }

    async fn get_players_for_game(&self, game_id: &Uuid) -> AppResult<Vec<Player>> {
        let players = sqlx::query_as!(
            Player,
            r#"
            SELECT id, game_id, username, active, is_observer, score
            FROM players
            WHERE game_id = $1
            "#,
            game_id
        )
        .fetch_all(&self.client)
        .await?;

        Ok(players)
    }

    async fn get_players_for_games(&self, game_ids: &Vec<Uuid>) -> AppResult<Vec<Player>> {
        let players = sqlx::query_as!(
            Player,
            r#"
            SELECT id, game_id, username, active, is_observer, score
            FROM players
            WHERE game_id = ANY($1)
            "#,
            game_ids
        )
        .fetch_all(&self.client)
        .await?;

        Ok(players)
    }

    async fn get_answers_for_game(&self, game_id: &Uuid) -> AppResult<Vec<Answer>> {
        Ok(sqlx::query_as!(
            Answer,
            r#"
            SELECT a.id, a.round_id, a.player_id, a.value, a.likes, a.shown
            FROM answers a
            join rounds r on r.id = a.round_id
            WHERE r.game_id = $1
            ORDER BY a.created ASC
            "#,
            game_id
        )
        .fetch_all(&self.client)
        .await?)
    }

    async fn get_answers_for_games(&self, game_ids: &Vec<Uuid>) -> AppResult<Vec<Answer>> {
        Ok(sqlx::query_as!(
            Answer,
            r#"
            SELECT a.id, a.round_id, a.player_id, a.value, a.likes, a.shown
            FROM answers a
            join rounds r on r.id = a.round_id
            WHERE r.game_id = ANY($1)
            ORDER BY a.created ASC
            "#,
            game_ids
        )
        .fetch_all(&self.client)
        .await?)
    }

    pub async fn get_by_round_id(&self, round_id: &Uuid) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            SELECT g.id
            FROM games g
            join rounds r on r.game_id = g.id
            WHERE r.id = $1
            "#,
            round_id
        )
        .fetch_one(&self.client)
        .await?
        .id;
        Ok(self.get(&game_id).await?)
    }

    pub async fn get_by_answer_id(&self, answer_id: &Uuid) -> AppResult<Game> {
        let game_id = sqlx::query!(
            r#"
            SELECT g.id
            FROM games g
            join rounds r on r.game_id = g.id
            join answers a on a.round_id = r.id
            WHERE a.id = $1
            "#,
            answer_id
        )
        .fetch_one(&self.client)
        .await?
        .id;
        Ok(self.get(&game_id).await?)
    }
}

struct GameRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    image_urls: Vec<String>,
    status: GameStatus,
    winner: Option<Uuid>,
}

struct RoundRow {
    id: Uuid,
    game_id: Uuid,
    round_number: i32,
    image_url: String,
    answers_closed: bool,
    round_winner: Option<Uuid>,
}

impl Into<Game> for GameRow {
    fn into(self) -> Game {
        Game {
            id: self.id,
            user_id: self.user_id,
            name: self.name,
            image_urls: self.image_urls,
            players: vec![],
            rounds: vec![],
            scores: HashMap::new(),
            status: self.status,
            winner: self.winner,
        }
    }
}

impl Into<Round> for RoundRow {
    fn into(self) -> Round {
        Round {
            id: self.id,
            game_id: self.game_id,
            round_number: self.round_number,
            image_url: self.image_url,
            answers: vec![],
            answers_closed: self.answers_closed,
            round_winner: self.round_winner,
        }
    }
}
