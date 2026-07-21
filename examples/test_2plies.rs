use postflop_solver::*;
use std::time::Instant;

fn main() {
    let flop = flop_from_str("QsJh4d").unwrap();
    let oop: Range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".parse().unwrap();
    let ip: Range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".parse().unwrap();

    let card_config = CardConfig{range:[oop,ip],flop,turn:NOT_DEALT,river:NOT_DEALT};
    let bet_sizes = BetSizeOptions::try_from(("60%, e, a", "2.5x")).unwrap();
    let tree_config = TreeConfig {
        initial_state: BoardState::Flop, starting_pot: 50, effective_stack: 250,
        rake_rate: 0.0, rake_cap: 0.0,
        flop_bet_sizes: [bet_sizes.clone(),bet_sizes.clone()],
        turn_bet_sizes: [bet_sizes.clone(),bet_sizes.clone()],
        river_bet_sizes: [bet_sizes.clone(),bet_sizes],
        turn_donk_sizes: None, river_donk_sizes: None,
        add_allin_threshold: 1.5, force_allin_threshold: 0.15, merging_threshold: 0.1,
        depth_limit: Some(BoardState::Flop),
        two_plies_lookahead: true,
    };
    let target = 50f32 * 0.005;

    println!("Full solve...");
    let t = Instant::now();
    let config_f = tree_config.clone();
    let mut game_f = PostFlopGame::with_config(card_config.clone(), ActionTree::new(TreeConfig{two_plies_lookahead:false,depth_limit:None,..config_f}).unwrap()).unwrap();
    game_f.allocate_memory(false);
    let exp_f = solve(&mut game_f, 2000, target, false);
    println!("{:.1}s (exploit={:.4})", t.elapsed().as_secs_f64(), exp_f);

    println!("方案C DL solve...");
    let t = Instant::now();
    let mut game_dl = PostFlopGame::with_config(card_config.clone(), ActionTree::new(tree_config).unwrap()).unwrap();
    game_dl.allocate_memory(false);
    let exp_dl = solve(&mut game_dl, 2000, target, false);
    println!("{:.1}s (exploit={:.4})", t.elapsed().as_secs_f64(), exp_dl);
}
