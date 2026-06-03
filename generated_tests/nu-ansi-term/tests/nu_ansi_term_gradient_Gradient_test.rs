use nu_ansi_term::gradient::{Gradient, TargetGround};
use nu_ansi_term::{Color, Rgb};

#[test]
fn rgb_gradient_from_color_rgb_interpolates_expected_points() {
    let gradient = Gradient::from_color_rgb(Color::Rgb(0, 0, 0), Color::Rgb(100, 150, 200));

    assert_eq!(gradient.at(0.0), Rgb::new(0, 0, 0));
    assert_eq!(gradient.at(1.0), Rgb::new(100, 150, 200));
    assert_eq!(gradient.at(0.5), Rgb::new(50, 75, 100));

    let quarter = gradient.at(0.25);
    assert_eq!(quarter, Rgb::new(25, 37, 50));

    let reversed = gradient.reverse();
    assert_eq!(reversed.at(0.0), Rgb::new(100, 150, 200));
    assert_eq!(reversed.at(1.0), Rgb::new(0, 0, 0));
}

#[test]
fn build_renders_foreground_gradient_for_each_character() {
    let gradient = Gradient::from_color_rgb(Color::Rgb(255, 0, 0), Color::Rgb(0, 0, 255));

    let rendered = gradient.build("abc", TargetGround::Foreground);

    assert!(rendered.contains("\x1b[38;2;255;0;0ma"));
    assert!(rendered.contains("\x1b[38;2;169;0;85mb"));
    assert!(rendered.contains("\x1b[38;2;84;0;170mc"));
    assert!(!rendered.contains("\x1b[48;2;"));
    assert!(rendered.ends_with("\x1b[0m"));

    let index_a = rendered.find('a').expect("rendered gradient should contain a");
    let index_b = rendered.find('b').expect("rendered gradient should contain b");
    let index_c = rendered.find('c').expect("rendered gradient should contain c");

    assert!(index_a < index_b);
    assert!(index_b < index_c);
}

#[test]
fn build_renders_background_gradient_and_handles_edge_cases() {
    let gradient = Gradient::from_color_rgb(Color::Rgb(10, 20, 30), Color::Rgb(30, 40, 50));

    let background = gradient.build("xy", TargetGround::Background);

    assert!(background.contains("\x1b[48;2;10;20;30mx"));
    assert!(background.contains("\x1b[48;2;20;30;40my"));
    assert!(!background.contains("\x1b[38;2;"));
    assert!(background.ends_with("\x1b[0m"));

    let index_x = background.find('x').expect("rendered gradient should contain x");
    let index_y = background.find('y').expect("rendered gradient should contain y");
    assert!(index_x < index_y);

    let single = gradient.build("z", TargetGround::Foreground);
    assert!(single.contains("\x1b[38;2;10;20;30mz"));
    assert!(single.ends_with("\x1b[0m"));

    let empty = gradient.build("", TargetGround::Foreground);
    assert_eq!(empty, "\x1b[0m");
}