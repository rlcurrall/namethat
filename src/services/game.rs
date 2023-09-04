use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream::SplitSink, SinkExt};
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::games::{
        Game, GameAction, GameBroadcast, GameMessage, NewAnswer, NewRound, PlayerType,
    },
    repositories::games::GameRepo,
};

#[derive(Clone)]
pub struct GameActionService {
    game_id: Uuid,
    game_repo: GameRepo,
    user_type: PlayerType,
}

impl GameActionService {
    pub fn new(game_id: Uuid, game_repo: GameRepo, user_type: PlayerType) -> Self {
        Self {
            game_id,
            game_repo,
            user_type,
        }
    }

    pub async fn handle_action(&self, message: &GameAction) -> Result<()> {
        match message {
            GameAction::PlayerJoin { .. } => (),
            GameAction::StartRound { round } => self.start_round(round.to_owned()).await?,
            GameAction::UserAnswer { round_id, answer } => {
                self.add_user_answer(round_id, answer).await?
            }
            GameAction::CloseAnswers { round_id } => self.close_answers(round_id).await?,
            GameAction::RevealAnswer { answer_id } => self.reveal_answer(answer_id).await?,
            GameAction::LikeAnswer { answer_id } => self.like_answer(answer_id).await?,
            GameAction::EndRound { round_id, winner } => {
                self.end_round(round_id, &winner.parse()?).await?
            }
            GameAction::EndGame => self.end_game().await?,
        }

        Ok(())
    }

    pub async fn start_round(&self, round: i32) -> Result<()> {
        let game = self.get_game().await?;

        if self.user_type != PlayerType::GameMaster {
            return Err(AppError::AuthorizationError(
                "User cannot modify the game".into(),
            ));
        }

        if round == 1 {
            self.game_repo.start(&game.id).await?;
        }

        let image_url = game
            .image_urls
            .get(round as usize - 1)
            .ok_or(AppError::ValidationError("Invalid round number".into()))?
            .clone();

        self.game_repo
            .add_round(NewRound {
                game_id: game.id,
                round_number: round,
                image_url,
            })
            .await?;

        Ok(())
    }

    pub async fn add_user_answer(&self, round_id: &Uuid, answer: &str) -> Result<()> {
        let player_id = match self.user_type {
            PlayerType::Player { id, .. } => id,
            _ => return Err(AppError::AuthorizationError("User cannot answer".into())),
        };

        let game = self.game_repo.get_by_round_id(&round_id).await?;
        if self.game_id != game.id {
            return Err(AppError::ValidationError(
                "Invalid game id for round".into(),
            ));
        }

        let round =
            game.rounds
                .iter()
                .find(|r| r.id == *round_id)
                .ok_or(AppError::ValidationError(
                    "Invalid round id for game".into(),
                ))?;
        if round.answers_closed {
            return Err(AppError::ValidationError(
                "Answers are closed for this round".into(),
            ));
        }

        self.game_repo
            .add_answer(NewAnswer {
                player_id,
                round_id: round_id.to_owned(),
                value: answer.to_owned(),
            })
            .await?;

        Ok(())
    }

    pub async fn close_answers(&self, round_id: &Uuid) -> Result<()> {
        let game = self.game_repo.get_by_round_id(&round_id).await?;
        if self.game_id != game.id {
            return Err(AppError::ValidationError(
                "Invalid game id for round".into(),
            ));
        }

        if self.user_type != PlayerType::GameMaster {
            return Err(AppError::AuthorizationError(
                "User cannot modify the game".into(),
            ));
        }

        self.game_repo.close_answers(&round_id).await?;

        Ok(())
    }

    pub async fn reveal_answer(&self, answer_id: &Uuid) -> Result<()> {
        let game = self.game_repo.get_by_answer_id(&answer_id).await?;
        if self.game_id != game.id {
            return Err(AppError::ValidationError("Invalid answer id".into()));
        }

        if self.user_type != PlayerType::GameMaster {
            return Err(AppError::AuthorizationError(
                "User cannot modify the game".into(),
            ));
        }

        self.game_repo.show_answer(&answer_id).await?;

        Ok(())
    }

