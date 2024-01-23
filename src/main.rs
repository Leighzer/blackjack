use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

// TODO bug fixes and clean up messaging/outputs - getting there is pretty good now

// Cards
// 1(or 11) 2, 3, 4, 5, 6, 7, 8, 9, 10, J(10), Q(10), K(10)

const PLAYER_STARTING_BALANCE: i32 = 500;

// fun house rules
const ALLOW_SPLIT_OF_SPLIT: bool = true;
const ALLOW_DOUBLE_DOWN_ON_SPLIT: bool = true;

fn main() {
    let mut is_game_running: bool = true;

    create_player_profile_if_not_exists();

    let mut player_profile: PlayerProfile = load_player_profile_from_disk();

    if player_profile.balance <= 0 {
        println!(
            "We see you are out of chips. Here, have {} chips on the house.",
            PLAYER_STARTING_BALANCE
        );
        player_profile.balance = PLAYER_STARTING_BALANCE;
        save_player_profile_to_disk(&player_profile);
    }

    let mut player_action_buffer = String::new();

    let mut deck: Vec<u8> = Vec::<u8>::new();

    while is_game_running {
        println!("You now have {} chips.", player_profile.balance);
        println!("How much would you like to bet? (e)xit if you would like to leave the table.");

        let mut player_bet = 0;
        let mut has_player_bet = false;

        while !has_player_bet {
            stdin()
                .read_line(&mut player_action_buffer)
                .expect("Error: failed to read input from stdin.");

            match player_action_buffer.to_lowercase().trim() {
                "e" => {
                    println!("Thanks for playing.");
                    std::process::exit(0);
                }
                val => match val.parse::<i32>() {
                    Ok(integer) => {
                        if integer > player_profile.balance {
                            println!(
                                    "You can only bet up to your balance {}. Please enter your bet again.",
                                    player_profile.balance
                                );
                        } else if integer <= 0 {
                            println!("You must bet at least 1 chip to play.");
                        } else {
                            player_bet = integer;
                            has_player_bet = true;
                        }
                    }
                    Err(_) => {
                        println!("Invalid input. Please enter your bet or (e)xit the table.");
                    }
                },
            }
            player_action_buffer = String::new();
        }

        player_profile.balance += play_round(&mut deck, player_profile.balance, player_bet);

        save_player_profile_to_disk(&player_profile);

        if player_profile.balance <= 0 {
            println!("You are broke. You have been kicked out of the casino.");
            is_game_running = false;
        }
    }
}

