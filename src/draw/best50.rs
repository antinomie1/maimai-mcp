//! Best 50 image (upstream `PlayerBest50.draw`).

use super::base::score_grid;
use super::canvas::{mai_dir, open, paste, paste_file, pic, resize, theme_pic};
use super::design_text;
use super::text::{BLACK, F, draw, draw_stroked};
use crate::core::models::{Best50, Plate, Player, Service, Theme};
use crate::core::music::MaiData;
use image::RgbaImage;

/// Remote images resolved before drawing (LXNS collections / QQ avatar).
#[derive(Default)]
pub struct Extras {
    pub plate: Option<RgbaImage>,
    pub icon: Option<RgbaImage>,
}

fn ra_pic(rating: u32, theme: Theme) -> &'static str {
    const STEPS: [(u32, &str); 10] = [
        (1000, "01"),
        (2000, "02"),
        (4000, "03"),
        (7000, "04"),
        (10000, "05"),
        (12000, "06"),
        (13000, "07"),
        (14000, "08"),
        (14500, "09"),
        (15000, "10"),
    ];
    if let Some((_, n)) = STEPS.iter().find(|(limit, _)| rating < *limit) {
        return n;
    }
    if theme == Theme::Circle && (16000..17000).contains(&rating) { "12" } else { "11" }
}

fn ra_star(rating: u32) -> u32 {
    const T: [u32; 12] = [14000, 14250, 14500, 14750, 15000, 15250, 15500, 15750, 16000, 16250, 16500, 16750];
    const N: [u32; 12] = [1, 2, 1, 2, 1, 2, 3, 4, 1, 2, 3, 4];
    let idx = T.iter().filter(|t| **t <= rating).count().saturating_sub(1);
    N[idx]
}

pub fn draw_best50(
    data: &MaiData,
    player: &Player,
    b50: &Best50,
    theme: Theme,
    service: Service,
    extras: Extras,
) -> RgbaImage {
    let mut im = open(&theme_pic(theme, "b50.png")).map_or_else(|| RgbaImage::new(1400, 1600), |i| (*i).clone());
    let color = theme.color();

    paste_file(&mut im, &theme_pic(theme, "logo.png"), 249, 120, 14, 60);

    let plate = extras.plate.or_else(|| {
        let path = match &player.plate {
            Plate::Name(name) => mai_dir("plate_version").join(format!("{name}.png")),
            Plate::Id(id) => mai_dir("plate").join(format!("UI_Plate_{id:06}.png")),
            Plate::None => pic("UI_Plate_550101.png"),
        };
        open(&path).or_else(|| open(&pic("UI_Plate_550101.png"))).map(|i| (*i).clone())
    });
    if let Some(p) = plate {
        paste(&mut im, &resize(&p, 800, 130), 300, 60);
    }

    let icon = extras.icon.or_else(|| {
        let local = player.icon.map(|id| mai_dir("icon").join(format!("UI_Icon_{id:06}.png")));
        local.and_then(|p| open(&p)).or_else(|| open(&pic("UI_Icon_509506.png"))).map(|i| (*i).clone())
    });
    if let Some(i) = icon {
        paste(&mut im, &resize(&i, 120, 120), 305, 65);
    }

    let (mut ra_size, mut num_x, mut num_y, mut gap, mut num_size) = ((186, 35), 520, 80, 15, (17, 20));
    let rating = player.rating;
    let star = theme == Theme::Circle && rating >= 14000;
    if star {
        (ra_size, num_x, num_y, gap, num_size) = ((170, 35), 515, 82, 13, (14, 17));
    }
    let ra_file = format!("UI_CMN_DXRating_{}.png", ra_pic(rating, theme));
    paste_file(&mut im, &theme_pic(theme, &ra_file), ra_size.0, ra_size.1, 435, 72);
    if star {
        let file = format!("UI_CMN_DXRating_Star_0{}.png", ra_star(rating));
        paste_file(&mut im, &theme_pic(theme, &file), 21, 35, 590, 72);
    }
    for (n, d) in format!("{rating:05}").chars().enumerate() {
        let file = pic(&format!("UI_NUM_Drating_{d}.png"));
        paste_file(&mut im, &file, num_size.0, num_size.1, num_x + gap * n as i32, num_y);
    }

    let cr = player.course_rank;
    let dani = if cr <= 10 { cr } else { cr + 1 };
    paste_file(&mut im, &pic("Name.png"), 0, 0, 435, 115);
    paste_file(&mut im, &pic(&format!("UI_DNM_DaniPlate_{dani:02}.png")), 80, 32, 625, 120);
    paste_file(&mut im, &pic(&format!("UI_FBR_Class_{:02}.png", player.class_rank)), 90, 54, 620, 60);

    let shougou = mai_dir("shougou");
    let (title, bg, font) = match &player.trophy {
        Some(t) => (t.name.clone(), format!("UI_CMN_Shougou_{}.png", t.color), F::Sy),
        None => (
            format!("B35: {} + B15: {} = {rating}", b50.sd_total, b50.dx_total),
            "UI_CMN_Shougou_Rainbow.png".to_string(),
            F::Tb,
        ),
    };
    paste_file(&mut im, &shougou.join(bg), 270, 27, 435, 160);
    draw(&mut im, font, 570, 172, 14.0, &title, BLACK, "mm");
    draw(&mut im, F::Sy, 445, 135, 20.0, &player.name, BLACK, "lm");
    draw_stroked(&mut im, F::Sy, 700, 1570, 22.0, &design_text(Some(service)), color, "mm", 5);

    score_grid(&mut im, data, &b50.sd, theme, 235);
    score_grid(&mut im, data, &b50.dx, theme, 1085);
    im
}
