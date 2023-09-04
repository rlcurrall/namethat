use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, sqlx::Type)]
#[sqlx(type_name = "game_status", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum GameStatus {
    Pending,
    Started,
    Finished,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// The unique identifier for the game.
    pub id: Uuid,
    /// The Game Master
    ///
    /// This is the user that created the game and has permissions to run the
    /// game.
    pub user_id: Uuid,
    /// The name of the game
    pub name: String,
    /// The images used for the game
    ///
    /// This is a list of URLs to images.
    pub image_urls: Vec<String>,
    /// The rounds for the game
    ///
    /// This list should only be created once the game has started, otherwise it
    /// will be empty.
    pub rounds: Vec<Round>,
    /// The players in the game
    pub players: Vec<Player>,
    /// The scores for the game
    ///
    /// The key is the username of the player and the value is the score.
    pub scores: HashMap<String, i32>,
    pub status: GameStatus,
    pub winner: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: Uuid,
    pub game_id: Uuid,
    pub username: String,
    pub active: bool,
    pub is_observer: bool,
    pub score: i32,
}

impl Player {
    pub fn to_player_type(self) -> PlayerType {
        match self.is_observer {
            true => PlayerType::Observer {
                id: self.id,
                display_name: self.username,
            },
            false => PlayerType::Player {
                id: self.id,
                display_name: self.username,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Round {
    pub id: Uuid,
    pub game_id: Uuid,
    /// The round number
    pub round_number: i32,
    /// The image for the round
    ///
    /// This is duplicated from the game so that we can keep track of the image
    /// for each round.
    pub image_url: String,
    /// Whether or not the answers for the round have been closed
    ///
    /// This is used to determine if the game master can reveal the answers
    /// for the round.
    pub answers_closed: bool,
    /// Contains the answers for the round
    ///
    /// The key is the username of the player
    /// and the value is a tuple containing the answer and the number of likes.
    pub answers: Vec<Answer>,
    /// Contains the user that was selected as the winner for the round
    pub round_winner: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub id: Uuid,
    pub round_id: Uuid,
    pub player_id: Uuid,
    pub value: String,
    pub likes: i32,
    pub shown: bool,
}

/// The state of the game at a given point in time
///
/// This is used to send the state of the game to players that are joining late
/// or to refresh the state of the game for players that are reconnecting.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    /// ID of the game
    pub game_id: Uuid,
    /// Name of the game
    pub name: String,
    /// ID of the active round
    ///
    /// Potentially unset if the game has not started yet or has already finished.
    pub round_id: Option<Uuid>,
    pub last_round: bool,
    /// Whether or not the answers for the round have been closed
    ///
    /// This is used to determine if the game master can reveal the answers
    /// for the round.
    pub answers_closed: bool,
    /// The status of the game
    ///
    /// Options are:
    /// - Pending: The game has been created but has not started yet.
    /// - Started: The game is currently in progress.
    /// - Finished: The game has finished.
    pub status: GameStatus,
    /// The players in the game
    ///
    /// This is a list of usernames provided by the users, and are not tied to
    /// a user account since we want to allow unauthenticated users to play.
    pub players: Vec<Player>,
    /// The round number
    ///
    /// This is only set if the game has started and has not finished yet.
    pub round_number: Option<i32>,
    /// The image for the round
    ///
    /// This is only set if the game has started and has not finished yet.
    pub image_url: Option<String>,
    /// The answers for the round
    pub answers: Vec<Answer>,
    /// Winner of the round
    ///
    /// The username of the player that was selected as the winner for the round.
    pub round_winner: Option<Player>,
    /// The scores for the game
    ///
    /// The key is the username of the player and the value is the score.
    pub scores: HashMap<String, i32>,
    /// The winner of the game
    ///
    /// The username of the player that was selected as the winner for the game.
    /// Determined by the user with the most points at the end of the game. This
    /// is only set if the game has finished.
    pub game_winner: Option<Player>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", content = "player", rename_all = "camelCase")]
pub enum PlayerType {
    GameMaster,
    #[serde(rename_all = "camelCase")]
    Player {
        id: Uuid,
        display_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Observer {
        id: Uuid,
        display_name: String,
    },
}

/// The message types that can be sent to modify the game state
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "message", rename_all = "camelCase")]
pub enum GameAction {
    /// Player has joined the game
    ///
    /// We will use the username provided by the user rather than the username
    /// of the authenticated user since we want to support guests who are not
    /// logged in. This should be stored on the session so that we can
    /// re-populate it if the user refreshes the page.
    #[serde(rename_all = "camelCase")]
    PlayerJoin { display_name: String },
    /// Start Round
    ///
    /// Update all the players to inform them that the next round is
    /// starting, this should cause the image to be updated for all players.
    #[serde(rename_all = "camelCase")]
    StartRound { round: i32 },
    /// User answer
    ///
    /// A user has given an answer to the current round, we will
    /// update all the players to inform them of the answer.
    #[serde(rename_all = "camelCase")]
    UserAnswer { round_id: Uuid, answer: String },
    /// Close answers
    ///
    /// The game master has closed the answers for the round, we will
    /// update all the players to inform them that the answers are
    /// closed.
    #[serde(rename_all = "camelCase")]
    CloseAnswers { round_id: Uuid },
    /// Reveal answer
    ///
    /// Once all answers are in, the game master will reveal one
    /// answer at a time to the players.
    #[serde(rename_all = "camelCase")]
    RevealAnswer { answer_id: Uuid },
    /// Like answer
    ///
    /// A user has liked an answer, we will update all the players
    /// to inform them of the like.
    #[serde(rename_all = "camelCase")]
    LikeAnswer { answer_id: Uuid },
    /// End round
    ///
    /// The game master has selected a winner and the round is over.
    #[serde(rename_all = "camelCase")]
    EndRound { round_id: Uuid, winner: String },
    /// End game
    EndGame,
}

/// The message types that can be sent to the players to update clients
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "message", rename_all = "camelCase")]
pub enum GameMessage {
    RequestDisplayName,
    UnavailableDisplayName,
    #[serde(rename_all = "camelCase")]
    JoinSuccess {
        player_type: PlayerType,
    },
    #[serde(rename_all = "camelCase")]
    NewPlayer {
        player_type: PlayerType,
    },
    #[serde(rename_all = "camelCase")]
    Notification {
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    StateChange {
        state: GameState,
    },
}

#[derive(Debug, Clone)]
pub struct GameBroadcast {
    pub game_id: Uuid,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGame {
    pub user_id: Uuid,
    pub name: String,
    pub image_urls: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGame {
    pub name: String,
    pub image_urls: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFilter {
    pub user_id: Option<Uuid>,
    pub status: Option<GameStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRound {
    pub game_id: Uuid,
    /// The round number
    pub round_number: i32,
    /// The image for the round
    ///
    /// This is duplicated from the game so that we can keep track of the image
    /// for each round.
    pub image_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAnswer {
    pub player_id: Uuid,
    pub round_id: Uuid,
    pub value: String,
}
