 use arcis_imports::*;

 #[encrypted]
 mod circuits {
     use arcis_imports::*;

    #[instruction]
    pub fn spin_slots(_mxe: Mxe) -> (u8, u8, u8) {
        let mut digits = [0u8,1,2,3,4,5,6,7,8,9];
        ArcisRNG::shuffle(&mut digits);
        (digits[0], digits[1], digits[2]).reveal()
    }

    #[instruction]
    pub fn roll_roulette(_mxe: Mxe) -> u8 {
        let mut nums = [
            0u8,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,
            19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36
        ];
        ArcisRNG::shuffle(&mut nums);
        nums[0].reveal()
    }

     // Coinflip: player sends encrypted choice; we generate RNG bool and compare
     pub struct UserChoice {
         pub choice: bool,
     }

    #[instruction]
    pub fn flip(input_ctxt: Enc<Shared, UserChoice>) -> bool {
        let input = input_ctxt.to_arcis();
        let toss = ArcisRNG::bool();
        (input.choice == toss).reveal()
    }

    #[instruction]
    pub fn roll_dice(_mxe: Mxe) -> u8 {
        let mut faces = [1u8,2,3,4,5,6];
        ArcisRNG::shuffle(&mut faces);
        faces[0].reveal()
    }

    // Blackjack circuits
    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    const POWS_OF_SIXTY_FOUR: [u128; 21] = [
        1, 64, 4096, 262144, 16777216, 1073741824, 68719476736, 4398046511104,
        281474976710656, 18014398509481984, 1152921504606846976, 73786976294838206464,
        4722366482869645213696, 302231454903657293676544, 19342813113834066795298816,
        1237940039285380274899124224, 79228162514264337593543950336,
        5070602400912917605986812821504, 324518553658426726783156020576256,
        20769187434139310514121985316880384, 1329227995784915872903807060280344576,
    ];

    pub struct Deck {
        pub card_one: u128,
        pub card_two: u128,
        pub card_three: u128,
    }

    impl Deck {
        pub fn from_array(array: [u8; 52]) -> Deck {
            let mut card_one = 0;
            for i in 0..21 {
                card_one += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }
            let mut card_two = 0;
            for i in 21..42 {
                card_two += POWS_OF_SIXTY_FOUR[i - 21] * array[i] as u128;
            }
            let mut card_three = 0;
            for i in 42..52 {
                card_three += POWS_OF_SIXTY_FOUR[i - 42] * array[i] as u128;
            }
            Deck { card_one, card_two, card_three }
        }

        fn to_array(&self) -> [u8; 52] {
            let mut card_one = self.card_one;
            let mut card_two = self.card_two;
            let mut card_three = self.card_three;
            let mut bytes = [0u8; 52];
            for i in 0..21 {
                bytes[i] = (card_one % 64) as u8;
                bytes[i + 21] = (card_two % 64) as u8;
                card_one >>= 6;
                card_two >>= 6;
            }
            for i in 42..52 {
                bytes[i] = (card_three % 64) as u8;
                card_three >>= 6;
            }
            bytes
        }
    }

    pub struct Hand {
        pub cards: u128,
    }

    impl Hand {
        pub fn from_array(array: [u8; 11]) -> Hand {
            let mut cards = 0;
            for i in 0..11 {
                cards += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }
            Hand { cards }
        }

        fn to_array(&self) -> [u8; 11] {
            let mut cards = self.cards;
            let mut bytes = [0u8; 11];
            for i in 0..11 {
                bytes[i] = (cards % 64) as u8;
                cards >>= 6;
            }
            bytes
        }
    }

    fn calculate_hand_value(hand: &[u8; 11], hand_length: u8) -> u8 {
        let mut value = 0;
        let mut has_ace = false;
        for i in 0..11 {
            let rank = if i < hand_length as usize { (hand[i] % 13) } else { 0 };
            if i < hand_length as usize {
                if rank == 0 {
                    value += 11;
                    has_ace = true;
                } else if rank > 10 {
                    value += 10;
                } else {
                    value += rank;
                }
            }
        }
        if value > 21 && has_ace {
            value -= 10;
        }
        value
    }

    #[instruction]
    pub fn shuffle_and_deal_cards(
        mxe: Mxe,
        mxe_again: Mxe,
        client: Shared,
        client_again: Shared,
    ) -> (
        Enc<Mxe, Deck>,
        Enc<Mxe, Hand>,
        Enc<Shared, Hand>,
        Enc<Shared, u8>,
    ) {
        let mut initial_deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);
        let deck = mxe.from_arcis(Deck::from_array(initial_deck));
        let mut dealer_cards = [53; 11];
        dealer_cards[0] = initial_deck[1];
        dealer_cards[1] = initial_deck[3];
        let dealer_hand = mxe_again.from_arcis(Hand::from_array(dealer_cards));
        let mut player_cards = [53; 11];
        player_cards[0] = initial_deck[0];
        player_cards[1] = initial_deck[2];
        let player_hand = client.from_arcis(Hand::from_array(player_cards));
        (deck, dealer_hand, player_hand, client_again.from_arcis(initial_deck[1]))
    }

    #[instruction]
    pub fn player_hit(
        deck_ctxt: Enc<Mxe, Deck>,
        player_hand_ctxt: Enc<Shared, Hand>,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Shared, Hand>, bool) {
        let deck = deck_ctxt.to_arcis().to_array();
        let mut player_hand = player_hand_ctxt.to_arcis().to_array();
        let player_hand_value = calculate_hand_value(&player_hand, player_hand_size);
        let is_bust = player_hand_value > 21;
        let new_card = if !is_bust {
            deck[(player_hand_size + dealer_hand_size) as usize]
        } else {
            53
        };
        player_hand[player_hand_size as usize] = new_card;
        (player_hand_ctxt.owner.from_arcis(Hand::from_array(player_hand)), is_bust.reveal())
    }

    #[instruction]
    pub fn player_stand(player_hand_ctxt: Enc<Shared, Hand>, player_hand_size: u8) -> bool {
        let player_hand = player_hand_ctxt.to_arcis().to_array();
        let value = calculate_hand_value(&player_hand, player_hand_size);
        (value > 21).reveal()
    }

    #[instruction]
    pub fn player_double_down(
        deck_ctxt: Enc<Mxe, Deck>,
        player_hand_ctxt: Enc<Shared, Hand>,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Shared, Hand>, bool) {
        let deck_array = deck_ctxt.to_arcis().to_array();
        let mut player_hand = player_hand_ctxt.to_arcis().to_array();
        let player_hand_value = calculate_hand_value(&player_hand, player_hand_size);
        let is_bust = player_hand_value > 21;
        let new_card = if !is_bust {
            deck_array[(player_hand_size + dealer_hand_size) as usize]
        } else {
            53
        };
        player_hand[player_hand_size as usize] = new_card;
        (player_hand_ctxt.owner.from_arcis(Hand::from_array(player_hand)), is_bust.reveal())
    }

    #[instruction]
    pub fn dealer_play(
        deck_ctxt: Enc<Mxe, Deck>,
        dealer_hand_ctxt: Enc<Mxe, Hand>,
        client: Shared,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Mxe, Hand>, Enc<Shared, Hand>, u8) {
        let deck_array = deck_ctxt.to_arcis().to_array();
        let mut dealer = dealer_hand_ctxt.to_arcis().to_array();
        let mut size = dealer_hand_size as usize;
        for _i in 0..7 {
            let val = calculate_hand_value(&dealer, size as u8);
            if val < 17 {
                dealer[size] = deck_array[(player_hand_size as usize + size)];
                size += 1;
            }
        }
        (dealer_hand_ctxt.owner.from_arcis(Hand::from_array(dealer)), client.from_arcis(Hand::from_array(dealer)), (size as u8).reveal())
    }

    #[instruction]
    pub fn resolve_game(
        player_hand: Enc<Shared, Hand>,
        dealer_hand: Enc<Mxe, Hand>,
        player_hand_length: u8,
        dealer_hand_length: u8,
    ) -> u8 {
        let player_hand = player_hand.to_arcis().to_array();
        let dealer_hand = dealer_hand.to_arcis().to_array();
        let player_value = calculate_hand_value(&player_hand, player_hand_length);
        let dealer_value = calculate_hand_value(&dealer_hand, dealer_hand_length);
        let result = if player_value > 21 {
            0
        } else if dealer_value > 21 {
            1
        } else if player_value > dealer_value {
            2
        } else if dealer_value > player_value {
            3
        } else {
            4
        };
        result.reveal()
    }
}

