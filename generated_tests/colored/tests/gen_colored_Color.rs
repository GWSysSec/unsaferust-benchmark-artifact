use colored::*;
use colored::control::{set_override, unset_override};
use std::borrow::Cow;

#[test]
fn test_to_fg_str_basic_colors() {
    set_override(true);

    let black_fg = Color::Black.to_fg_str();
    let red_fg = Color::Red.to_fg_str();
    let green_fg = Color::Green.to_fg_str();
    let yellow_fg = Color::Yellow.to_fg_str();
    let blue_fg = Color::Blue.to_fg_str();
    let magenta_fg = Color::Magenta.to_fg_str();
    let cyan_fg = Color::Cyan.to_fg_str();
    let white_fg = Color::White.to_fg_str();

    assert_eq!(black_fg, "30");
    assert_eq!(red_fg, "31");
    assert_eq!(green_fg, "32");
    assert_eq!(yellow_fg, "33");
    assert_eq!(blue_fg, "34");
    assert_eq!(magenta_fg, "35");
    assert_eq!(cyan_fg, "36");
    assert_eq!(white_fg, "37");

    unset_override();
}

#[test]
fn test_to_bg_str_basic_colors() {
    set_override(true);

    let black_bg = Color::Black.to_bg_str();
    let red_bg = Color::Red.to_bg_str();
    let green_bg = Color::Green.to_bg_str();
    let yellow_bg = Color::Yellow.to_bg_str();
    let blue_bg = Color::Blue.to_bg_str();
    let magenta_bg = Color::Magenta.to_bg_str();
    let cyan_bg = Color::Cyan.to_bg_str();
    let white_bg = Color::White.to_bg_str();

    assert_eq!(black_bg, "40");
    assert_eq!(red_bg, "41");
    assert_eq!(green_bg, "42");
    assert_eq!(yellow_bg, "43");
    assert_eq!(blue_bg, "44");
    assert_eq!(magenta_bg, "45");
    assert_eq!(cyan_bg, "46");
    assert_eq!(white_bg, "47");

    unset_override();
}

#[test]
fn test_to_fg_str_bright_colors() {
    set_override(true);

    let bright_black_fg = Color::BrightBlack.to_fg_str();
    let bright_red_fg = Color::BrightRed.to_fg_str();
    let bright_green_fg = Color::BrightGreen.to_fg_str();
    let bright_yellow_fg = Color::BrightYellow.to_fg_str();
    let bright_blue_fg = Color::BrightBlue.to_fg_str();
    let bright_magenta_fg = Color::BrightMagenta.to_fg_str();
    let bright_cyan_fg = Color::BrightCyan.to_fg_str();
    let bright_white_fg = Color::BrightWhite.to_fg_str();

    assert_eq!(bright_black_fg, "90");
    assert_eq!(bright_red_fg, "91");
    assert_eq!(bright_green_fg, "92");
    assert_eq!(bright_yellow_fg, "93");
    assert_eq!(bright_blue_fg, "94");
    assert_eq!(bright_magenta_fg, "95");
    assert_eq!(bright_cyan_fg, "96");
    assert_eq!(bright_white_fg, "97");

    unset_override();
}

#[test]
fn test_to_bg_str_bright_colors() {
    set_override(true);

    let bright_black_bg = Color::BrightBlack.to_bg_str();
    let bright_red_bg = Color::BrightRed.to_bg_str();
    let bright_green_bg = Color::BrightGreen.to_bg_str();
    let bright_yellow_bg = Color::BrightYellow.to_bg_str();
    let bright_blue_bg = Color::BrightBlue.to_bg_str();
    let bright_magenta_bg = Color::BrightMagenta.to_bg_str();
    let bright_cyan_bg = Color::BrightCyan.to_bg_str();
    let bright_white_bg = Color::BrightWhite.to_bg_str();

    assert_eq!(bright_black_bg, "100");
    assert_eq!(bright_red_bg, "101");
    assert_eq!(bright_green_bg, "102");
    assert_eq!(bright_yellow_bg, "103");
    assert_eq!(bright_blue_bg, "104");
    assert_eq!(bright_magenta_bg, "105");
    assert_eq!(bright_cyan_bg, "106");
    assert_eq!(bright_white_bg, "107");

    unset_override();
}

#[test]
fn test_to_fg_str_truecolor() {
    set_override(true);

    let custom = Color::TrueColor { r: 255, g: 128, b: 0 };
    let fg_str = custom.to_fg_str();
    assert_eq!(fg_str, "38;2;255;128;0");

    let custom2 = Color::TrueColor { r: 0, g: 0, b: 0 };
    let fg_str2 = custom2.to_fg_str();
    assert_eq!(fg_str2, "38;2;0;0;0");

    let custom3 = Color::TrueColor { r: 255, g: 255, b: 255 };
    let fg_str3 = custom3.to_fg_str();
    assert_eq!(fg_str3, "38;2;255;255;255");

    let custom4 = Color::TrueColor { r: 100, g: 200, b: 50 };
    let fg_str4 = custom4.to_fg_str();
    assert_eq!(fg_str4, "38;2;100;200;50");


    assert_ne!(fg_str, "");
    assert_ne!(fg_str2, fg_str);
    assert_ne!(fg_str3, fg_str2);
    assert_ne!(fg_str4, fg_str3);

    unset_override();
}