    pub async fn like_answer(&self, answer_id: &Uuid) -> Result<()> {
        let game = self.game_repo.get_by_answer_id(&answer_id).await?;
        if self.game_id != game.id {
            return Err(AppError::ValidationError("Invalid answer id".into()));
        }

        self.game_repo.increment_like(&answer_id).await?;

        Ok(())
    }

    pub async fn end_round(&self, round_id: &Uuid, winner: &Uuid) -> Result<()> {
        let game = self.game_repo.get_by_round_id(&round_id).await?;
        if self.game_id != game.id {
            return Err(AppError::ValidationError("Invalid round id".into()));
        }

        if self.user_type != PlayerType::GameMaster {
            return Err(AppError::AuthorizationError(
                "User cannot modify the game".into(),
            ));
        }

        self.game_repo.end_round(round_id, winner).await?;
        self.game_repo.increment_score(winner).await?;

        Ok(())
    }

    pub async fn end_game(&self) -> Result<()> {
        let game = self.get_game().await?;

        if self.user_type != PlayerType::GameMaster {
            return Err(AppError::AuthorizationError(
                "User cannot modify the game".into(),
            ));
        }

        self.game_repo.end(&game.id).await?;

        Ok(())
    }

    async fn get_game(&self) -> Result<Game> {
        Ok(self.game_repo.get(&self.game_id).await?)
    }
}

pub struct GameMessageService {
    game_id: Uuid,
    game_repo: GameRepo,
    sender: SplitSink<WebSocket, Message>,
}

impl GameMessageService {
    pub fn new(game_id: Uuid, game_repo: GameRepo, sender: SplitSink<WebSocket, Message>) -> Self {
        Self {
            game_id,
            game_repo,
            sender,
        }
    }

    pub async fn send_text(&mut self, text: &str) -> Result<()> {
        self.sender
            .send(Message::Text(text.to_owned()))
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn send(&mut self, message: GameMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.sender
            .send(Message::Text(json))
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn close(&mut self) -> Result<()> {
        self.sender
            .send(Message::Close(None))
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn request_display_name(&mut self) -> Result<()> {
        self.send(GameMessage::RequestDisplayName).await
    }

    pub async fn unavailable_display_name(&mut self) -> Result<()> {
        self.send(GameMessage::UnavailableDisplayName).await
    }

    pub async fn join_success(&mut self, player_type: &PlayerType) -> Result<()> {
        self.send(GameMessage::JoinSuccess {
            player_type: player_type.to_owned(),
        })
        .await
    }

    pub async fn game_state(&mut self) -> Result<()> {
        let state = self.game_repo.get_state(&self.game_id).await?;
        self.send(GameMessage::StateChange { state }).await
    }
}

#[derive(Clone)]
pub struct GameBroadcastService {
    game_id: Uuid,
    game_repo: GameRepo,
    sender: Sender<GameBroadcast>,
}

impl GameBroadcastService {
    pub fn new(game_id: Uuid, game_repo: GameRepo, sender: Sender<GameBroadcast>) -> Self {
        Self {
            game_id,
            game_repo,
            sender,
        }
    }

    pub async fn broadcast_action(&self, action: &GameAction) -> Result<()> {
        match action {
            GameAction::PlayerJoin { .. } => (),
            _ => {
                let state = self.game_repo.get_state(&self.game_id).await?;
                self.broadcast(GameMessage::StateChange { state })?
            }
        }
        Ok(())
    }

    pub async fn broadcast_game_state(&self) -> Result<()> {
        let state = self.game_repo.get_state(&self.game_id).await?;
        self.broadcast(GameMessage::StateChange { state })?;
        Ok(())
    }

    pub async fn broadcast_new_player(&self, player_type: &PlayerType) -> Result<()> {
        self.broadcast(GameMessage::NewPlayer {
            player_type: player_type.to_owned(),
        })?;
        Ok(())
    }

    pub fn broadcast(&self, message: GameMessage) -> Result<()> {
        let message = serde_json::to_string(&message)?;
        self.sender
            .send(GameBroadcast {
                game_id: self.game_id.clone(),
                message,
            })
            .map_err(|e| AppError::InternalError(format!("Could not broadcast message: {}", e)))?;
        Ok(())
    }
}
