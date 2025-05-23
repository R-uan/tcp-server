use crate::{
    models::{client_requests::ConnectionRequest, deck::Deck, http_response::PartialPlayerProfile},
    utils::{errors::PlayerConnectionError, logger::Logger},
    SETTINGS,
};
use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};
use crate::models::client_requests::ReconnectionRequest;

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub level: u32,
    pub username: String,
    pub current_deck: Deck,
    pub player_token: String,
    pub current_deck_id: String,
}

impl Player {
    /// Attempts to construct a `Player` from a UTF-8-encoded payload.
    ///
    /// Expects the payload to be a newline-delimited string with the following format:
    /// ```
    /// <id>
    /// <username>
    /// <current_deck_id>
    /// <level>
    /// ```
    ///
    /// Returns:
    /// - `Ok(Player)` if parsing succeeds
    /// - `Err(InvalidPlayerPayload)` if UTF-8 is invalid or format is incorrect
    pub async fn new_connection(payload: &[u8]) -> Result<Self, PlayerConnectionError> {
        return match serde_cbor::from_slice::<ConnectionRequest>(payload) {
            Err(error) => {
                let reason = error.to_string();
                Logger::error(&format!("[PLAYER] {}", &reason));
                Err(PlayerConnectionError::InvalidPlayerPayload(format!(
                    "{reason} (ConnRequest CBOR Deserialisation)"
                )))
            }
            Ok(request) => {
                let player_profile = Player::get_player_profile(&request.auth_token).await?;
                Logger::info(&format!(
                    "[PLAYER] Fetched `{}`'s profile",
                    &player_profile.username
                ));

                let player_deck =
                    Player::get_player_deck(&request.current_deck_id, &request.auth_token).await?;
                Logger::info(&format!(
                    "[PLAYER] Fetched `{}`'s deck with {} cards",
                    &player_profile.username,
                    player_deck.cards.len()
                ));

                Ok(Player {
                    id: request.player_id,
                    current_deck: player_deck,
                    player_token: request.auth_token,
                    level: player_profile.level,
                    username: player_profile.username,
                    current_deck_id: request.current_deck_id,
                })
            }
        };
    }

    pub async fn reconnection(payload: &[u8]) -> Result<String, PlayerConnectionError> {
        return match serde_cbor::from_slice::<ReconnectionRequest>(payload) {
            Ok(request) => {
                let player_profile = Player::get_player_profile(&request.auth_token).await?;
                if player_profile.id != request.player_id {
                    return Err(PlayerConnectionError::PlayerDoesNotMatch);
                }
                return Ok(player_profile.id);
            }
            Err(error) => {
                let reason = error.to_string();
                Logger::error(&format!("[PLAYER] {}", &reason));
                Err(PlayerConnectionError::InvalidPlayerPayload(format!(
                    "{reason} (ConnRequest CBOR Deserialisation)"
                )))
            }        }
    }
    
    async fn get_player_deck(deck_id: &str, token: &str) -> Result<Deck, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/deck/{}", settings.deck_server, deck_id);
        let reqwest_client = reqwest::Client::new();
        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let result = response
                        .json::<Deck>()
                        .await
                        .map_err(|_| PlayerConnectionError::InvalidDeckFormat);
                    result
                }
                StatusCode::NOT_FOUND => Err(PlayerConnectionError::DeckNotFound),
                _ => {
                    let error_msg = response.text().await.unwrap();
                    Logger::error(&format!("[PLAYER] {}", &error_msg));
                    Err(PlayerConnectionError::UnexpectedDeckError)
                }
            },
            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedDeckError),
                    _ => Err(PlayerConnectionError::UnexpectedDeckError),
                };
            }
        };
    }

    async fn get_player_profile(
        token: &str,
    ) -> Result<PartialPlayerProfile, PlayerConnectionError> {
        let settings = SETTINGS.get().expect("Settings not initialized");
        let api_url = format!("{}/api/player/profile", settings.auth_server);
        let reqwest_client = reqwest::Client::new();
        return match reqwest_client
            .get(api_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                // Why is reqwest unauthorized not an error, kinda cringe...
                if response.status() == StatusCode::UNAUTHORIZED {
                    return Err(PlayerConnectionError::UnauthorizedPlayerError);
                }

                let result = response.json::<PartialPlayerProfile>().await.map_err(|_| {
                    PlayerConnectionError::InvalidPlayerPayload(
                        "Failed to deserialize player profile".to_string(),
                    )
                });
                result
            }

            Err(e) => {
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                Logger::error(&format!("[PLAYER] Profile fetch error ({})", status));
                return match status {
                    StatusCode::UNAUTHORIZED => Err(PlayerConnectionError::UnauthorizedPlayerError),
                    _ => Err(PlayerConnectionError::UnexpectedPlayerError),
                };
            }
        };
    }
}