#[test]
fn test_to_bg_str_truecolor() {
    set_override(true);

    let custom = Color::TrueColor { r: 255, g: 128, b: 0 };
    let bg_str = custom.to_bg_str();
    assert_eq!(bg_str, "48;2;255;128;0");

    let custom2 = Color::TrueColor { r: 0, g: 0, b: 0 };
    let bg_str2 = custom2.to_bg_str();
    assert_eq!(bg_str2, "48;2;0;0;0");

    let custom3 = Color::TrueColor { r: 255, g: 255, b: 255 };
    let bg_str3 = custom3.to_bg_str();
    assert_eq!(bg_str3, "48;2;255;255;255");

    let custom4 = Color::TrueColor { r: 1, g: 2, b: 3 };
    let bg_str4 = custom4.to_bg_str();
    assert_eq!(bg_str4, "48;2;1;2;3");


    let fg_str = custom.to_fg_str();
    assert_ne!(bg_str, fg_str);
    assert!(bg_str.starts_with("48;2;"));
    assert!(fg_str.starts_with("38;2;"));

    unset_override();
}

#[test]
fn test_fg_bg_str_consistency_with_colored_output() {
    set_override(true);


    let red_fg_code = Color::Red.to_fg_str();
    let red_bg_code = Color::Red.to_bg_str();

    assert_eq!(red_fg_code, "31");
    assert_eq!(red_bg_code, "41");

    let colored_str = "hello".red().to_string();
    assert!(colored_str.contains(&*red_fg_code));

    let bg_colored_str = "hello".on_red().to_string();
    assert!(bg_colored_str.contains(&*red_bg_code));


    assert_ne!(Color::Blue.to_fg_str(), Color::Blue.to_bg_str());
    assert_ne!(Color::Green.to_fg_str(), Color::Green.to_bg_str());
    assert_ne!(Color::Cyan.to_fg_str(), Color::Cyan.to_bg_str());
    assert_ne!(Color::White.to_fg_str(), Color::White.to_bg_str());

    unset_override();
}

#[test]
fn test_fg_bg_str_cow_type_behavior() {
    set_override(true);


    let red_fg: Cow<'static, str> = Color::Red.to_fg_str();
    let red_bg: Cow<'static, str> = Color::Red.to_bg_str();


    assert_eq!(&*red_fg, "31");
    assert_eq!(&*red_bg, "41");


    let tc_fg: Cow<'static, str> = Color::TrueColor { r: 10, g: 20, b: 30 }.to_fg_str();
    let tc_bg: Cow<'static, str> = Color::TrueColor { r: 10, g: 20, b: 30 }.to_bg_str();

    assert_eq!(&*tc_fg, "38;2;10;20;30");
    assert_eq!(&*tc_bg, "48;2;10;20;30");


    let cloned_fg = tc_fg.clone();
    let cloned_bg = tc_bg.clone();
    assert_eq!(cloned_fg, tc_fg);
    assert_eq!(cloned_bg, tc_bg);

    unset_override();
}

#[test]
fn test_fg_bg_str_all_colors_differ_from_each_other() {
    set_override(true);

    let colors = vec![
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ];


    let fg_codes: Vec<Cow<'static, str>> = colors.iter().map(|c| c.to_fg_str()).collect();
    for i in 0..fg_codes.len() {
        for j in (i + 1)..fg_codes.len() {
            assert_ne!(fg_codes[i], fg_codes[j]);
        }
    }


    let bg_codes: Vec<Cow<'static, str>> = colors.iter().map(|c| c.to_bg_str()).collect();
    for i in 0..bg_codes.len() {
        for j in (i + 1)..bg_codes.len() {
            assert_ne!(bg_codes[i], bg_codes[j]);
        }
    }


    for color in &colors {
        assert_ne!(color.to_fg_str(), color.to_bg_str());
    }

    unset_override();
}

#[test]
fn test_truecolor_boundary_values_fg_bg() {
    set_override(true);


    let min_color = Color::TrueColor { r: 0, g: 0, b: 0 };
    assert_eq!(min_color.to_fg_str(), "38;2;0;0;0");
    assert_eq!(min_color.to_bg_str(), "48;2;0;0;0");


    let max_color = Color::TrueColor { r: 255, g: 255, b: 255 };
    assert_eq!(max_color.to_fg_str(), "38;2;255;255;255");
    assert_eq!(max_color.to_bg_str(), "48;2;255;255;255");


    let r_max = Color::TrueColor { r: 255, g: 0, b: 0 };
    assert_eq!(r_max.to_fg_str(), "38;2;255;0;0");
    assert_eq!(r_max.to_bg_str(), "48;2;255;0;0");

    let g_max = Color::TrueColor { r: 0, g: 255, b: 0 };
    assert_eq!(g_max.to_fg_str(), "38;2;0;255;0");
    assert_eq!(g_max.to_bg_str(), "48;2;0;255;0");

    unset_override();
}