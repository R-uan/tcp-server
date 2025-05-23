use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::game::player::Player;
use super::deck::CardRef;

#[derive(Serialize, Clone)]
pub struct PrivateGameStateView {
    pub turn: u32,
    pub red_player: PrivatePlayerView,
    pub blue_player: PrivatePlayerView,
}

#[derive(Serialize, Clone)]
pub struct PublicGameStateView {
    pub turn: u32,
    pub red_player: PublicPlayerView,
    pub blue_player: PublicPlayerView,
}

#[derive(Serialize, Clone)]
pub struct PublicPlayerView {
    pub id: String,
    pub health: i32,
    pub mana: u32,

    pub hand_size: usize,
    pub deck_size: usize,
    pub graveyard_size: usize,
    
    pub board: BoardView,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivatePlayerView {
    pub id: String,
    pub health: i32,
    pub mana: u32,

    pub hand_size: usize,
    pub deck_size: usize,
    pub current_hand: [Option<CardView>; 10],

    pub board: BoardView,
    pub graveyard_size: usize,
    pub graveyard: GraveyardView,
}

impl PrivatePlayerView {
    pub fn from_player(player: Arc<Player>) -> Self {
        PrivatePlayerView {
            id: player.id.clone(),
            health: 30,
            mana: 1,

            hand_size: 0,
            board: BoardView::default(),
            deck_size: player.current_deck.cards.len(),
            graveyard: GraveyardView::default(),
            graveyard_size: 0,
            current_hand: [None, None, None, None, None, None, None, None, None, None],
        }
    }
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct CardView {
    pub id: String,
    pub hand_id: u32,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub card_type: String,
    pub effects: Vec<String>,
    pub owner_id: String,
    pub is_exhausted: bool,
    pub position: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct BoardView {
    pub creatures: [Option<CardRef>; 6],
    pub artifacts: [Option<CardRef>; 3],
    pub enchantments: [Option<CardRef>; 3],
}

impl Default for BoardView {
    fn default() -> Self {
        Self {
            artifacts: [None, None, None],
            enchantments: [None, None, None],
            creatures: [None, None, None, None, None, None],
        }
    }
}

#[derive(Serialize, Clone, Deserialize, Debug, Default)]
pub struct GraveyardView {
    pub creatures: Vec<CardRef>,
    pub artifacts: Vec<CardRef>,
    pub enchantments: Vec<CardRef>,
}
