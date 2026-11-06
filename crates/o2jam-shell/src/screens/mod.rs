//! Window screens: scene painters plus the [`ScreenGame`] loop.

pub mod game;
pub mod paint;
pub mod side;

pub use game::ScreenGame;
pub use paint::paint_screen;
pub use side::{
    bind_side_scene, can_return_side, can_visit_side, load_side_figures, paint_side_scene,
    side_sprite_names, SideScreen, ALL_SIDE_SCREENS, LANKING_BIND_FILE, LANKING_BLOCK,
    MUSIC_SHOP_BIND_FILE, MUSIC_SHOP_BLOCK, MYROOM_BIND_FILE, MYROOM_BLOCK, PAYMENT2_BIND_FILE,
    PAYMENT2_BLOCK, PAYMENT_BIND_FILE, PAYMENT_BLOCK, PLANET_BIND_FILE, PLANET_BLOCK,
    SHOP_BIND_FILE, SHOP_BLOCK, TUTORIAL_BACKGROUND_SPRITE,
};
