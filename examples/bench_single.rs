use clap::Parser;
use postflop_solver::*;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "bench_single", about = "单牌面 Full EV vs DL EV 计算工具")]
struct Args {
    /// 翻牌，如 "8h8s8d"
    #[arg(long)]
    flop: String,

    /// 输出 CSV 文件路径
    #[arg(long)]
    output: String,

    /// 底池大小（BB）
    #[arg(long, default_value = "5")]
    pot: i32,

    /// 有效筹码（BB）
    #[arg(long, default_value = "100")]
    stack: i32,

    /// 抽水比例（0.0 ~ 1.0）
    #[arg(long, default_value = "0.0")]
    rake: f64,
}

fn board_texture_label(texture: BoardTexture) -> &'static str {
    match texture {
        BoardTexture::Trips => "三条面",
        BoardTexture::PairedRainbow => "对子彩虹",
        BoardTexture::PairedTwoTone => "对子两同花",
        BoardTexture::Rainbow => "纯彩虹",
        BoardTexture::TwoToneA => "两同花A",
        BoardTexture::TwoToneB => "两同花B",
        BoardTexture::MonotoneA => "单调面A",
        BoardTexture::MonotoneB => "单调面B",
    }
}

fn main() {
    let args = Args::parse();

    let flop = flop_from_str(&args.flop).expect("Invalid flop format. Use e.g. '8h8s8d'");
    let texture = detect_board_texture(flop);
    let texture_label = board_texture_label(texture);

    // BB unit: multiply by 10 for integer pot/stack
    let starting_pot = args.pot * 10;
    let effective_stack = args.stack * 10;
    let spr = args.stack as f64 / args.pot as f64;

    let oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
    let ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

    let card_config = CardConfig {
        range: [oop_range.parse().unwrap(), ip_range.parse().unwrap()],
        flop,
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };

    let bet_sizes = BetSizeOptions::try_from(("60%, e, a", "2.5x")).unwrap();

    let base_tree_config = TreeConfig {
        initial_state: BoardState::Flop,
        starting_pot,
        effective_stack,
        rake_rate: args.rake,
        rake_cap: 0.0,
        flop_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
        turn_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
        river_bet_sizes: [bet_sizes.clone(), bet_sizes],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 1.5,
        force_allin_threshold: 0.15,
        merging_threshold: 0.1,
        ..Default::default()
    };

    let target_exp = starting_pot as f32 * 0.005;

    println!("牌面: {} ({})", args.flop, texture_label);
    println!("SPR: {:.1} (pot={}BB, stack={}BB)", spr, args.pot, args.stack);

    // === Full Solve ===
    print!("Full solve... ");
    let t = Instant::now();
    let mut game_full = PostFlopGame::with_config(
        card_config.clone(),
        ActionTree::new(base_tree_config.clone()).unwrap(),
    )
    .unwrap();
    game_full.allocate_memory(false);
    let exp_full = solve(&mut game_full, 2000, target_exp, false);
    println!("{:.1}s (exploit={:.4})", t.elapsed().as_secs_f64(), exp_full);

    // === DL Solve ===
    print!("DL solve... ");
    let t = Instant::now();
    let mut dl_config = base_tree_config.clone();
    dl_config.depth_limit = Some(BoardState::Flop);
    dl_config.equity_pos_correction = 0.05;
    let mut game_dl = PostFlopGame::with_config(
        card_config.clone(),
        ActionTree::new(dl_config).unwrap(),
    )
    .unwrap();
    game_dl.allocate_memory(false);
    let exp_dl = solve(&mut game_dl, 2000, target_exp, false);
    println!("{:.1}s (exploit={:.4})", t.elapsed().as_secs_f64(), exp_dl);

    // === Collect data ===
    game_full.back_to_root();
    game_dl.back_to_root();
    game_full.cache_normalized_weights();
    game_dl.cache_normalized_weights();

    let mut csv_lines = Vec::new();
    csv_lines.push(format!("牌面: {}", args.flop));
    csv_lines.push(format!("牌面类型: {}", texture_label));
    csv_lines.push(format!("SPR: {:.1}", spr));
    csv_lines.push(
        "位置,手牌,大桶,Draw,FullEV,DLEv,DLEv_corrected,DLEv偏差%,修正后偏差%,修正结果".to_string(),
    );

    let bucket_labels = [
        "桶1:超强成牌", "桶2:中等顶对", "桶3:中等成牌",
        "桶4:超强听牌", "桶5:常规强听牌", "桶6:弱听牌", "桶7:纯空气",
    ];
    let positions = ["OOP", "IP"];

    for player in 0..2 {
        let cards = game_dl.private_cards(player);
        let ev_dl = game_dl.expected_values(player);
        let ev_corrected = game_dl.expected_values_with_board_correction(player);
        let ev_full = game_full.expected_values(player);
        let weights = game_full.normalized_weights(player);
        let names = holes_to_strings(cards).unwrap();

        // Collect and sort by full EV
        let mut rows: Vec<(String, String, String, f32, f32, f32)> = Vec::new();
        for (i, name) in names.iter().enumerate() {
            if weights[i] == 0.0 { continue; }
            let (c1, c2) = cards[i];
            let (cat, draw) = classify_hand((c1, c2), flop, NOT_DEALT);
            let bucket = HandBucket::classify(cat, draw, (c1, c2), flop);
            rows.push((
                name.clone(),
                bucket_labels[bucket as usize].to_string(),
                format!("{:?}", draw),
                ev_full[i],
                ev_dl[i],
                ev_corrected[i],
            ));
        }
        rows.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());

        for (name, bucket, draw, full, dl, corrected) in &rows {
            let dl_bias = if full.abs() > 0.01 {
                (dl - full) / full * 100.0
            } else {
                0.0
            };
            let corr_bias = if full.abs() > 0.01 {
                (corrected - full) / full * 100.0
            } else {
                0.0
            };
            let result = if corr_bias.abs() < dl_bias.abs() - 0.01 {
                "改善"
            } else if corr_bias.abs() > dl_bias.abs() + 0.01 {
                "恶化"
            } else {
                "没改变"
            };

            csv_lines.push(format!(
                "{},{},{},{},{:.2},{:.2},{:.2},{:+.1}%,{:+.1}%,{}",
                positions[player], name, bucket, draw, full, dl, corrected, dl_bias, corr_bias, result,
            ));
        }
    }

    let csv_content = csv_lines.join("\n") + "\n";
    std::fs::write(&args.output, &csv_content).expect("Failed to write CSV");
    println!("CSV 已保存: {}", args.output);
}
