/// 仅重新运行 DL solve，FullEV 从已有 CSV 读取。
/// 用法：cargo run --release --example rerun_dl

use postflop_solver::*;
use std::collections::HashMap;
use std::time::Instant;

struct BoardDef {
    id: usize,
    name: &'static str,
    flop: &'static str,
    pot: i32,
    stack: i32,
    spr: i32,
    src_dir: &'static str,
    out_dir: &'static str,
}

fn main() {
    let tests = [
        BoardDef { id: 1, name: "三条面", flop: "8h8s8d", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 2, name: "对子彩虹", flop: "KhKd5c", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 3, name: "对子两同花", flop: "KhKc5c", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 4, name: "纯彩虹", flop: "QsJh4d", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 5, name: "两同花A", flop: "QsJs4h", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 6, name: "两同花B", flop: "QhJs4s", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 7, name: "单调面A", flop: "AsJs9s", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 8, name: "单调面B", flop: "8s6s3s", pot: 5, stack: 25, spr: 5, src_dir: "doc/spr5", out_dir: "doc/flop_pure" },
        BoardDef { id: 1, name: "三条面", flop: "8h8s8d", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 2, name: "对子彩虹", flop: "KhKd5c", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 3, name: "对子两同花", flop: "KhKc5c", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 4, name: "纯彩虹", flop: "QsJh4d", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 5, name: "两同花A", flop: "QsJs4h", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 6, name: "两同花B", flop: "QhJs4s", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 7, name: "单调面A", flop: "AsJs9s", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
        BoardDef { id: 8, name: "单调面B", flop: "8s6s3s", pot: 5, stack: 100, spr: 20, src_dir: "doc/spr20", out_dir: "doc/flop_pure" },
    ];

    std::fs::create_dir_all("doc/flop_pure").unwrap();

    for t in &tests {
        let src_file = format!("{}/{}_{}_SPR{}.csv", t.src_dir, t.flop, t.name, t.spr);
        let out_file = format!("{}/{}_{}_SPR{}.csv", t.out_dir, t.flop, t.name, t.spr);

        // Read FullEV from existing CSV
        let src_content = std::fs::read_to_string(&src_file).unwrap_or_else(|_| {
            panic!("Cannot find source file: {}", src_file);
        });
        let src_lines: Vec<&str> = src_content.lines().collect();
        let board_label = src_lines[1].trim_start_matches("牌面类型: ").to_string();

        // Parse hand->FullEV from source
        let mut full_ev_map: HashMap<String, f32> = HashMap::new();
        for line in &src_lines[4..] {
            let cols: Vec<&str> = line.trim().split(',').collect();
            if cols.len() < 10 { continue; }
            full_ev_map.insert(format!("{}|{}|{}", cols[0], cols[1], cols[3]), cols[4].parse().unwrap_or(0.0));
        }

        // === Run DL solve ===
        let flop_cards = flop_from_str(t.flop).unwrap();
        let texture = detect_board_texture(flop_cards);
        let texture_label = match texture {
            BoardTexture::Trips => "三条面",
            BoardTexture::PairedRainbow => "对子彩虹",
            BoardTexture::PairedTwoTone => "对子两同花",
            BoardTexture::Rainbow => "纯彩虹",
            BoardTexture::TwoToneA => "两同花A",
            BoardTexture::TwoToneB => "两同花B",
            BoardTexture::MonotoneA => "单调面A",
            BoardTexture::MonotoneB => "单调面B",
        };

        let starting_pot = t.pot * 10;
        let effective_stack = t.stack * 10;
        let spr = t.stack as f64 / t.pot as f64;

        let oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
        let ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

        let card_config = CardConfig {
            range: [oop_range.parse().unwrap(), ip_range.parse().unwrap()],
            flop: flop_cards,
            turn: NOT_DEALT,
            river: NOT_DEALT,
        };

        let bet_sizes = BetSizeOptions::try_from(("60%, e, a", "2.5x")).unwrap();
        let tree_config = TreeConfig {
            initial_state: BoardState::Flop,
            starting_pot,
            effective_stack,
            rake_rate: 0.0,
            rake_cap: 0.0,
            flop_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
            turn_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
            river_bet_sizes: [bet_sizes.clone(), bet_sizes],
            turn_donk_sizes: None,
            river_donk_sizes: None,
            add_allin_threshold: 1.5,
            force_allin_threshold: 0.15,
            merging_threshold: 0.1,
            depth_limit: Some(BoardState::Flop),
            ..Default::default()
        };

        let target_exp = starting_pot as f32 * 0.005;

        println!("DL solve #{} {} SPR={}...", t.id, t.flop, t.spr);
        let time = Instant::now();
        let mut game_dl = PostFlopGame::with_config(card_config.clone(), ActionTree::new(tree_config).unwrap()).unwrap();
        game_dl.allocate_memory(false);
        let exp_dl = solve(&mut game_dl, 2000, target_exp, false);
        let elapsed = time.elapsed().as_secs_f64();

        game_dl.back_to_root();
        game_dl.cache_normalized_weights();

        let mut csv = String::new();
        csv.push_str(&format!("牌面: {}\n", t.flop));
        csv.push_str(&format!("牌面类型: {}\n", texture_label));
        csv.push_str(&format!("SPR: {:.1}\n", spr));
        csv.push_str("位置,手牌,大桶,Draw,FullEV,DLEv,DLEv偏差%\n");

        let bucket_labels = [
            "桶1:超强成牌", "桶2:中等顶对", "桶3:中等成牌",
            "桶4:超强听牌", "桶5:常规强听牌", "桶6:弱听牌", "桶7:纯空气",
        ];

        for player in 0..2 {
            let cards = game_dl.private_cards(player);
            let ev_dl = game_dl.expected_values(player);
            let weights = game_dl.normalized_weights(player);
            let names = holes_to_strings(cards).unwrap();

            let mut rows: Vec<(String, String, String, String, f32, f32)> = Vec::new();

            for (i, name) in names.iter().enumerate() {
                if weights[i] == 0.0 { continue; }
                let (c1, c2) = cards[i];
                let (cat, draw) = classify_hand((c1, c2), flop_cards, NOT_DEALT);
                let bucket = HandBucket::classify(cat, draw, (c1, c2), flop_cards);
                let pos = if player == 0 { "OOP" } else { "IP" };
                let key = format!("{}|{}|{:?}", pos, name, draw);
                let full = *full_ev_map.get(&key).unwrap_or(&0.0);

                rows.push((
                    pos.to_string(),
                    name.clone(),
                    bucket_labels[bucket as usize].to_string(),
                    format!("{:?}", draw),
                    full,
                    ev_dl[i],
                ));
            }

            rows.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());

            for (pos, name, bucket, draw, full, dl) in &rows {
                let bias = if full.abs() > 0.01 { (dl - full) / full * 100.0 } else { 0.0 };
                csv.push_str(&format!("{},{},{},{},{:.2},{:.2},{:+.1}%\n", pos, name, bucket, draw, full, dl, bias));
            }
        }

        std::fs::write(&out_file, &csv).unwrap();
        println!("  {:.1}s (exploit={:.4}) → {}", elapsed, exp_dl, out_file);
    }
    println!("全部完成");
}
