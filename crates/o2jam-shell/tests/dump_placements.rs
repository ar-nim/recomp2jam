//! Temporary diagnostic: myroom staged multi-frame figures.
use o2jam_shell::screens::side::SideScreen;
use o2jam_shell::staging::{stage_state, state_of_side, StageContext};

#[test]
fn dump_myroom_figures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
    let bytes = std::fs::read(root.join("O2Jam/Image/Interface1.opi")).expect("archive");
    let art = stage_state(
        &bytes,
        state_of_side(SideScreen::MyRoom).unwrap(),
        StageContext::default(),
    )
    .art
    .expect("art");
    for (name, fig) in &art.figures {
        if fig.len() > 1 {
            println!("FIG {name} len={} parts={:?}", fig.len(), fig.part_lens());
        }
    }
    for p in &art.placements {
        if p.code >= 0x1550_002F && p.code <= 0x1550_0032 {
            println!("MR {:08X} {:?} {:?}", p.code, p.sprite, p.rect);
        }
    }
}