fn play_round(deck: &mut Vec<u8>, initial_player_balance: i32, initial_player_bet: i32) -> i32 {
    let mut player = Player {
        hands: vec![PlayerHand {
            cards: vec![],
            bet: initial_player_bet,
            payout: None,
            is_complete_taking_actions: false,
            avaiable_actions: vec![],
            previous_actions_taken: vec![],
            is_starting_hand: true,
        }],
    };

    let mut player_working_balance = initial_player_balance - initial_player_bet;

    let mut dealer_hand: Vec<u8> = Vec::<u8>::new();

    let mut player_action_buffer = String::new();

    for hand in &mut player.hands {
        deal_from_deck(deck, hand);
    }
    deal_from_deck_legacy(deck, &mut dealer_hand);

    for hand in &mut player.hands {
        deal_from_deck(deck, hand);
        hand.avaiable_actions = get_player_actions(player_working_balance, hand);
    }
    deal_from_deck_legacy(deck, &mut dealer_hand);

    let is_dealer_blackjack = get_hand_sum_legacy(&dealer_hand) == 21;
    let mut is_any_blackjack = is_dealer_blackjack;
    for hand in &mut player.hands {
        let is_hand_blackjack = get_hand_sum(hand) == 21;
        is_any_blackjack |= is_hand_blackjack;
        if is_hand_blackjack && is_dealer_blackjack {
            println!("You and the dealer hit blackjack!");
            hand.is_complete_taking_actions = true;
            hand.payout = Some(0);
        } else if is_hand_blackjack {
            println!("You hit blackjack!");
            hand.is_complete_taking_actions = true;
            hand.payout = Some((hand.bet as f32 * 1.5) as i32);
        } else if is_dealer_blackjack {
            println!("The dealer hit blackjack!");
            hand.is_complete_taking_actions = true;
            hand.payout = Some(-hand.bet);
        }
    }

    // we'll show all cards if there is a blackjack as for now
    // the game would immediately end - let's let players count cards ;)
    if is_any_blackjack {
        print_hands(&dealer_hand, &player, false);
    }

    // play out all hands here
    while !player
        .hands
        .iter()
        .all(|hand| hand.is_complete_taking_actions)
    {
        // unwrap as we know there is an incomplete hand among the player's hands
        let first_incomplete_hand_index: usize = get_first_incomplete_hand_index(&player).unwrap();

        // player action loop until they are done with hand
        while !player.hands[first_incomplete_hand_index].is_complete_taking_actions {
            print_hands(&dealer_hand, &player, true);
            print_player_actions(&player.hands[first_incomplete_hand_index].avaiable_actions);

            let mut has_player_action = false;
            while !has_player_action {
                stdin()
                    .read_line(&mut player_action_buffer)
                    .expect("Error: failed to read player input from stdin.");
                match player_action_buffer.to_lowercase().trim() {
                    "h" => {
                        if !player.hands[first_incomplete_hand_index]
                            .avaiable_actions
                            .contains(&PlayerAction::Hit)
                        {
                            println!("You cannot hit at this time. Please enter a valid option.");
                            print_player_actions(
                                &player.hands[first_incomplete_hand_index].avaiable_actions,
                            );
                        } else {
                            player.hands[first_incomplete_hand_index]
                                .previous_actions_taken
                                .push(PlayerAction::Hit);
                            has_player_action = true;

                            println!("You decided to hit!");
                            deal_from_deck(deck, &mut player.hands[first_incomplete_hand_index]);
                            let player_hand_sum: u8 =
                                get_hand_sum(&player.hands[first_incomplete_hand_index]);

                            if player_hand_sum > 21 {
                                println!("Sorry you have busted!");
                                player.hands[first_incomplete_hand_index]
                                    .is_complete_taking_actions = true;
                                player.hands[first_incomplete_hand_index].payout =
                                    Some(-player.hands[first_incomplete_hand_index].bet);
                            }
                        }
                    }
                    "s" => {
                        if !player.hands[first_incomplete_hand_index]
                            .avaiable_actions
                            .contains(&PlayerAction::Stay)
                        {
                            println!("You cannot stay at this time. Please enter a valid option.");
                            print_player_actions(
                                &player.hands[first_incomplete_hand_index].avaiable_actions,
                            );
                        } else {
                            player.hands[first_incomplete_hand_index]
                                .previous_actions_taken
                                .push(PlayerAction::Stay);
                            has_player_action = true;

                            println!("You decided to stay!");
                            player.hands[first_incomplete_hand_index].is_complete_taking_actions =
                                true;
                        }
                    }
                    "d" => {
                        if !player.hands[first_incomplete_hand_index]
                            .avaiable_actions
                            .contains(&PlayerAction::DoubleDown)
                        {
                            println!(
                                "You cannot double down at this time. Please enter a valid option."
                            );
                            print_player_actions(
                                &player.hands[first_incomplete_hand_index].avaiable_actions,
                            );
                        } else {
                            player.hands[first_incomplete_hand_index]
                                .previous_actions_taken
                                .push(PlayerAction::DoubleDown);
                            has_player_action = true;

                            player_working_balance -= player.hands[first_incomplete_hand_index].bet;
                            player.hands[first_incomplete_hand_index].bet *= 2;
                            println!(
                                "You decided to double down! Your bet for this hand is now {}!",
                                player.hands[first_incomplete_hand_index].bet
                            );
                            deal_from_deck(deck, &mut player.hands[first_incomplete_hand_index]);
                            let player_hand_sum: u8 =
                                get_hand_sum(&player.hands[first_incomplete_hand_index]);
                            if player_hand_sum > 21 {
                                println!("Sorry you have busted!");
                                player.hands[first_incomplete_hand_index].payout =
                                    Some(-player.hands[first_incomplete_hand_index].bet);
                            }

                            player.hands[first_incomplete_hand_index].is_complete_taking_actions =
                                true;

                            // show the hit even though we'll continue on to the dealer for more suspense
                            print_hands(&dealer_hand, &player, true);
                        }
                    }
                    "p" => {
                        if !player.hands[first_incomplete_hand_index]
                            .avaiable_actions
                            .contains(&PlayerAction::Split)
                        {
                            println!("You cannot split at this time. Please enter a valid option.");
                            print_player_actions(
                                &player.hands[first_incomplete_hand_index].avaiable_actions,
                            );
                        } else {
                            player.hands[first_incomplete_hand_index]
                                .previous_actions_taken
                                .push(PlayerAction::Split);
                            has_player_action = true;
                            player_working_balance -= player.hands[first_incomplete_hand_index].bet;

                            player.hands[first_incomplete_hand_index].cards.remove(0);

                            deal_from_deck(deck, &mut player.hands[first_incomplete_hand_index]);

                            let mut new_hand = PlayerHand {
                                cards: vec![player.hands[first_incomplete_hand_index].cards[0]],
                                bet: player.hands[first_incomplete_hand_index].bet,
                                payout: None,
                                is_complete_taking_actions: false,
                                avaiable_actions: vec![],
                                previous_actions_taken: vec![],
                                is_starting_hand: false,
                            };
                            deal_from_deck(deck, &mut new_hand);
                            new_hand.avaiable_actions =
                                get_player_actions(player_working_balance, &mut new_hand);
                            player.hands.push(new_hand);
                        }
                    }
                    _ => {
                        println!("Please enter a valid option.");
                        print_player_actions(
                            &player.hands[first_incomplete_hand_index].avaiable_actions,
                        );
                    }
                }
                player_action_buffer = String::new();
                player.hands[first_incomplete_hand_index].avaiable_actions = get_player_actions(
                    player_working_balance,
                    &player.hands[first_incomplete_hand_index],
                );
            }
        }
    }

    // if we need to play out the dealer hand to pay out remaining hands
    if !player.hands.iter().all(|hand| hand.payout.is_some()) {
        println!("Dealer hand starts!");

        let mut is_dealer_hand_done = false;
        while !is_dealer_hand_done {
            let dealer_hand_sum: u8 = get_hand_sum_legacy(&dealer_hand);

            if dealer_hand_sum < 17 {
                // dealer hit
                println!("Dealer hits!");
                deal_from_deck_legacy(deck, &mut dealer_hand);

                let mut dealer_hand_sum = 0;
                for i in &dealer_hand {
                    dealer_hand_sum += i;
                }

                if dealer_hand_sum > 21 {
                    println!("Dealer has busted!");
                    is_dealer_hand_done = true;
                    for hand in &mut player.hands {
                        if hand.payout.is_none() {
                            hand.payout = Some(hand.bet);
                        }
                    }
                }
            } else {
                // dealer stay
                println!("Dealer stays!");
                is_dealer_hand_done = true;
            }
            print_hands(&dealer_hand, &player, false);
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    // if there is not already a winner from earlier
    let has_hands_to_resolve = player.hands.iter().any(|h| h.payout.is_none());
    if has_hands_to_resolve {
        // compare hands
        let dealer_hand_sum = get_hand_sum_legacy(&dealer_hand);
        println!("Dealer has {}", dealer_hand_sum);
        for i in 0..player.hands.len() {
            if player.hands[i].payout.is_none() {
                let hand_sum = get_hand_sum(&player.hands[i]);
                if player.hands.len() > 1 {
                    println!("Player hand {} has {}", i + 1, hand_sum);
                } else {
                    println!("Player has {}", hand_sum);
                }

                match hand_sum.cmp(&dealer_hand_sum) {
                    Ordering::Equal => {
                        player.hands[i].payout = Some(0);
                    }
                    Ordering::Greater => {
                        player.hands[i].payout = Some(player.hands[i].bet);
                    }
                    Ordering::Less => {
                        player.hands[i].payout = Some(-player.hands[i].bet);
                    }
                }
            }
        }
    }

    let mut total_payout = 0;
    for hand in &player.hands {
        let payout = hand.payout.expect("Error payout does not have value.");
        if payout > 0 {
            println!("You won {}!", payout.abs());
        } else if payout == 0 {
            println!("Push!");
        } else {
            println!("You lost {}!", payout.abs());
        }
        total_payout += payout;
    }

    if player.hands.len() > 1 {
        if total_payout > 0 {
            println!("In total you won {}!", total_payout.abs());
        } else if total_payout == 0 {
            println!("In total it was a push!");
        } else {
            println!("In total you lost {}!", total_payout.abs());
        }
    }

    total_payout
}

fn get_first_incomplete_hand_index(player: &Player) -> Option<usize> {
    player
        .hands
        .iter()
        .position(|hand| !hand.is_complete_taking_actions)
}

fn deal_from_deck(deck: &mut Vec<u8>, hand: &mut PlayerHand) {
    if deck.is_empty() {
        shuffle_new_deck(deck);
    }

    let card = deck.remove(deck.len() - 1);

    hand.cards.push(card);
}

fn deal_from_deck_legacy(deck: &mut Vec<u8>, hand: &mut Vec<u8>) {
    if deck.is_empty() {
        shuffle_new_deck(deck);
    }

    let card = deck.remove(deck.len() - 1);

    hand.push(card);
}

fn shuffle_new_deck(deck: &mut Vec<u8>) {
    for _ in 0..4 {
        for j in 1..=13 {
            if j > 10 {
                deck.push(10);
            } else {
                deck.push(j);
            }
        }
    }

    deck.shuffle(&mut rand::thread_rng());
}

fn print_hands(dealer_hand: &Vec<u8>, player: &Player, hide_first_dealer_card: bool) {
    print_hand_legacy("Dealer", dealer_hand, hide_first_dealer_card);
    let first_incomplete_hand_index = get_first_incomplete_hand_index(&player);
    if player.hands.len() > 1 {
        for i in 0..player.hands.len() {
            let needs_active_marker = match first_incomplete_hand_index {
                Some(val) => i == val,
                None => false,
            };
            
            print_hand(
                format!("Player hand {}", i + 1).as_str(),
                &player.hands[i],
                needs_active_marker,
            );
        }
    } else if player.hands.len() == 1 {
        print_hand("Player", &player.hands[0], false);
    }
}

fn print_hand(player_name: &str, hand: &PlayerHand, display_active_marker: bool) {
    let mut hand_string = "[".to_string();
    for i in 0..hand.cards.len() {
        hand_string.push_str(&(hand.cards[i].to_string()));
        if i < hand.cards.len() - 1 {
            hand_string.push(' ');
        }
    }
    hand_string.push(']');

    if display_active_marker {
        hand_string.push_str("*");
    }

    println!("{}: {}", player_name, hand_string);
}

fn print_hand_legacy(player_name: &str, hand: &Vec<u8>, hide_first_card: bool) {
    let mut hand_string = "[".to_string();
    if hide_first_card {
        for i in 0..hand.len() {
            if i == 0 {
                hand_string.push('*');
            } else {
                hand_string.push_str(&(hand[i].to_string()));
            }
            if i < hand.len() - 1 {
                hand_string.push(' ');
            }
        }
    } else {
        for i in 0..hand.len() {
            hand_string.push_str(&(hand[i].to_string()));
            if i < hand.len() - 1 {
                hand_string.push(' ');
            }
        }
    }
    hand_string.push(']');

    println!("{}: {}", player_name, hand_string);
}

fn all_elements_equal<T: PartialEq>(vec: &[T]) -> bool {
    vec.first()
        .map(|first| vec.iter().all(|x| x == first))
        .unwrap_or(true)
}

fn get_player_actions(player_working_balance: i32, player_hand: &PlayerHand) -> Vec<PlayerAction> {
    let mut player_actions = vec![PlayerAction::Hit, PlayerAction::Stay];

    if player_hand.bet <= player_working_balance
        && !player_hand
            .previous_actions_taken
            .contains(&PlayerAction::DoubleDown)
        && (ALLOW_DOUBLE_DOWN_ON_SPLIT || player_hand.is_starting_hand)
    {
        player_actions.push(PlayerAction::DoubleDown);
    }

    if player_hand.cards.len() == 2
        && all_elements_equal(&player_hand.cards)
        && (ALLOW_SPLIT_OF_SPLIT || player_hand.is_starting_hand)
        && player_hand.bet <= player_working_balance
        && !player_hand
            .previous_actions_taken
            .contains(&PlayerAction::Split)
    {
        player_actions.push(PlayerAction::Split);
    }

    player_actions
}

fn print_player_actions(player_actions: &[PlayerAction]) {
    let player_actions_string_output = player_actions
        .iter()
        .map(|action| match action {
            PlayerAction::Hit => "(h)it",
            PlayerAction::Stay => "(s)tay",
            PlayerAction::DoubleDown => "(d)ouble down",
            PlayerAction::Split => "s(p)lit",
        })
        .collect::<Vec<_>>()
        .join(" ");

    println!("{}", player_actions_string_output)
}

#[derive(Debug, PartialEq)]
enum PlayerAction {
    Hit,
    Stay,
    DoubleDown,
    Split,
}

fn get_player_profile_path_buf() -> PathBuf {
    let exe_path =
        std::env::current_exe().expect("Error: Failed to get the current executable path.");
    let exe_dir = exe_path
        .parent()
        .expect("Error: Failed to get directory of the current executable.");
    let file_name = "player_profile.json";
    let full_path = exe_dir.join(file_name);

    #[allow(clippy::let_and_return)]
    full_path
}

fn create_player_profile_if_not_exists() {
    let full_path = get_player_profile_path_buf();

    if !full_path.exists() {
        println!(
            "We see you are a new player! We are starting your account with {} chips.",
            PLAYER_STARTING_BALANCE
        );
        save_player_profile_to_disk(&PlayerProfile {
            balance: PLAYER_STARTING_BALANCE,
        })
    }
}

fn load_player_profile_from_disk() -> PlayerProfile {
    let full_path = get_player_profile_path_buf();

    // Open the file in read-only mode.
    let file = File::open(full_path).expect("Error: Player profile file not found.");

    let reader = BufReader::new(file);

    let player_data: PlayerProfile =
        serde_json::from_reader(reader).expect("Error: Failed to parse player profile data.");

    player_data
}

fn save_player_profile_to_disk(player_profile: &PlayerProfile) {
    let full_path = get_player_profile_path_buf();
    let file = File::create(&full_path).unwrap_or_else(|_| {
        panic!(
            "Error: Failed to create player profile file at {}",
            full_path.display()
        )
    });
    serde_json::to_writer(file, player_profile).expect("Error: Failed to save player profile.");
}

fn get_hand_sum(hand: &PlayerHand) -> u8 {
    get_hand_sum_legacy(&hand.cards)
}

fn get_hand_sum_legacy(cards: &[u8]) -> u8 {
    let min_sum: u8 = cards.iter().sum();

    let number_of_aces = cards.iter().filter(|&&x| x == 1_u8).count() as u8;

    let max_ace_10_padding = (21_u8.saturating_sub(min_sum)) / 10_u8; // max amount of 10s we can add without going over 21

    // compare what aces we have to the ideal amount of padding to be added
    // make sure we add the best amount we can considering how much aces we have
    let ace_adjustment = std::cmp::min(number_of_aces, max_ace_10_padding);

    #[allow(clippy::let_and_return)]
    let hand_value = min_sum + (ace_adjustment * 10);

    hand_value
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerProfile {
    pub balance: i32,
}

struct Player {
    pub hands: Vec<PlayerHand>,
}

struct PlayerHand {
    pub cards: Vec<u8>,
    pub bet: i32,
    pub payout: Option<i32>,
    pub is_complete_taking_actions: bool,
    pub avaiable_actions: Vec<PlayerAction>,
    pub previous_actions_taken: Vec<PlayerAction>,
    pub is_starting_hand: bool,
}

#[test]
fn test_get_hand_sum() {
    assert_eq!(get_hand_sum_legacy(&vec![10, 10]), 20);
    assert_eq!(get_hand_sum_legacy(&vec![10, 10, 10]), 30);
    assert_eq!(get_hand_sum_legacy(&vec![5, 6]), 11);
    assert_eq!(get_hand_sum_legacy(&vec![6, 10]), 16);
    assert_eq!(get_hand_sum_legacy(&vec![1, 10]), 21);
    assert_eq!(get_hand_sum_legacy(&vec![4, 5]), 9);
    assert_eq!(get_hand_sum_legacy(&vec![4, 5, 1]), 20);
    assert_eq!(get_hand_sum_legacy(&vec![4, 5, 1, 1]), 21);
    assert_eq!(get_hand_sum_legacy(&vec![4, 5, 1, 1, 1]), 12);
    assert_eq!(get_hand_sum_legacy(&vec![10, 10, 1]), 21);
    assert_eq!(get_hand_sum_legacy(&vec![10, 8]), 18);
    assert_eq!(get_hand_sum_legacy(&vec![10, 8, 1]), 19);
    assert_eq!(get_hand_sum_legacy(&vec![10, 8, 1, 1]), 20);
    assert_eq!(get_hand_sum_legacy(&vec![]), 0);
    assert_eq!(get_hand_sum_legacy(&vec![1]), 11);
    assert_eq!(get_hand_sum_legacy(&vec![1, 1]), 12);
    assert_eq!(get_hand_sum_legacy(&vec![1, 1, 1, 1, 1, 1, 1, 1, 1]), 19);
    assert_eq!(get_hand_sum_legacy(&vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1]), 20);
    assert_eq!(
        get_hand_sum_legacy(&vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        21
    );
    assert_eq!(
        get_hand_sum_legacy(&vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        12
    );
    assert_eq!(get_hand_sum_legacy(&vec![4]), 4);
}
