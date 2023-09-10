use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use axum_sessions::extractors::WritableSession;
use futures_util::{stream::SplitStream, StreamExt};
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    extractors::auth::AuthUser,
    models::{
        games::{GameAction, GameBroadcast, GameStatus, PlayerType},
        users::User,
    },
    services::game::{GameActionService, GameBroadcastService, GameMessageService},
    AppState,
};

pub async fn game_websocket(
    ws: WebSocketUpgrade,
    AuthUser(user): AuthUser,
    session: WritableSession,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
) -> crate::error::AppResult<impl IntoResponse> {
    let session_id = session.id().to_string();

    Ok(ws.on_upgrade(move |socket| async move {
        match game_handler(socket, user, state, game_id, session_id).await {
            Ok(_) => (),
            Err(e) => tracing::error!("Error in websocket handler: {:?}", e),
        }
    }))
}

async fn game_handler(
    socket: WebSocket,
    user: Option<User>,
    state: Arc<AppState>,
    game_id: Uuid,
    session_id: String,
) -> crate::error::AppResult<()> {
    let (sender, mut receiver) = socket.split();
    let rx = state.tx.subscribe();
    let tx = state.tx.clone();

    let broadcast_service = GameBroadcastService::new(game_id.clone(), state.game_repo.clone(), tx);
    let mut message_service =
        GameMessageService::new(game_id.clone(), state.game_repo.clone(), sender);

    let player_type = get_player_type(
        &game_id,
        &session_id,
        &state,
        &user,
        &mut receiver,
        &mut message_service,
    )
    .await?;
    let player_id = match player_type {
        PlayerType::Player { id, .. } => Some(id),
        PlayerType::Observer { id, .. } => Some(id),
        PlayerType::GameMaster => None,
    };

    message_service.join_success(&player_type).await?;

    let game_service = GameActionService::new(
        game_id.clone(),
        state.game_repo.clone(),
        player_type.clone(),
    );

    let mut send_task = get_send_task(rx, game_id, message_service, user);
    let mut recv_task = get_recv_task(
        receiver,
        player_type.clone(),
        game_service.clone(),
        broadcast_service.clone(),
    );

    // Broadcast the new player message
    broadcast_service.broadcast_new_player(&player_type).await?;
    broadcast_service.broadcast_game_state().await?;

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    tracing::info!("Websocket closed");
    if let Some(player_id) = player_id {
        tracing::info!("Marking player inactive");
        state
            .game_repo
            .mark_player_inactive(&player_id)
            .await
            .map_err(|e| {
                tracing::error!("marking inactive user");
                e
            })?;
        broadcast_service
            .broadcast_game_state()
            .await
            .map_err(|e| {
                tracing::error!("sending broadcast of state");
                e
            })?;
    }

    Ok(())
}

fn get_send_task(
    mut rx: Receiver<GameBroadcast>,
    game_id: Uuid,
    mut message_service: GameMessageService,
    user: Option<User>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Ok(broadcast) = rx.recv().await {
            match handle_broadcast(broadcast.clone(), &game_id, &mut message_service).await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!(
                        "Error in broadcast handler: {:?}\nUser: {:#?}\nBroadcast: {:#?}",
                        e,
                        user,
                        broadcast
                    );
                }
            }
        }
    })
}

fn get_recv_task(
    mut receiver: SplitStream<WebSocket>,
    player_type: PlayerType,
    recv_game_service: GameActionService,
    broadcast_service: GameBroadcastService,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match player_type {
            PlayerType::Observer { .. } => loop {
                let message = receiver.next().await;
                match message {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    _ => (),
                }
            }, // messages from observers are ignored
            PlayerType::GameMaster | PlayerType::Player { .. } => {
                while let Some(Ok(Message::Text(message))) = receiver.next().await {
                    match handle_incoming_message(message, &recv_game_service, &broadcast_service)
                        .await
                    {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!("Error in incoming message handler: {:?}", e);
                        }
                    }
                }
            }
        }
    })
}

async fn get_player_type(
    game_id: &Uuid,
    session_id: &String,
    state: &Arc<AppState>,
    user: &Option<User>,
    receiver: &mut SplitStream<WebSocket>,
    message_service: &mut GameMessageService,
) -> AppResult<PlayerType> {
    let game = state.game_repo.get(game_id).await?;

    // if the user is the game master, use their username
    if let Some(user) = user {
        if user.id == game.user_id {
            return Ok(PlayerType::GameMaster);
        }
    }

    // Check if the user has already joined this game
    if let Some(player_id) = state
        .session_manager
        .get_game_display_name(&session_id, game_id)
        .await?
    {
        if let Ok(player) = state.game_repo.get_player(&player_id).await {
            state.game_repo.mark_player_active(&player.id).await?;
            return match game.id == player.game_id {
                true => Ok(player.to_player_type()),
                false => Err(AppError::InternalError("Player is not in the game".into())),
            };
        }
    }

    message_service.request_display_name().await?;

    loop {
        match receiver.next().await {
            Some(Err(_)) | Some(Ok(Message::Close(_))) => {
                return Err(AppError::InternalError("Websocket closed".into()))
            }
            Some(Ok(Message::Text(msg))) => {
                if let Ok(GameAction::PlayerJoin { display_name: name }) =
                    serde_json::from_str::<GameAction>(&msg)
                {
                    // Get latest version of the game in the case someone else has joined
                    // with the same name
                    let game = state.game_repo.get(game_id).await?;
                    let observer = game.status != GameStatus::Pending;

                    // If the name is not taken, add the player to the game
                    if name != "Game Master".to_string()
                        && !game.players.iter().any(|p| p.username == name)
                    {
                        let game = state
                            .game_repo
                            .add_player(&game.id, &name, &observer)
                            .await?;
                        let player = game.players.iter().find(|p| p.username == name).ok_or(
                            AppError::InternalError("Player not found after adding to game".into()),
                        )?;
                        state
                            .session_manager
                            .set_game_display_name(session_id, &game.id, &player.id)
                            .await?;

                        return Ok(player.clone().to_player_type());
                    }

                    // Otherwise, let the user know that the name is taken and wait for
                    // them to send another name
                    message_service.unavailable_display_name().await?;
                }
            }
            _ => (/* continue looping */),
        }
    }
}

/// Determines if a broadcast message should be sent to the user.
async fn handle_broadcast(
    broadcast: GameBroadcast,
    game_id: &Uuid,
    message_service: &mut GameMessageService,
) -> AppResult<()> {
    if broadcast.game_id == game_id.to_owned() {
        message_service.send_text(&broadcast.message).await?;
    }
    Ok(())
}

async fn handle_incoming_message(
    message: String,
    game_service: &GameActionService,
    broadcast_service: &GameBroadcastService,
) -> AppResult<()> {
    let action = serde_json::from_str::<GameAction>(&message)?;
    game_service.handle_action(&action).await?;
    broadcast_service.broadcast_action(&action).await?;

    Ok(())
}
