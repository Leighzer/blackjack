use crate::enums::PlayerAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub balance: i32,
}

pub struct Player {
    pub hands: Vec<PlayerHand>,
}

pub struct PlayerHand {
    pub cards: Vec<u8>,
    pub bet: i32,
    pub payout: Option<i32>,
    pub is_complete_taking_actions: bool,
    pub avaiable_actions: Vec<PlayerAction>,
    pub previous_actions_taken: Vec<PlayerAction>,
    pub is_starting_hand: bool,
}

// pub struct Card {
//     pub face_value: char,
//     pub numeric_value: u8,
//     pub is_visible: bool,
// }
