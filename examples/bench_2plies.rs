use postflop_solver::*;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flop_str = &args[1]; // "QsJh4d"
    let spr: i32 = args[2].parse().unwrap(); // 5 or 20
    let pot = 5;
    let stack = spr * pot;

    let flop = flop_from_str(flop_str).unwrap();
    let oop: Range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".parse().unwrap();
    let ip: Range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".parse().unwrap();
    let cc = CardConfig{range:[oop,ip],flop,turn:NOT_DEALT,river:NOT_DEALT};
    let bs = BetSizeOptions::try_from(("60%, e, a","2.5x")).unwrap();
    let tc = TreeConfig{
        initial_state:BoardState::Flop,starting_pot:pot*10,effective_stack:stack*10,
        rake_rate:0.0,rake_cap:0.0,flop_bet_sizes:[bs.clone(),bs.clone()],
        turn_bet_sizes:[bs.clone(),bs.clone()],river_bet_sizes:[bs.clone(),bs],
        turn_donk_sizes:None,river_donk_sizes:None,
        add_allin_threshold:1.5,force_allin_threshold:0.15,merging_threshold:0.1,
        depth_limit:None,two_plies_lookahead:false,
    };
    let target = (pot*10)as f32*0.005;

    // Full solve
    print!("Full... ");let t=Instant::now();
    let mut gf=PostFlopGame::with_config(cc.clone(),ActionTree::new(tc.clone()).unwrap()).unwrap();
    gf.allocate_memory(false);solve(&mut gf,2000,target,false);
    print!("{:.1}s ",t.elapsed().as_secs_f64());

    // 方案C
    print!("2Plies... ");let t=Instant::now();
    let mut dc=tc.clone();dc.depth_limit=Some(BoardState::Flop);dc.two_plies_lookahead=true;
    let mut gd=PostFlopGame::with_config(cc.clone(),ActionTree::new(dc).unwrap()).unwrap();
    gd.allocate_memory(false);solve(&mut gd,2000,target,false);
    println!("{:.1}s",t.elapsed().as_secs_f64());

    gf.back_to_root();gd.back_to_root();
    gf.cache_normalized_weights();gd.cache_normalized_weights();

    // 方案C α修正
    let texture = detect_board_texture(flop);
    let ctx = BoardCorrectionContext::new(texture);
    let bucket_labels = ["桶1:超强成牌","桶2:中等顶对","桶3:中等成牌","桶4:超强听牌","桶5:常规强听牌","桶6:弱听牌","桶7:纯空气"];

    let out_file = format!("doc/2plies_{}_SPR{}.csv",flop_str,spr);
    let mut csv=String::from("位置,手牌,大桶,FullEV,DLEv,DLEv_corrected,DLEv偏差%,修正后偏差%\n");
    for p in 0..2{
        let pos=if p==0{"OOP"}else{"IP"};
        let cards=gd.private_cards(p);
        let ev_f=gf.expected_values(p);
        let ev_d=gd.expected_values(p);
        let names=holes_to_strings(cards).unwrap();
        let mut rows:Vec<_>=(0..cards.len()).filter(|&i|ev_f[i]!=0.0).map(|i|{
            let(c1,c2)=cards[i];
            let(cat,draw)=classify_hand((c1,c2),flop,NOT_DEALT);
            let bkt=HandBucket::classify(cat,draw,(c1,c2),flop);
            let alpha=ctx.alpha(p,bkt);
            let corr=ev_d[i]*alpha as f32;
            let db=if ev_f[i].abs()>0.01{(ev_d[i]-ev_f[i])/ev_f[i]*100.0}else{0.0};
            let cb=if ev_f[i].abs()>0.01{(corr-ev_f[i])/ev_f[i]*100.0}else{0.0};
            (names[i].clone(),bucket_labels[bkt as usize],ev_f[i],ev_d[i],corr,db,cb)
        }).collect();
        rows.sort_by(|a,b|a.2.partial_cmp(&b.2).unwrap());
        for(n,bk,f,d,c,db,cb)in &rows{csv.push_str(&format!("{},{},{},{:.2},{:.2},{:.2},{:+.1}%,{:+.1}%\n",pos,n,bk,f,d,c,db,cb));}
    }
    std::fs::write(&out_file,&csv).unwrap();
    println!("→ {}",out_file);
}
