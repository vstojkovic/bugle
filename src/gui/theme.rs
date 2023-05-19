use std::collections::HashMap;

use fltk::app;
use fltk::enums::Color;
use lazy_static::lazy_static;

use crate::config::ThemeChoice;

pub struct Theme {
    colors: HashMap<u8, Color>,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            colors: HashMap::new(),
        }
    }

    pub fn from_config(theme: ThemeChoice) -> &'static Self {
        match theme {
            ThemeChoice::Light => &LIGHT_THEME,
            ThemeChoice::Dark => &DARK_THEME,
        }
    }

    pub fn set_color(&mut self, index: u8, color: Color) {
        self.colors.insert(index, color);
    }

    pub fn apply(&self) {
        for (&idx, color) in self.colors.iter() {
            let (r, g, b) = color.to_rgb();
            app::set_color(Color::by_index(idx), r, g, b);
        }
        app::redraw();
    }
}

lazy_static! {
    static ref LIGHT_THEME: Theme = {
        let mut theme = Theme::new();
        theme.set_color(0, Color::from_rgb(0x00, 0x00, 0x00));
        theme.set_color(1, Color::from_rgb(0xFF, 0x00, 0x00));
        theme.set_color(2, Color::from_rgb(0x00, 0xFF, 0x00));
        theme.set_color(3, Color::from_rgb(0xFF, 0xFF, 0x00));
        theme.set_color(4, Color::from_rgb(0x00, 0x00, 0xFF));
        theme.set_color(5, Color::from_rgb(0xFF, 0x00, 0xFF));
        theme.set_color(6, Color::from_rgb(0x00, 0xFF, 0xFF));
        theme.set_color(7, Color::from_rgb(0xFF, 0xFF, 0xFF));
        theme.set_color(8, Color::from_rgb(0x55, 0x55, 0x55));
        theme.set_color(9, Color::from_rgb(0xC6, 0x71, 0x71));
        theme.set_color(10, Color::from_rgb(0x71, 0xC6, 0x71));
        theme.set_color(11, Color::from_rgb(0x8E, 0x8E, 0x38));
        theme.set_color(12, Color::from_rgb(0x71, 0x71, 0xC6));
        theme.set_color(13, Color::from_rgb(0x8E, 0x38, 0x8E));
        theme.set_color(14, Color::from_rgb(0x38, 0x8E, 0x8E));
        theme.set_color(15, Color::from_rgb(0x00, 0x00, 0x80));
        theme.set_color(16, Color::from_rgb(0x98, 0x98, 0xA8));
        theme.set_color(32, Color::from_rgb(0x00, 0x00, 0x00));
        theme.set_color(33, Color::from_rgb(0x0D, 0x0D, 0x0D));
        theme.set_color(34, Color::from_rgb(0x1A, 0x1A, 0x1A));
        theme.set_color(35, Color::from_rgb(0x26, 0x26, 0x26));
        theme.set_color(36, Color::from_rgb(0x31, 0x31, 0x31));
        theme.set_color(37, Color::from_rgb(0x3D, 0x3D, 0x3D));
        theme.set_color(38, Color::from_rgb(0x48, 0x48, 0x48));
        theme.set_color(39, Color::from_rgb(0x55, 0x55, 0x55));
        theme.set_color(40, Color::from_rgb(0x5F, 0x5F, 0x5F));
        theme.set_color(41, Color::from_rgb(0x6A, 0x6A, 0x6A));
        theme.set_color(42, Color::from_rgb(0x75, 0x75, 0x75));
        theme.set_color(43, Color::from_rgb(0x80, 0x80, 0x80));
        theme.set_color(44, Color::from_rgb(0x8A, 0x8A, 0x8A));
        theme.set_color(45, Color::from_rgb(0x95, 0x95, 0x95));
        theme.set_color(46, Color::from_rgb(0xA0, 0xA0, 0xA0));
        theme.set_color(47, Color::from_rgb(0xAA, 0xAA, 0xAA));
        theme.set_color(48, Color::from_rgb(0xB5, 0xB5, 0xB5));
        theme.set_color(49, Color::from_rgb(0xC0, 0xC0, 0xC0));
        theme.set_color(50, Color::from_rgb(0xCB, 0xCB, 0xCB));
        theme.set_color(51, Color::from_rgb(0xD5, 0xD5, 0xD5));
        theme.set_color(52, Color::from_rgb(0xE0, 0xE0, 0xE0));
        theme.set_color(53, Color::from_rgb(0xEA, 0xEA, 0xEA));
        theme.set_color(54, Color::from_rgb(0xF5, 0xF5, 0xF5));
        theme.set_color(55, Color::from_rgb(0xFF, 0xFF, 0xFF));
        theme.set_color(56, Color::from_rgb(0x00, 0x00, 0x00));
        theme.set_color(57, Color::from_rgb(0x00, 0x24, 0x00));
        theme.set_color(58, Color::from_rgb(0x00, 0x49, 0x00));
        theme.set_color(59, Color::from_rgb(0x00, 0x6D, 0x00));
        theme.set_color(60, Color::from_rgb(0x00, 0x92, 0x00));
        theme.set_color(61, Color::from_rgb(0x00, 0xB6, 0x00));
        theme.set_color(62, Color::from_rgb(0x00, 0xDB, 0x00));
        theme.set_color(63, Color::from_rgb(0x00, 0xFF, 0x00));
        theme.set_color(64, Color::from_rgb(0x40, 0x00, 0x00));
        theme.set_color(65, Color::from_rgb(0x40, 0x24, 0x00));
        theme.set_color(66, Color::from_rgb(0x40, 0x49, 0x00));
        theme.set_color(67, Color::from_rgb(0x40, 0x6D, 0x00));
        theme.set_color(68, Color::from_rgb(0x40, 0x92, 0x00));
        theme.set_color(69, Color::from_rgb(0x40, 0xB6, 0x00));
        theme.set_color(70, Color::from_rgb(0x40, 0xDB, 0x00));
        theme.set_color(71, Color::from_rgb(0x40, 0xFF, 0x00));
        theme.set_color(72, Color::from_rgb(0x80, 0x00, 0x00));
        theme.set_color(73, Color::from_rgb(0x80, 0x24, 0x00));
        theme.set_color(74, Color::from_rgb(0x80, 0x49, 0x00));
        theme.set_color(75, Color::from_rgb(0x80, 0x6D, 0x00));
        theme.set_color(76, Color::from_rgb(0x80, 0x92, 0x00));
        theme.set_color(77, Color::from_rgb(0x80, 0xB6, 0x00));
        theme.set_color(78, Color::from_rgb(0x80, 0xDB, 0x00));
        theme.set_color(79, Color::from_rgb(0x80, 0xFF, 0x00));
        theme.set_color(80, Color::from_rgb(0xBF, 0x00, 0x00));
        theme.set_color(81, Color::from_rgb(0xBF, 0x24, 0x00));
        theme.set_color(82, Color::from_rgb(0xBF, 0x49, 0x00));
        theme.set_color(83, Color::from_rgb(0xBF, 0x6D, 0x00));
        theme.set_color(84, Color::from_rgb(0xBF, 0x92, 0x00));
        theme.set_color(85, Color::from_rgb(0xBF, 0xB6, 0x00));
        theme.set_color(86, Color::from_rgb(0xBF, 0xDB, 0x00));
        theme.set_color(87, Color::from_rgb(0xBF, 0xFF, 0x00));
        theme.set_color(88, Color::from_rgb(0xFF, 0x00, 0x00));
        theme.set_color(89, Color::from_rgb(0xFF, 0x24, 0x00));
        theme.set_color(90, Color::from_rgb(0xFF, 0x49, 0x00));
        theme.set_color(91, Color::from_rgb(0xFF, 0x6D, 0x00));
        theme.set_color(92, Color::from_rgb(0xFF, 0x92, 0x00));
        theme.set_color(93, Color::from_rgb(0xFF, 0xB6, 0x00));
        theme.set_color(94, Color::from_rgb(0xFF, 0xDB, 0x00));
        theme.set_color(95, Color::from_rgb(0xFF, 0xFF, 0x00));
        theme.set_color(96, Color::from_rgb(0x00, 0x00, 0x40));
        theme.set_color(97, Color::from_rgb(0x00, 0x24, 0x40));
        theme.set_color(98, Color::from_rgb(0x00, 0x49, 0x40));
        theme.set_color(99, Color::from_rgb(0x00, 0x6D, 0x40));
        theme.set_color(100, Color::from_rgb(0x00, 0x92, 0x40));
        theme.set_color(101, Color::from_rgb(0x00, 0xB6, 0x40));
        theme.set_color(102, Color::from_rgb(0x00, 0xDB, 0x40));
        theme.set_color(103, Color::from_rgb(0x00, 0xFF, 0x40));
        theme.set_color(104, Color::from_rgb(0x40, 0x00, 0x40));
        theme.set_color(105, Color::from_rgb(0x40, 0x24, 0x40));
        theme.set_color(106, Color::from_rgb(0x40, 0x49, 0x40));
        theme.set_color(107, Color::from_rgb(0x40, 0x6D, 0x40));
        theme.set_color(108, Color::from_rgb(0x40, 0x92, 0x40));
        theme.set_color(109, Color::from_rgb(0x40, 0xB6, 0x40));
        theme.set_color(110, Color::from_rgb(0x40, 0xDB, 0x40));
        theme.set_color(111, Color::from_rgb(0x40, 0xFF, 0x40));
        theme.set_color(112, Color::from_rgb(0x80, 0x00, 0x40));
        theme.set_color(113, Color::from_rgb(0x80, 0x24, 0x40));
        theme.set_color(114, Color::from_rgb(0x80, 0x49, 0x40));
        theme.set_color(115, Color::from_rgb(0x80, 0x6D, 0x40));
        theme.set_color(116, Color::from_rgb(0x80, 0x92, 0x40));
        theme.set_color(117, Color::from_rgb(0x80, 0xB6, 0x40));
        theme.set_color(118, Color::from_rgb(0x80, 0xDB, 0x40));
        theme.set_color(119, Color::from_rgb(0x80, 0xFF, 0x40));
        theme.set_color(120, Color::from_rgb(0xBF, 0x00, 0x40));
        theme.set_color(121, Color::from_rgb(0xBF, 0x24, 0x40));
        theme.set_color(122, Color::from_rgb(0xBF, 0x49, 0x40));
        theme.set_color(123, Color::from_rgb(0xBF, 0x6D, 0x40));
        theme.set_color(124, Color::from_rgb(0xBF, 0x92, 0x40));
        theme.set_color(125, Color::from_rgb(0xBF, 0xB6, 0x40));
        theme.set_color(126, Color::from_rgb(0xBF, 0xDB, 0x40));
        theme.set_color(127, Color::from_rgb(0xBF, 0xFF, 0x40));
        theme.set_color(128, Color::from_rgb(0xFF, 0x00, 0x40));
        theme.set_color(129, Color::from_rgb(0xFF, 0x24, 0x40));
        theme.set_color(130, Color::from_rgb(0xFF, 0x49, 0x40));
        theme.set_color(131, Color::from_rgb(0xFF, 0x6D, 0x40));
        theme.set_color(132, Color::from_rgb(0xFF, 0x92, 0x40));
        theme.set_color(133, Color::from_rgb(0xFF, 0xB6, 0x40));
        theme.set_color(134, Color::from_rgb(0xFF, 0xDB, 0x40));
        theme.set_color(135, Color::from_rgb(0xFF, 0xFF, 0x40));
        theme.set_color(136, Color::from_rgb(0x00, 0x00, 0x80));
        theme.set_color(137, Color::from_rgb(0x00, 0x24, 0x80));
        theme.set_color(138, Color::from_rgb(0x00, 0x49, 0x80));
        theme.set_color(139, Color::from_rgb(0x00, 0x6D, 0x80));
        theme.set_color(140, Color::from_rgb(0x00, 0x92, 0x80));
        theme.set_color(141, Color::from_rgb(0x00, 0xB6, 0x80));
        theme.set_color(142, Color::from_rgb(0x00, 0xDB, 0x80));
        theme.set_color(143, Color::from_rgb(0x00, 0xFF, 0x80));
        theme.set_color(144, Color::from_rgb(0x40, 0x00, 0x80));
        theme.set_color(145, Color::from_rgb(0x40, 0x24, 0x80));
        theme.set_color(146, Color::from_rgb(0x40, 0x49, 0x80));
        theme.set_color(147, Color::from_rgb(0x40, 0x6D, 0x80));
        theme.set_color(148, Color::from_rgb(0x40, 0x92, 0x80));
        theme.set_color(149, Color::from_rgb(0x40, 0xB6, 0x80));
        theme.set_color(150, Color::from_rgb(0x40, 0xDB, 0x80));
        theme.set_color(151, Color::from_rgb(0x40, 0xFF, 0x80));
        theme.set_color(152, Color::from_rgb(0x80, 0x00, 0x80));
        theme.set_color(153, Color::from_rgb(0x80, 0x24, 0x80));
        theme.set_color(154, Color::from_rgb(0x80, 0x49, 0x80));
        theme.set_color(155, Color::from_rgb(0x80, 0x6D, 0x80));
        theme.set_color(156, Color::from_rgb(0x80, 0x92, 0x80));
        theme.set_color(157, Color::from_rgb(0x80, 0xB6, 0x80));
        theme.set_color(158, Color::from_rgb(0x80, 0xDB, 0x80));
        theme.set_color(159, Color::from_rgb(0x80, 0xFF, 0x80));
        theme.set_color(160, Color::from_rgb(0xBF, 0x00, 0x80));
        theme.set_color(161, Color::from_rgb(0xBF, 0x24, 0x80));
        theme.set_color(162, Color::from_rgb(0xBF, 0x49, 0x80));
        theme.set_color(163, Color::from_rgb(0xBF, 0x6D, 0x80));
        theme.set_color(164, Color::from_rgb(0xBF, 0x92, 0x80));
        theme.set_color(165, Color::from_rgb(0xBF, 0xB6, 0x80));
        theme.set_color(166, Color::from_rgb(0xBF, 0xDB, 0x80));
        theme.set_color(167, Color::from_rgb(0xBF, 0xFF, 0x80));
        theme.set_color(168, Color::from_rgb(0xFF, 0x00, 0x80));
        theme.set_color(169, Color::from_rgb(0xFF, 0x24, 0x80));
        theme.set_color(170, Color::from_rgb(0xFF, 0x49, 0x80));
        theme.set_color(171, Color::from_rgb(0xFF, 0x6D, 0x80));
        theme.set_color(172, Color::from_rgb(0xFF, 0x92, 0x80));
        theme.set_color(173, Color::from_rgb(0xFF, 0xB6, 0x80));
        theme.set_color(174, Color::from_rgb(0xFF, 0xDB, 0x80));
        theme.set_color(175, Color::from_rgb(0xFF, 0xFF, 0x80));
        theme.set_color(176, Color::from_rgb(0x00, 0x00, 0xBF));
        theme.set_color(177, Color::from_rgb(0x00, 0x24, 0xBF));
        theme.set_color(178, Color::from_rgb(0x00, 0x49, 0xBF));
        theme.set_color(179, Color::from_rgb(0x00, 0x6D, 0xBF));
        theme.set_color(180, Color::from_rgb(0x00, 0x92, 0xBF));
        theme.set_color(181, Color::from_rgb(0x00, 0xB6, 0xBF));
        theme.set_color(182, Color::from_rgb(0x00, 0xDB, 0xBF));
        theme.set_color(183, Color::from_rgb(0x00, 0xFF, 0xBF));
        theme.set_color(184, Color::from_rgb(0x40, 0x00, 0xBF));
        theme.set_color(185, Color::from_rgb(0x40, 0x24, 0xBF));
        theme.set_color(186, Color::from_rgb(0x40, 0x49, 0xBF));
        theme.set_color(187, Color::from_rgb(0x40, 0x6D, 0xBF));
        theme.set_color(188, Color::from_rgb(0x40, 0x92, 0xBF));
        theme.set_color(189, Color::from_rgb(0x40, 0xB6, 0xBF));
        theme.set_color(190, Color::from_rgb(0x40, 0xDB, 0xBF));
        theme.set_color(191, Color::from_rgb(0x40, 0xFF, 0xBF));
        theme.set_color(192, Color::from_rgb(0x80, 0x00, 0xBF));
        theme.set_color(193, Color::from_rgb(0x80, 0x24, 0xBF));
        theme.set_color(194, Color::from_rgb(0x80, 0x49, 0xBF));
        theme.set_color(195, Color::from_rgb(0x80, 0x6D, 0xBF));
        theme.set_color(196, Color::from_rgb(0x80, 0x92, 0xBF));
        theme.set_color(197, Color::from_rgb(0x80, 0xB6, 0xBF));
        theme.set_color(198, Color::from_rgb(0x80, 0xDB, 0xBF));
        theme.set_color(199, Color::from_rgb(0x80, 0xFF, 0xBF));
        theme.set_color(200, Color::from_rgb(0xBF, 0x00, 0xBF));
        theme.set_color(201, Color::from_rgb(0xBF, 0x24, 0xBF));
        theme.set_color(202, Color::from_rgb(0xBF, 0x49, 0xBF));
        theme.set_color(203, Color::from_rgb(0xBF, 0x6D, 0xBF));
        theme.set_color(204, Color::from_rgb(0xBF, 0x92, 0xBF));
        theme.set_color(205, Color::from_rgb(0xBF, 0xB6, 0xBF));
        theme.set_color(206, Color::from_rgb(0xBF, 0xDB, 0xBF));
        theme.set_color(207, Color::from_rgb(0xBF, 0xFF, 0xBF));
        theme.set_color(208, Color::from_rgb(0xFF, 0x00, 0xBF));
        theme.set_color(209, Color::from_rgb(0xFF, 0x24, 0xBF));
        theme.set_color(210, Color::from_rgb(0xFF, 0x49, 0xBF));
        theme.set_color(211, Color::from_rgb(0xFF, 0x6D, 0xBF));
        theme.set_color(212, Color::from_rgb(0xFF, 0x92, 0xBF));
        theme.set_color(213, Color::from_rgb(0xFF, 0xB6, 0xBF));
        theme.set_color(214, Color::from_rgb(0xFF, 0xDB, 0xBF));
        theme.set_color(215, Color::from_rgb(0xFF, 0xFF, 0xBF));
        theme.set_color(216, Color::from_rgb(0x00, 0x00, 0xFF));
        theme.set_color(217, Color::from_rgb(0x00, 0x24, 0xFF));
        theme.set_color(218, Color::from_rgb(0x00, 0x49, 0xFF));
        theme.set_color(219, Color::from_rgb(0x00, 0x6D, 0xFF));
        theme.set_color(220, Color::from_rgb(0x00, 0x92, 0xFF));
        theme.set_color(221, Color::from_rgb(0x00, 0xB6, 0xFF));
        theme.set_color(222, Color::from_rgb(0x00, 0xDB, 0xFF));
        theme.set_color(223, Color::from_rgb(0x00, 0xFF, 0xFF));
        theme.set_color(224, Color::from_rgb(0x40, 0x00, 0xFF));
        theme.set_color(225, Color::from_rgb(0x40, 0x24, 0xFF));
        theme.set_color(226, Color::from_rgb(0x40, 0x49, 0xFF));
        theme.set_color(227, Color::from_rgb(0x40, 0x6D, 0xFF));
        theme.set_color(228, Color::from_rgb(0x40, 0x92, 0xFF));
        theme.set_color(229, Color::from_rgb(0x40, 0xB6, 0xFF));
        theme.set_color(230, Color::from_rgb(0x40, 0xDB, 0xFF));
        theme.set_color(231, Color::from_rgb(0x40, 0xFF, 0xFF));
        theme.set_color(232, Color::from_rgb(0x80, 0x00, 0xFF));
        theme.set_color(233, Color::from_rgb(0x80, 0x24, 0xFF));
        theme.set_color(234, Color::from_rgb(0x80, 0x49, 0xFF));
        theme.set_color(235, Color::from_rgb(0x80, 0x6D, 0xFF));
        theme.set_color(236, Color::from_rgb(0x80, 0x92, 0xFF));
        theme.set_color(237, Color::from_rgb(0x80, 0xB6, 0xFF));
        theme.set_color(238, Color::from_rgb(0x80, 0xDB, 0xFF));
        theme.set_color(239, Color::from_rgb(0x80, 0xFF, 0xFF));
        theme.set_color(240, Color::from_rgb(0xBF, 0x00, 0xFF));
        theme.set_color(241, Color::from_rgb(0xBF, 0x24, 0xFF));
        theme.set_color(242, Color::from_rgb(0xBF, 0x49, 0xFF));
        theme.set_color(243, Color::from_rgb(0xBF, 0x6D, 0xFF));
        theme.set_color(244, Color::from_rgb(0xBF, 0x92, 0xFF));
        theme.set_color(245, Color::from_rgb(0xBF, 0xB6, 0xFF));
        theme.set_color(246, Color::from_rgb(0xBF, 0xDB, 0xFF));
        theme.set_color(247, Color::from_rgb(0xBF, 0xFF, 0xFF));
        theme.set_color(248, Color::from_rgb(0xFF, 0x00, 0xFF));
        theme.set_color(249, Color::from_rgb(0xFF, 0x24, 0xFF));
        theme.set_color(250, Color::from_rgb(0xFF, 0x49, 0xFF));
        theme.set_color(251, Color::from_rgb(0xFF, 0x6D, 0xFF));
        theme.set_color(252, Color::from_rgb(0xFF, 0x92, 0xFF));
        theme.set_color(253, Color::from_rgb(0xFF, 0xB6, 0xFF));
        theme.set_color(254, Color::from_rgb(0xFF, 0xDB, 0xFF));
        theme.set_color(255, Color::from_rgb(0xFF, 0xFF, 0xFF));
        theme.set_color(254, Color::from_rgb(0xDC, 0xF0, 0xF0));

        theme
    };
    pub static ref DARK_THEME: Theme = {
        let mut theme = Theme::new();

        theme.set_color(0, Color::from_rgb(0xFF, 0xFF, 0xFF));
        theme.set_color(1, Color::from_rgb(0x96, 0x1E, 0x1E));
        theme.set_color(2, Color::from_rgb(0x00, 0xB4, 0x00));
        theme.set_color(3, Color::from_rgb(0xB4, 0xB4, 0x00));
        theme.set_color(4, Color::from_rgb(0x00, 0x00, 0xB4));
        theme.set_color(5, Color::from_rgb(0xB4, 0x00, 0xB4));
        theme.set_color(6, Color::from_rgb(0x00, 0xB4, 0xB4));
        theme.set_color(7, Color::from_rgb(0x3A, 0x3A, 0x3A));
        theme.set_color(8, Color::from_rgb(0x26, 0x26, 0x26));
        theme.set_color(9, Color::from_rgb(0x96, 0x5A, 0x5A));
        theme.set_color(10, Color::from_rgb(0x5A, 0x96, 0x5A));
        theme.set_color(11, Color::from_rgb(0x96, 0x96, 0x5A));
        theme.set_color(12, Color::from_rgb(0x5A, 0x5A, 0x96));
        theme.set_color(13, Color::from_rgb(0x96, 0x5A, 0x96));
        theme.set_color(14, Color::from_rgb(0x5A, 0x96, 0x96));
        theme.set_color(15, Color::from_rgb(0xD6, 0xD6, 0xD6));
        theme.set_color(16, Color::from_rgb(0x18, 0x18, 0x18));
        theme.set_color(32, Color::from_rgb(0xE0, 0xE0, 0xE0));
        theme.set_color(33, Color::from_rgb(0x0A, 0x0A, 0x0A));
        theme.set_color(34, Color::from_rgb(0x10, 0x10, 0x10));
        theme.set_color(35, Color::from_rgb(0x15, 0x15, 0x15));
        theme.set_color(36, Color::from_rgb(0x1A, 0x1A, 0x1A));
        theme.set_color(37, Color::from_rgb(0x20, 0x20, 0x20));
        theme.set_color(38, Color::from_rgb(0x25, 0x25, 0x25));
        theme.set_color(39, Color::from_rgb(0x2A, 0x2A, 0x2A));
        theme.set_color(40, Color::from_rgb(0x30, 0x30, 0x30));
        theme.set_color(41, Color::from_rgb(0x35, 0x35, 0x35));
        theme.set_color(42, Color::from_rgb(0x3A, 0x3A, 0x3A));
        theme.set_color(43, Color::from_rgb(0x40, 0x40, 0x40));
        theme.set_color(44, Color::from_rgb(0x45, 0x45, 0x45));
        theme.set_color(45, Color::from_rgb(0x4A, 0x4A, 0x4A));
        theme.set_color(46, Color::from_rgb(0x50, 0x50, 0x50));
        theme.set_color(47, Color::from_rgb(0x55, 0x55, 0x55));
        theme.set_color(48, Color::from_rgb(0x5A, 0x5A, 0x5A));
        theme.set_color(49, Color::from_rgb(0x53, 0x53, 0x53));
        theme.set_color(50, Color::from_rgb(0x65, 0x65, 0x65));
        theme.set_color(51, Color::from_rgb(0x6A, 0x6A, 0x6A));
        theme.set_color(52, Color::from_rgb(0x70, 0x70, 0x70));
        theme.set_color(53, Color::from_rgb(0x75, 0x75, 0x75));
        theme.set_color(54, Color::from_rgb(0x7A, 0x7A, 0x7A));
        theme.set_color(55, Color::from_rgb(0xB4, 0xB4, 0xB4));
        theme.set_color(56, Color::from_rgb(0x00, 0x00, 0x00));
        theme.set_color(59, Color::from_rgb(0x1E, 0x8C, 0x1E));
        theme.set_color(63, Color::from_rgb(0x00, 0xB4, 0x00));
        theme.set_color(71, Color::from_rgb(0x00, 0xB4, 0x00));
        theme.set_color(88, Color::from_rgb(0xB4, 0x00, 0x00));
        theme.set_color(90, Color::from_rgb(0xC8, 0x48, 0x3C));
        theme.set_color(91, Color::from_rgb(0xB4, 0x78, 0x00));
        theme.set_color(94, Color::from_rgb(0x96, 0x64, 0x1E));
        theme.set_color(95, Color::from_rgb(0xB4, 0xB4, 0x00));
        theme.set_color(124, Color::from_rgb(0xBF, 0x91, 0x3F));
        theme.set_color(254, Color::from_rgb(0x3C, 0x42, 0x42));
        theme.set_color(255, Color::from_rgb(0x32, 0x32, 0x32));

        theme
    };
}
